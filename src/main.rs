use calamine::{open_workbook, open_workbook_auto, DataType, Range, Reader, Xls};
use std::{
    env,
    fs::File,
    io::{self, BufWriter, Error, Result, Write},
    path::PathBuf,
};

const PATH: &str = "/tmp/export2022819.xls";
const SHEET_NAME: &str = "Movimientos";

fn skip_to_header_row(
    range: Range<DataType>,
    expected_headers: Vec<&str>,
) -> Result<Range<DataType>> {
    if let Some((ix, _)) = range.rows().enumerate().find(|(i, row)| {
        println!("i: {}", i);
        println!("row: {:?}", row);
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

fn main() {
    // let mut workbook: Xls<_> = open_workbook(PATH).expect("Cannot open file");
    // if let Some(Ok(transactions)) = workbook.worksheet_range(SHEET_NAME) {
    //     let transactions_skipped = skip_rows(transactions, 8).expect("should work");
    //     println!(
    //         "Transactions with skipped rows: {:?}",
    //         transactions_skipped.rows()
    //     );
    //     // let rows = movimientos.rows();

    //     // let num_rows = rows.len();
    //     // for (i, row) in rows.enumerate() {
    //     //     println!("{}:row={:?}", i, row);
    //     // }
    // }
    // // converts first argument into a csv (same name, silently overrides
    // // if the file already exists

    let file = env::args()
        .nth(1)
        .expect("Please provide an excel file to convert");
    let sheet = env::args()
        .nth(2)
        .expect("Expecting a sheet name as second argument");

    let sce = PathBuf::from(file);
    match sce.extension().and_then(|s| s.to_str()) {
        Some("xlsx") | Some("xlsm") | Some("xlsb") | Some("xls") => (),
        _ => panic!("Expecting an excel file"),
    }

    let dest = sce.with_extension("csv");
    let mut dest = BufWriter::new(File::create(dest).unwrap());
    let mut xl = open_workbook_auto(&sce).unwrap();
    let range = xl.worksheet_range(&sheet).unwrap().unwrap();

    write_range(&mut dest, &range).unwrap();

    let mut workbook = open_workbook_auto(&sce.with_extension("csv")).unwrap();
    let range = workbook.worksheet_range(&sheet).unwrap().unwrap();
    let transactions_skipped = skip_rows(range, 8).expect("should work");

    println!(
        "Transactions with skipped rows: {:?}",
        transactions_skipped.rows()
    );
}

fn write_range<W: Write>(dest: &mut W, range: &Range<DataType>) -> std::io::Result<()> {
    let n = range.get_size().1 - 1;
    for r in range.rows() {
        for (i, c) in r.iter().enumerate() {
            match *c {
                DataType::Empty => Ok(()),
                DataType::String(ref s) => write!(dest, "{}", s),
                DataType::Float(ref f) | DataType::DateTime(ref f) => write!(dest, "{}", f),
                DataType::Int(ref i) => write!(dest, "{}", i),
                DataType::Error(ref e) => write!(dest, "{:?}", e),
                DataType::Bool(ref b) => write!(dest, "{}", b),
            }?;
            if i != n {
                write!(dest, ";")?;
            }
        }
        write!(dest, "\r\n")?;
    }
    Ok(())
}
