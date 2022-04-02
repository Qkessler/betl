use calamine::{Reader, open_workbook, Xls};

const PATH: &str = "/Users/enrikes/Documents/santander-ledger/test-file.xls";
const SHEET_NAME: &str = "Movimientos" ;

fn main() {
    let mut workbook: Xls<_> = open_workbook(PATH).expect("Cannot open file");
    if let Some(Ok(movimientos)) = workbook.worksheet_range(SHEET_NAME) {
        let rows = movimientos.rows(); 
        // let num_rows = rows.len();
        for (i, row) in rows.enumerate() {
            println!("{}:row={:?}", i, row);
        }

    }
}

