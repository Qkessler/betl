use std::io::{self, Error, ErrorKind};

use super::banks::BankConfig;
use crate::Transaction;

use calamine::{open_workbook, DataType, Range, RangeDeserializerBuilder, Reader, Xls};
use chrono::NaiveDateTime;
use serde::Deserialize;

pub trait BankStatement {
    fn parse_transactions<T>(
        input_file: &str,
        config: &BankConfig,
        should_reverse: bool,
    ) -> Vec<Transaction>
    where
        T: std::fmt::Debug + serde::de::DeserializeOwned;
}

pub trait TransactionConverter<T> {
    fn convert(transaction: T) -> Transaction;
}

#[derive(Debug, Deserialize)]
pub struct RevolutTransaction {
    #[serde(rename = "Type")]
    transaction_type: String,
    #[serde(rename = "Product")]
    product: String,
    #[serde(rename = "Started Date")]
    operation_date: String,
    #[serde(rename = "Completed Date")]
    value_date: String,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Amount")]
    amount: f32,
    #[serde(rename = "Fee")]
    fee: f32,
    #[serde(rename = "Currency")]
    currency: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Balance")]
    balance: String,
}

pub struct RevolutBankStatement;
impl BankStatement for RevolutBankStatement {
    fn parse_transactions<T>(
        input_file: &str,
        _: &BankConfig,
        should_reverse: bool,
    ) -> Vec<Transaction>
    where
        T: std::fmt::Debug + serde::de::DeserializeOwned,
    {
        let mut rdr = csv::Reader::from_path(&input_file).expect("File exists and is readable");
        let transactions: Vec<Transaction> = rdr
            .deserialize()
            .map(|result| result.expect("deserialization to work"))
            .map(|transaction: RevolutTransaction| Self::convert(transaction))
            .collect();

        return if should_reverse {
            transactions.into_iter().rev().collect()
        } else {
            transactions
        };
    }
}
impl TransactionConverter<RevolutTransaction> for RevolutBankStatement {
    fn convert(transaction: RevolutTransaction) -> Transaction {
        let operation_date =
            NaiveDateTime::parse_from_str(&transaction.operation_date, "%Y-%m-%d %H:%M:%S")
                .expect("date to be passed in the correct format");
        let value_date =
            NaiveDateTime::parse_from_str(&transaction.value_date, "%Y-%m-%d %H:%M:%S")
                .expect("date to be passed in the correct format");
        Transaction {
            operation_date: Some(operation_date.date()),
            value_date: Some(value_date.date()),
            description: transaction.description,
            amount: transaction.amount,
        }
    }
}

pub struct XlsBankStatement;
impl XlsBankStatement {
    fn skip_rows(range: Range<DataType>, n: u32) -> io::Result<Range<DataType>> {
        let start = range.start().unwrap();
        let end = range.end().unwrap();
        Ok(range.range((start.0 + n, start.1), end))
    }

    fn modify_headers(input_file: &str, config: &BankConfig) -> io::Result<Range<DataType>> {
        let mut workbook: Xls<_> = open_workbook(input_file).expect("Cannot open file");
        if let Some(Ok(worksheet)) = workbook.worksheet_range(config.sheet_name) {
            let mut range = Self::skip_rows(worksheet, config.skip_row_num).expect("should work");
            config.headers.iter().enumerate().for_each(|(i, header)| {
                range.set_value(
                    (config.skip_row_num, i as u32),
                    DataType::String((*header).to_owned()),
                )
            });
            Ok(range)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Couldn't open worksheet for SHEET_NAME = {:?}",
                    config.sheet_name
                ),
            ))
        }
    }
}

impl BankStatement for XlsBankStatement {
    fn parse_transactions<T>(
        input_file: &str,
        config: &BankConfig,
        should_reverse: bool,
    ) -> Vec<Transaction> {
        let workbook = Self::modify_headers(&input_file, &config)
            .unwrap_or_else(|e| panic!("Header modification failed. Error: {}", e));
        let transactions: Vec<Transaction> = RangeDeserializerBuilder::with_headers(config.headers)
            .from_range::<_, Transaction>(&workbook)
            .expect("Deserializer should work.")
            .map(|transaction| transaction.unwrap())
            .collect();

        return if should_reverse {
            transactions.into_iter().rev().collect()
        } else {
            transactions
        };
    }
}
