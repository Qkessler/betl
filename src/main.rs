use calamine::{open_workbook, DataType, Range, RangeDeserializerBuilder, Reader, Xls};
use chrono::{NaiveDate, Utc};
use clap::Parser;
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

const SHEET_NAME: &str = "Movimientos";
const ACCOUNT: &str = "Assets:Checking";
const CONFIG_FILE: &str = ".config/santander_ledger.json";
const DATE_FORMAT: &str = "%d/%m/%Y";
const HEADERS: &[&str] = &[
    "operation_date",
    "value_date",
    "description",
    "amount",
    "total",
];

/// Santander transactions parser. Example usage: santander-ledger -f /tmp/transactions.xls
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Transactions file path.
    #[clap(short, long)]
    file: String,
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    lazy_static! {
        static ref DEFAULT_DATE: NaiveDate = Utc::now().date_naive();
    }
    let data_type = calamine::DataType::deserialize(deserializer);
    Ok(match data_type?.get_string() {
        Some(date) => Some(NaiveDate::parse_from_str(date, DATE_FORMAT).unwrap_or(*DEFAULT_DATE)),
        None => Some(*DEFAULT_DATE),
    })
}

#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(deserialize_with = "deserialize_date")]
    operation_date: Option<NaiveDate>,
    #[serde(deserialize_with = "deserialize_date")]
    value_date: Option<NaiveDate>,
    description: String,
    amount: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    mappings: HashMap<String, String>,
}

fn skip_rows(range: Range<DataType>, n: u32) -> io::Result<Range<DataType>> {
    let start = range.start().unwrap();
    let end = range.end().unwrap();
    Ok(range.range((start.0 + n, start.1), end))
}

fn parse_transactions(range: &Range<DataType>) -> Vec<Transaction> {
    RangeDeserializerBuilder::with_headers(HEADERS)
        .from_range::<_, Transaction>(range)
        .expect("Deserializer should work.")
        .map(|transaction| transaction.unwrap())
        .collect()
}

fn build_transaction_string(transaction: &Transaction, config: Option<&Config>) -> String {
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

fn write_transactions(transactions: &[Transaction], path: &str, config: Option<&Config>) {
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

fn modify_headers(input_file: &str) -> io::Result<Range<DataType>> {
    let mut workbook: Xls<_> = open_workbook(input_file).expect("Cannot open file");
    if let Some(Ok(worksheet)) = workbook.worksheet_range(SHEET_NAME) {
        let mut range = skip_rows(worksheet, 7).expect("should work");
        HEADERS.iter().enumerate().for_each(|(i, header)| {
            range.set_value((7, i as u32), DataType::String((*header).to_owned()))
        });
        Ok(range)
    } else {
        Err(Error::new(
            io::ErrorKind::Other,
            format!("Couldn't open worksheet for SHEET_NAME = {:?}", SHEET_NAME),
        ))
    }
}

fn main() {
    let args = Args::parse();
    let input_file = args.file;
    let home_dir = dirs::home_dir().unwrap().join(CONFIG_FILE);
    let config: Option<Config> = if let Ok(file) = File::open(home_dir) {
        let reader = BufReader::new(file);
        match serde_json::from_reader::<BufReader<File>, Config>(reader) {
            Ok(config) => Some(config),
            Err(_) => None,
        }
    } else {
        None
    };

    let workbook = modify_headers(&input_file).unwrap();
    let transactions = parse_transactions(&workbook);
    write_transactions(&transactions, &input_file, config.as_ref());
}
