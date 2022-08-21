use calamine::{open_workbook, DataType, Range, RangeDeserializerBuilder, Reader, Xls};
use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{self, BufReader, Error, Write},
    path::Path,
};

const SHEET_NAME: &str = "Movimientos";
const ACCOUNT: &str = "Assets:Checking";
const CONFIG_FILE: &str = ".config/santander_ledger.json";
const HEADERS: &[&str] = &[
    "fecha_operacion",
    "fecha_valor",
    "concepto",
    "importe",
    "saldo",
];

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let data_type = calamine::DataType::deserialize(deserializer);
    Ok(data_type?.as_date())
}

#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(deserialize_with = "deserialize_date")]
    fecha_operacion: Option<NaiveDate>,
    #[serde(deserialize_with = "deserialize_date")]
    fecha_valor: Option<NaiveDate>,
    concepto: String,
    importe: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    mappings: HashMap<String, String>,
}

fn skip_to_header_row(
    range: Range<DataType>,
    expected_headers: Vec<&str>,
) -> io::Result<Range<DataType>> {
    if let Some((ix, _)) = range.rows().enumerate().find(|(_, row)| {
        expected_headers
            .iter()
            .all(|h| row.iter().any(|cell: &DataType| &cell.to_string() == h))
    }) {
        skip_rows(range, ix as u32)
    } else {
        println!(
            "Couldn't find header row with expected headers: {:?}",
            expected_headers
        );
        Err(Error::new(
            io::ErrorKind::Other,
            format!(
                "Couldn't find header row with expected headers: {:?}",
                expected_headers
            ),
        ))
    }
}

fn skip_rows(range: Range<DataType>, n: u32) -> io::Result<Range<DataType>> {
    let start = range.start().unwrap();
    let end = range.end().unwrap();
    Ok(range.range((start.0 + n, start.1), end))
}

fn compute_transactions(path: &str, config: Option<&Config>) {
    let mut workbook: Xls<_> = open_workbook(path).expect("Cannot open file");
    if let Some(Ok(worksheet)) = workbook.worksheet_range(SHEET_NAME) {
        let transactions_skipped =
            skip_to_header_row(worksheet, HEADERS.to_vec()).expect("should work");
        let transactions: Vec<Transaction> = RangeDeserializerBuilder::with_headers(HEADERS)
            .from_range::<_, Transaction>(&transactions_skipped)
            .expect("Deserializer should work.")
            .map(|transaction| transaction.unwrap())
            .collect();

        write_transactions(&transactions, path, config);
    }
}

fn build_transaction_string(transaction: &Transaction, config: Option<&Config>) -> String {
    let mut transaction_string = format!(
        "{} * {}\n    {}               {:.2}€\n",
        transaction
            .fecha_operacion
            .expect("Date should be present")
            .format("%Y-%m-%d"),
        transaction.concepto,
        ACCOUNT,
        transaction.importe
    );
    transaction_string = match config {
        Some(config) => {
            if let Some(matched_concepto) = config.mappings.keys().find(|regex| {
                Regex::new(regex)
                    .unwrap()
                    .is_match(transaction.concepto.as_str())
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

fn main() {
    let input_file = env::args().nth(1).expect("Please provide an input file.");
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

    compute_transactions(input_file.as_str(), config.as_ref());
}
