use calamine::Reader;
use sqlite_loadable::prelude::*;
use sqlite_loadable::{
    api, define_table_function,
    table::{BestIndexError, ConstraintOperator, IndexInfo, VTab, VTabArguments, VTabCursor},
    Result,
};

use std::{mem, os::raw::c_int};

static CREATE_SQL: &str = "CREATE TABLE x(name, visible, workbook hidden)";
enum Columns {
    Name,
    Visible,
    Workbook,
}
fn column(index: i32) -> Option<Columns> {
    match index {
        0 => Some(Columns::Name),
        1 => Some(Columns::Visible),
        2 => Some(Columns::Workbook),
        _ => None,
    }
}

#[repr(C)]
pub struct SheetsTable {
    /// must be first
    base: sqlite3_vtab,
}

impl<'vtab> VTab<'vtab> for SheetsTable {
    type Aux = ();
    type Cursor = SheetsCursor;

    fn connect(
        _db: *mut sqlite3,
        _aux: Option<&Self::Aux>,
        _args: VTabArguments,
    ) -> Result<(String, SheetsTable)> {
        let base: sqlite3_vtab = unsafe { mem::zeroed() };
        let vtab = SheetsTable { base };
        // TODO db.config(VTabConfig::Innocuous)?;
        Ok((CREATE_SQL.to_owned(), vtab))
    }
    fn destroy(&self) -> Result<()> {
        Ok(())
    }

    fn best_index(&self, mut info: IndexInfo) -> core::result::Result<(), BestIndexError> {
        let mut has_workbook = false;
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
                _ => (),
            }
        }
        if !has_workbook {
            return Err(BestIndexError::Error);
        }
        info.set_estimated_cost(100000.0);
        info.set_estimated_rows(100000);
        info.set_idxnum(1);

        Ok(())
    }

    fn open(&mut self) -> Result<SheetsCursor> {
        Ok(SheetsCursor::new())
    }
}

#[repr(C)]
pub struct SheetsCursor {
    /// Base class. Must be first
    base: sqlite3_vtab_cursor,
    rowid: i64,
    workbook: Option<calamine::Sheets<std::io::Cursor<Vec<u8>>>>,
}
impl SheetsCursor {
    fn new() -> SheetsCursor {
        let base: sqlite3_vtab_cursor = unsafe { mem::zeroed() };
        SheetsCursor {
            base,
            rowid: 0,
            workbook: None,
        }
    }
}

impl VTabCursor for SheetsCursor {
    fn filter(
        &mut self,
        _idx_num: c_int,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        let raw = api::value_blob(values.get(0).expect("1st min constraint is required"));
        let data = raw.to_vec();
        self.workbook =
            Some(calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data)).unwrap());
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        self.rowid += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        self.workbook
            .as_ref()
            .unwrap()
            .sheets_metadata()
            .get(self.rowid as usize)
            .is_none()
    }

    fn column(&self, context: *mut sqlite3_context, i: c_int) -> Result<()> {
        let m = self
            .workbook
            .as_ref()
            .unwrap()
            .sheets_metadata()
            .get(self.rowid as usize)
            .unwrap();
        match column(i) {
            Some(Columns::Name) => {
                api::result_text(context, &m.name)?;
            }
            Some(Columns::Visible) => {}
            Some(Columns::Workbook) => {
                //context_result_int(0);
            }
            _ => (),
        }
        Ok(())
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.rowid)
    }
}
