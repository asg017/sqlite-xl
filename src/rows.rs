use sqlite_loadable::prelude::*;
use sqlite_loadable::scalar::scalar_function_raw;
use sqlite_loadable::table::VTabFind;
use sqlite_loadable::{
    api,
    table::{BestIndexError, ConstraintOperator, IndexInfo, VTab, VTabArguments, VTabCursor},
    Result,
};
use std::ffi::c_void;
use std::{mem, os::raw::c_int};

use calamine::{Data, Reader};

use crate::sheet_range::{parse_sheet_reference, SheetTarget};

static CREATE_SQL: &str = "CREATE TABLE x(row_number, row, workbook hidden, sheet hidden)";
enum Columns {
    RowNumber,
    Row,
    Workbook,
    Sheet,
}
fn column(index: i32) -> Option<Columns> {
    match index {
        0 => Some(Columns::RowNumber),
        1 => Some(Columns::Row),
        2 => Some(Columns::Workbook),
        3 => Some(Columns::Sheet),
        _ => None,
    }
}

#[repr(C)]
pub struct RowsTable {
    /// must be first
    base: sqlite3_vtab,
}

impl<'vtab> VTab<'vtab> for RowsTable {
    type Aux = ();
    type Cursor = RowsCursor;

    fn connect(
        _db: *mut sqlite3,
        _aux: Option<&Self::Aux>,
        _args: VTabArguments,
    ) -> Result<(String, RowsTable)> {
        let base: sqlite3_vtab = unsafe { mem::zeroed() };
        let vtab = RowsTable { base };
        // TODO db.config(VTabConfig::Innocuous)?;
        Ok((CREATE_SQL.to_owned(), vtab))
    }
    fn destroy(&self) -> Result<()> {
        Ok(())
    }

    fn best_index(&self, mut info: IndexInfo) -> core::result::Result<(), BestIndexError> {
        let mut has_workbook = false;
        let mut has_sheet = false;
        for mut constraint in info.constraints() {
            match column(constraint.column_idx()) {
                Some(Columns::Workbook) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(1);
                        has_workbook = true;
                    } else {
                        return Err(BestIndexError::Constraint);
                    }
                }
                Some(Columns::Sheet) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(2);
                        has_sheet = true;
                    }
                }
                _ => (),
            }
        }
        if !has_workbook {
            return Err(BestIndexError::Error);
        }
        info.set_estimated_cost(100000.0);
        info.set_estimated_rows(100000);
        info.set_idxnum(if has_sheet { 2 } else { 1 });

        Ok(())
    }

    fn open(&mut self) -> Result<RowsCursor> {
        Ok(RowsCursor::new())
    }
}

impl<'vtab> VTabFind<'vtab> for RowsTable {
  fn find_function(
      &mut self,
      argc: i32,
      name: &str,
  ) -> Option<(
      unsafe extern "C" fn(*mut sqlite3_context, i32, *mut *mut sqlite3_value),
      Option<i32>,
      Option<*mut c_void>,
  )> {
      if name == "->>" && argc == 2 {
        let(f, p) = scalar_function_raw(crate::xl_at);
        return Some((f, None, Some(p.cast())));
          //return Some((scalar_function_raw(crate::xl_at), None, None));
      }
      None
  }
}

#[repr(C)]
pub struct RowsCursor {
    /// Base class. Must be first
    base: sqlite3_vtab_cursor,
    rowid: i64,
    start_row_number: Option<u32>,
    values: Option<Vec<Vec<Data>>>,
}
impl RowsCursor {
    fn new() -> RowsCursor {
        let base: sqlite3_vtab_cursor = unsafe { mem::zeroed() };
        RowsCursor {
            base,
            rowid: 0,
            start_row_number: None,
            values: None,
        }
    }
}

impl VTabCursor for RowsCursor {
    fn filter(
        &mut self,
        idx_num: c_int,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        let raw = api::value_blob(values.first().expect("1st min constraint is required"));
        let data = raw.to_vec();
        let mut workbook =
            calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data))
                .map_err(|e| sqlite_loadable::Error::new_message(format!("cannot open workbook: {e}")))?;

        // Parse the optional second argument: can be a plain sheet name,
        // or a sheet-qualified reference like 'Sheet1!A13:*'
        let (sheet_name, start_row, end_row) = if idx_num == 2 {
            let arg = api::value_text(values.get(1).unwrap())?;
            match parse_sheet_reference(arg) {
                Ok(ref parsed) if parsed.sheet.is_some() => {
                    let sheet = parsed.sheet.clone().unwrap();
                    let (sr, er) = match &parsed.target {
                        SheetTarget::Range(r) => (Some(r.start.1), Some(r.end.1)),
                        SheetTarget::OpenRange(r) => (r.start.row, r.end.row),
                        SheetTarget::Cell(c) => (Some(c.location.1), Some(c.location.1)),
                    };
                    (sheet, sr, er)
                }
                _ => {
                    // No '!' found or parse failed — treat as plain sheet name
                    (arg.to_owned(), None, None)
                }
            }
        } else {
            (workbook.sheet_names().first().unwrap().clone(), None, None)
        };

        let worksheet_range = workbook.worksheet_range(&sheet_name)
            .map_err(|_| sqlite_loadable::Error::new_message(format!("sheet '{}' not found", sheet_name)))?;
        let ws_start_row = worksheet_range.start().map(|(_, r)| r).unwrap_or(0);

        // Apply row bounds from the parsed range
        let skip = start_row.map(|sr| sr.saturating_sub(ws_start_row) as usize).unwrap_or(0);
        let take = end_row.map(|er| (er.saturating_sub(ws_start_row) as usize) + 1 - skip);

        self.start_row_number = Some(ws_start_row + skip as u32 + 1);
        let iter = worksheet_range.rows().skip(skip);
        let values: Vec<Vec<Data>> = match take {
            Some(n) => iter.take(n).map(|v| v.to_owned()).collect(),
            None => iter.map(|v| v.to_owned()).collect(),
        };
        self.values = Some(values);
        self.rowid = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        self.rowid += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        self.values
            .as_ref()
            .unwrap()
            .get(self.rowid as usize)
            .is_none()
    }

    fn column(&self, context: *mut sqlite3_context, i: c_int) -> Result<()> {
        let v = self
            .values
            .as_ref()
            .unwrap()
            .get(self.rowid as usize)
            .unwrap();
        match column(i) {
          Some(Columns::RowNumber) => {
            api::result_int64(context, self.rowid + self.start_row_number.unwrap()as i64);
          }
            Some(Columns::Row) => {
                api::result_pointer(context, b"ROW\0", v.to_owned());
            }
            Some(Columns::Workbook) | Some(Columns::Sheet) => {
                //context_result_int(0);
            }
            None => (),
        }
        Ok(())
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.rowid)
    }
}
