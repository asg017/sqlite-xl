mod cells;
mod parser;
mod rows;
mod sheets;

use calamine::{DataType, Reader};
use sqlite_loadable::{api, define_scalar_function, Result};
use sqlite_loadable::{define_table_function, prelude::*};

pub fn xl_at(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    unsafe {
        let row: *mut Vec<DataType> = api::value_pointer(&values[0], b"ROW\0").unwrap();
        let idx = api::value_int64(&values[1]);
        crate::result_xl_data(context, (*row).get(idx as usize).unwrap())?;
    }
    Ok(())
}

fn result_xl_data(context: *mut sqlite3_context, data: &DataType) -> Result<()> {
    match data {
        DataType::Int(value) => api::result_int64(context, *value),
        DataType::Float(value) => api::result_double(context, *value),
        DataType::String(value) => api::result_text(context, value)?,
        DataType::Bool(value) => api::result_bool(context, *value),
        DataType::DateTime(value) => api::result_double(context, *value),
        DataType::Duration(value) => api::result_double(context, *value),
        DataType::DateTimeIso(value) => api::result_text(context, value)?,
        DataType::DurationIso(value) => api::result_text(context, value)?,
        DataType::Error(value) => {
            api::result_text(context, format!("{value}"))?;
        }
        DataType::Empty => api::result_null(context),
    }
    Ok(())
}
#[sqlite_entrypoint]
pub fn sqlite3_xl_init(db: *mut sqlite3) -> Result<()> {
    define_table_function::<sheets::SheetsTable>(db, "xl_sheets", None)?;
    define_table_function::<cells::CellsTable>(db, "xl_cells", None)?;
    define_table_function::<rows::RowsTable>(db, "xl_rows", None)?;
    define_scalar_function(db, "xl_at", 2, xl_at, FunctionFlags::UTF8)?;
    Ok(())
}
