mod cells;
mod parser;
mod rows;
mod sheets;

use calamine::{DataType, Reader};
use parser::column_name_to_idx;
use sqlite_loadable::table::define_table_function_with_find;
use sqlite_loadable::{api, define_scalar_function, Result};
use sqlite_loadable::{define_table_function, prelude::*};

pub fn xl_at(context: *mut sqlite3_context, values: &[*mut sqlite3_value]) -> Result<()> {
    unsafe {
        let row: *mut Vec<DataType> = api::value_pointer(&values[0], b"ROW\0").unwrap();
        let idx = match api::value_type(&values[1]) {
          api::ValueType::Integer => api::value_int64(&values[1]),
          api::ValueType::Text => {
            column_name_to_idx(api::value_text(&values[1])?).unwrap().into()
          }
          _ => todo!(),
        };
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

pub fn xl_version(context: *mut sqlite3_context, _values: &[*mut sqlite3_value]) -> Result<()> {
    api::result_text(context, format!("v{}", env!("CARGO_PKG_VERSION")))?;
    Ok(())
}
#[sqlite_entrypoint]
pub fn sqlite3_xl_init(db: *mut sqlite3) -> Result<()> {
    define_table_function::<sheets::SheetsTable>(db, "xl_sheets", None)?;
    define_table_function::<cells::CellsTable>(db, "xl_cells", None)?;
    define_table_function_with_find::<rows::RowsTable>(db, "xl_rows", None)?;
    define_scalar_function(db, "xl_at", 2, xl_at, FunctionFlags::UTF8)?;
    define_scalar_function(db, "xl_version", 0, xl_version, FunctionFlags::UTF8)?;
    Ok(())
}

#[cfg(target_os = "emscripten")]
#[no_mangle]
pub extern "C" fn sqlite3_wasm_extra_init(_unused: *const std::ffi::c_char) -> std::ffi::c_int {
    use sqlite_loadable::SQLITE_OKAY;
    unsafe {
        sqlite_loadable::ext::sqlite3ext_auto_extension(std::mem::transmute(
            sqlite3_xl_init as *const (),
        ));
    }

    SQLITE_OKAY
}
