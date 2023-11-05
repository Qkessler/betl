use clap::clap_derive::ArgEnum;

#[derive(Debug, PartialEq, Clone, ArgEnum)]
pub enum Bank {
    Bankia,
    Santander,
    Revolut,
}

pub struct BankConfig<'a> {
    pub skip_row_num: u32,
    pub headers: &'static [&'static str],
    pub sheet_name: &'a str,
    pub base_account: &'a str,
}
