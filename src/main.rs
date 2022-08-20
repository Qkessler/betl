use calamine::{open_workbook, DataType, Range, RangeDeserializerBuilder, Reader, Xls};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Error, Result, Write},
    path::Path,
};

const PATH: &str = "/tmp/export2022819.xls";
const SHEET_NAME: &str = "Movimientos";
const DATE_FORMAT: &str = "%d/%m/%Y";
const ACCOUNT: &str = "Assets:Checking";
const CONFIG_FILE: &str = "/Users/enrikes/.config/santander_ledger.json";
const HEADERS: &[&str] = &[
    "fecha_operacion",
    "fecha_valor",
    "concepto",
    "importe",
    "saldo",
];

mod date_serde {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    use crate::DATE_FORMAT;

    pub fn serialize<S>(date: &Option<NaiveDate>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(ref d) = *date {
            return s.serialize_str(&d.format(DATE_FORMAT).to_string());
        }
        s.serialize_none()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data_type = calamine::DataType::deserialize(deserializer);
        Ok(data_type?.as_date())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(with = "date_serde")]
    fecha_operacion: Option<NaiveDate>,
    #[serde(with = "date_serde")]
    fecha_valor: Option<NaiveDate>,
    concepto: String,
    importe: isize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    mappings: HashMap<String, String>,
}

fn skip_to_header_row(
    range: Range<DataType>,
    expected_headers: Vec<&str>,
) -> Result<Range<DataType>> {
    if let Some((ix, _)) = range.rows().enumerate().find(|(i, row)| {
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

fn skip_rows(range: Range<DataType>, n: u32) -> Result<Range<DataType>> {
    let start = range.start().unwrap();
    let end = range.end().unwrap();
    Ok(range.range((start.0 + n, start.1), end))
}

fn compute_transactions(path: &str, config: &Config) {
    println!("{:?}", config);
    let mut workbook: Xls<_> = open_workbook(path).expect("Cannot open file");
    if let Some(Ok(worksheet)) = workbook.worksheet_range(SHEET_NAME) {
        let transactions_skipped =
            skip_to_header_row(worksheet, HEADERS.to_vec()).expect("should work");
        let transactions: Vec<Transaction> = RangeDeserializerBuilder::with_headers(HEADERS)
            .from_range::<_, Transaction>(&transactions_skipped)
            .expect("Deserializer should work.")
            .map(|transaction| transaction.unwrap())
            .collect();

        println!("{:?}", transactions);

        write_transactions(&transactions, path, config);
    }
}

fn build_transaction_string(transaction: &Transaction) -> String {
    format!(
        "{} * {}\n    {}               {}€\n\n",
        transaction
            .fecha_operacion
            .expect("Date should be present")
            .format("%Y-%m-%d"),
        transaction.concepto,
        ACCOUNT,
        transaction.importe
    )
}

fn write_transactions(transactions: &[Transaction], path: &str, config: &Config) {
    let path = Path::new(path).with_extension("ledger");
    if let Ok(mut file) = File::create(path) {
        transactions.iter().for_each(|transaction| {
            let transaction_string = build_transaction_string(transaction);
            print!("{}", transaction_string);
            file.write_all(transaction_string.as_bytes())
                .unwrap_or_else(|_| panic!("Unable to write transaction {:?}", transaction));
        });
    }
}

fn main() {
    let file = File::open(CONFIG_FILE).unwrap();
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader).unwrap();
    compute_transactions(PATH, &config);
}
