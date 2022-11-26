use calamine::{open_workbook, DataType, Range, RangeDeserializerBuilder, Reader, Xls};
use chrono::{NaiveDate, Utc};
use clap::{clap_derive::ArgEnum, Parser};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Error, Write},
    path::Path,
};
#[macro_use]
extern crate lazy_static;

const SANTANDER_SHEET_NAME: &str = "Movimientos";
const ACCOUNT: &str = "Assets:Checking";
const CONFIG_FILE: &str = ".config/santander_ledger.json";
const DATE_FORMAT: &str = "%d/%m/%Y";
const SANTANDER_HEADERS: &[&str] = &[
    "operation_date",
    "value_date",
    "description",
    "amount",
    "total",
];
const BANKIA_HEADERS: &[&str] = &[
    "operation_date",
    "value_date",
    "description",
    "more",
    "amount",
    "total",
];

#[derive(Debug, PartialEq, Clone, ArgEnum)]
enum Bank {
    Bankia,
    Santander,
}

/// Santander transactions parser. Example usage: santander-ledger -f /tmp/transactions.xls
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Transactions file path.
    #[clap(short, long)]
    file: String,

    #[clap(short, long, arg_enum)]
    bank: Bank,
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    lazy_static! {
        static ref DEFAULT_DATE: NaiveDate = Utc::now().date_naive();
    }
    let data_type = calamine::DataType::deserialize(deserializer)?;
    Ok(match data_type.as_date() {
        Some(date) => Some(date),
        None => match data_type.get_string() {
            Some(date) => {
                Some(NaiveDate::parse_from_str(date, DATE_FORMAT).unwrap_or(*DEFAULT_DATE))
            }
            None => Some(*DEFAULT_DATE),
        },
    })
}

// cargo run -- -b bankia -f /tmp/Movimientos_cuenta_0262497.xls
// cargo run -- -b santander -f /tmp/export20221126.xls

#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(deserialize_with = "deserialize_date")]
    operation_date: Option<NaiveDate>,
    #[serde(deserialize_with = "deserialize_date")]
    value_date: Option<NaiveDate>,
    description: String,
    amount: f32,
}

struct BankConfig<'a> {
    skip_row_num: u32,
    headers: &'static [&'static str],
    sheet_name: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mappings {
    mappings: HashMap<String, String>,
}

fn skip_rows(range: Range<DataType>, n: u32) -> io::Result<Range<DataType>> {
    let start = range.start().unwrap();
    let end = range.end().unwrap();
    Ok(range.range((start.0 + n, start.1), end))
}

fn parse_transactions(range: &Range<DataType>, config: &BankConfig) -> Vec<Transaction> {
    RangeDeserializerBuilder::with_headers(config.headers)
        .from_range::<_, Transaction>(range)
        .expect("Deserializer should work.")
        .map(|transaction| transaction.unwrap())
        .collect()
}

fn build_transaction_string(transaction: &Transaction, config: Option<&Mappings>) -> String {
    let mut transaction_string = format!(
        "{} * {}\n    {}               {:.2}€\n",
        transaction
            .operation_date
            .expect("Date should be present")
            .format("%Y-%m-%d"),
        transaction.description,
        ACCOUNT,
        transaction.amount
    );
    transaction_string = match config {
        Some(config) => {
            if let Some(matched_concepto) = config.mappings.keys().find(|regex| {
                Regex::new(regex)
                    .unwrap()
                    .is_match(transaction.description.as_str())
            }) {
                format!(
                    "{}    {}\n",
                    transaction_string,
                    config.mappings.get(matched_concepto).unwrap()
                )
            } else {
                transaction_string
            }
        }
        None => transaction_string,
    };
    transaction_string.push('\n');
    transaction_string
}

fn write_transactions(transactions: &[Transaction], path: &str, config: Option<&Mappings>) {
    let path = Path::new(path).with_extension("ledger");
    if let Ok(mut file) = File::create(path) {
        transactions.iter().for_each(|transaction| {
            let transaction_string = build_transaction_string(transaction, config);
            print!("{}", transaction_string);
            file.write_all(transaction_string.as_bytes())
                .unwrap_or_else(|_| panic!("Unable to write transaction {:?}", transaction));
        });
    }
}

fn modify_headers(input_file: &str, config: &BankConfig) -> io::Result<Range<DataType>> {
    let mut workbook: Xls<_> = open_workbook(input_file).expect("Cannot open file");
    if let Some(Ok(worksheet)) = workbook.worksheet_range(config.sheet_name) {
        let mut range = skip_rows(worksheet, config.skip_row_num).expect("should work");
        config.headers.iter().enumerate().for_each(|(i, header)| {
            range.set_value(
                (config.skip_row_num, i as u32),
                DataType::String((*header).to_owned()),
            )
        });
        Ok(range)
    } else {
        Err(Error::new(
            io::ErrorKind::Other,
            format!(
                "Couldn't open worksheet for SHEET_NAME = {:?}",
                config.sheet_name
            ),
        ))
    }
}

fn parse_config<'a>(bank: &Bank, sheet_name: Option<&'a str>) -> BankConfig<'a> {
    match bank {
        Bank::Bankia => BankConfig {
            skip_row_num: 2,
            headers: BANKIA_HEADERS,
            sheet_name: sheet_name
                .unwrap_or_else(|| panic!("Should have sheet_name passed as parameter.")),
        },
        Bank::Santander => BankConfig {
            skip_row_num: 7,
            headers: SANTANDER_HEADERS,
            sheet_name: sheet_name.unwrap_or_else(|| SANTANDER_SHEET_NAME),
        },
    }
}

fn main() {
    let args = Args::parse();
    let input_file = args.file;
    let home_dir = dirs::home_dir().unwrap().join(CONFIG_FILE);
    let mappings: Option<Mappings> = if let Ok(file) = File::open(home_dir) {
        let reader = BufReader::new(file);
        match serde_json::from_reader::<BufReader<File>, Mappings>(reader) {
            Ok(config) => Some(config),
            Err(_) => None,
        }
    } else {
        None
    };

    let file_without_extension = Path::new(&input_file)
        .file_stem()
        .unwrap_or_else(|| panic!("Should be able to get real file name."));
    let sheet_name: Option<&str> = match &args.bank {
        Bank::Bankia => Some(file_without_extension.to_str().unwrap_or_default()),
        Bank::Santander => None,
    };

    let config = parse_config(&args.bank, sheet_name);
    let workbook = modify_headers(&input_file, &config)
        .unwrap_or_else(|e| panic!("Header modification failed. Error: {}", e));
    let transactions = parse_transactions(&workbook, &config);
    write_transactions(&transactions, &input_file, mappings.as_ref());
}
