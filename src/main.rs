mod bank_statement;
mod banks;

use bank_statement::{BankStatement, ExcelBankStatement, RevolutBankStatement, RevolutTransaction};
use banks::{Bank, BankConfig};
use calamine::{Xls, Xlsx};
use chrono::{NaiveDate, Utc};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Write},
    path::Path,
};

const SANTANDER_SHEET_NAME: &str = "Movimientos";
const SANTANDER_BASE_ACCOUNT: &str = "Assets:Checking";
const BANKIA_BASE_ACCOUNT: &str = "Assets:Emergency fund";
const REVOLUT_BASE_ACCOUNT: &str = "Assets:Revolut";
const EVO_BANK_BASE_ACCOUNT: &str = "Assets:EvoBank";
const BANKINTER_BASE_ACCOUNT: &str = "Assets:Bankinter";
const CONFIG_FILE: &str = ".config/betl.json";
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
const REVOLUT_HEADERS: &[&str] = &[
    "type",
    "product",
    "operation_date",
    "value_date",
    "description",
    "amount",
    "fee",
    "currency",
    "state",
    "total",
];
const EVO_BANK_HEADERS: &[&str] = &[
    "operation_date",
    "value_date",
    "description",
    "amount",
    "currency",
    "total",
];
const BANKINTER_HEADERS: &[&str] = &[
    "operation_date",
    "value_date",
    "description",
    "amount",
    "total",
    "currency",
];

static DEFAULT_DATE: Lazy<NaiveDate> = Lazy::new(|| Utc::now().date_naive());

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Transactions file path.
    #[clap(short, long)]
    file: String,

    #[clap(short, long, arg_enum)]
    bank: Bank,

    #[clap(short, long, action)]
    reverse: bool,
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    #[serde(deserialize_with = "deserialize_date")]
    operation_date: Option<NaiveDate>,
    #[serde(deserialize_with = "deserialize_date")]
    value_date: Option<NaiveDate>,
    description: String,
    amount: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mappings {
    mappings: HashMap<String, String>,
}

fn build_transaction_string<'a>(
    transaction: &Transaction,
    config: Option<&Mappings>,
    base_account: &'a str,
) -> String {
    let mut transaction_string = format!(
        "{} * {}\n    {}               {:.2}€\n",
        transaction
            .operation_date
            .expect("Date should be present")
            .format("%Y-%m-%d"),
        transaction.description,
        base_account,
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

fn write_transactions(
    transactions: &[Transaction],
    path: &str,
    mappings_config: Option<&Mappings>,
    config: &BankConfig,
) {
    let path = Path::new(path).with_extension("ledger");
    let base_account = config.base_account;
    if let Ok(mut file) = File::create(path) {
        transactions.iter().for_each(|transaction| {
            let transaction_string =
                build_transaction_string(transaction, mappings_config, base_account);
            print!("{}", transaction_string);
            file.write_all(transaction_string.as_bytes())
                .unwrap_or_else(|_| panic!("Unable to write transaction {:?}", transaction));
        });
    }
}

fn parse_config<'a>(bank: &Bank, sheet_name: Option<&'a str>) -> BankConfig<'a> {
    match bank {
        Bank::Bankia => BankConfig {
            skip_row_num: 2,
            headers: BANKIA_HEADERS,
            sheet_name: sheet_name
                .unwrap_or_else(|| panic!("Should have sheet_name passed as parameter.")),
            base_account: BANKIA_BASE_ACCOUNT,
        },
        Bank::Santander => BankConfig {
            skip_row_num: 7,
            headers: SANTANDER_HEADERS,
            sheet_name: sheet_name.unwrap_or(SANTANDER_SHEET_NAME),
            base_account: SANTANDER_BASE_ACCOUNT,
        },
        Bank::Revolut => BankConfig {
            skip_row_num: 0,
            headers: REVOLUT_HEADERS,
            sheet_name: sheet_name
                .unwrap_or_else(|| panic!("Should have sheet_name passed as parameter.")),
            base_account: REVOLUT_BASE_ACCOUNT,
        },
        Bank::EvoBank => BankConfig {
            skip_row_num: 1,
            headers: EVO_BANK_HEADERS,
            sheet_name: sheet_name
                .unwrap_or_else(|| panic!("Should have sheet_name passed as parameter.")),
            base_account: EVO_BANK_BASE_ACCOUNT,
        },
        Bank::Bankinter => BankConfig {
            skip_row_num: 8,
            headers: BANKINTER_HEADERS,
            sheet_name: sheet_name
                .unwrap_or_else(|| panic!("Should have sheet_name passed as parameter.")),
            base_account: BANKINTER_BASE_ACCOUNT,
        },
    }
}

fn main() {
    let args = Args::parse();
    let input_file = args.file;
    let should_reverse = args.reverse;
    let home_dir = dirs::home_dir().unwrap().join(CONFIG_FILE);
    let mappings: Option<Mappings> = if let Ok(file) = File::open(home_dir) {
        let reader = BufReader::new(file);
        serde_json::from_reader::<BufReader<File>, Mappings>(reader).ok()
    } else {
        None
    };

    let file_without_extension = Path::new(&input_file)
        .file_stem()
        .unwrap_or_else(|| panic!("Should be able to get real file name."));
    let file_name = file_without_extension.to_str().unwrap_or_default();
    let sheet_name: Option<&str> = match &args.bank {
        Bank::Bankia => Some(file_name),
        Bank::Santander => None,
        Bank::Revolut => Some(file_name),
        Bank::EvoBank | Bank::Bankinter => Some("Movimientos"),
    };

    let config = parse_config(&args.bank, sheet_name);
    let transactions = match &args.bank {
        Bank::Bankia | Bank::Santander | Bank::EvoBank => {
            ExcelBankStatement::<Xls<_>>::parse_transactions::<Transaction>(
                &input_file,
                &config,
                should_reverse,
            )
        }
        Bank::Bankinter => ExcelBankStatement::<Xlsx<_>>::parse_transactions::<Transaction>(
            &input_file,
            &config,
            should_reverse,
        ),
        Bank::Revolut => RevolutBankStatement::parse_transactions::<RevolutTransaction>(
            &input_file,
            &config,
            should_reverse,
        ),
    };
    write_transactions(&transactions, &input_file, mappings.as_ref(), &config);
}
