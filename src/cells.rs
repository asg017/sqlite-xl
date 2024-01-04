use sqlite_loadable::prelude::*;
use sqlite_loadable::{
    api,
    table::{BestIndexError, ConstraintOperator, IndexInfo, VTab, VTabArguments, VTabCursor},
    Result,
};
use std::{mem, os::raw::c_int};

use calamine::{DataType, Reader};

use crate::parser::parse_range_reference;

static CREATE_SQL: &str = "CREATE TABLE x(row, column, value, workbook hidden, range hidden)";
enum Columns {
    Row,
    Column,
    Value,
    Workbook,
    Range,
}
fn column(index: i32) -> Option<Columns> {
    match index {
        0 => Some(Columns::Row),
        1 => Some(Columns::Column),
        2 => Some(Columns::Value),
        3 => Some(Columns::Workbook),
        4 => Some(Columns::Range),
        _ => None,
    }
}

#[repr(C)]
pub struct CellsTable {
    /// must be first
    base: sqlite3_vtab,
}

impl<'vtab> VTab<'vtab> for CellsTable {
    type Aux = ();
    type Cursor = CellsCursor;

    fn connect(
        _db: *mut sqlite3,
        _aux: Option<&Self::Aux>,
        _args: VTabArguments,
    ) -> Result<(String, CellsTable)> {
        let base: sqlite3_vtab = unsafe { mem::zeroed() };
        let vtab = CellsTable { base };
        // TODO db.config(VTabConfig::Innocuous)?;
        Ok((CREATE_SQL.to_owned(), vtab))
    }
    fn destroy(&self) -> Result<()> {
        Ok(())
    }

    fn best_index(&self, mut info: IndexInfo) -> core::result::Result<(), BestIndexError> {
        let mut has_workbook = false;
        let mut has_range = false;
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
                Some(Columns::Range) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(2);
                        has_range = true;
                    } else {
                        return Err(BestIndexError::Constraint);
                    }
                }
                _ => (),
            }
        }
        if !has_workbook || !has_range {
            return Err(BestIndexError::Error);
        }
        info.set_estimated_cost(100000.0);
        info.set_estimated_rows(100000);
        info.set_idxnum(1);

        Ok(())
    }

    fn open(&mut self) -> Result<CellsCursor> {
        Ok(CellsCursor::new())
    }
}

#[repr(C)]
pub struct CellsCursor {
    /// Base class. Must be first
    base: sqlite3_vtab_cursor,
    rowid: i64,
    values: Option<Vec<(usize, usize, DataType)>>,
}
impl CellsCursor {
    fn new() -> CellsCursor {
        let base: sqlite3_vtab_cursor = unsafe { mem::zeroed() };
        CellsCursor {
            base,
            rowid: 0,
            values: None,
        }
    }
}

impl VTabCursor for CellsCursor {
    fn filter(
        &mut self,
        _idx_num: c_int,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        let raw = api::value_blob(values.get(0).expect("1st min constraint is required"));
        let range = api::value_text(values.get(1).unwrap()).unwrap();
        let data = raw.to_vec();
        let mut workbook =
            calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data)).unwrap();
        let range = parse_range_reference(range).unwrap();
        let worksheet_range = workbook.worksheet_range("Sheet1").unwrap();
        let values: Vec<(usize, usize, DataType)> = worksheet_range
            .cells()
            .filter_map(|(row, column, v)| {
                if (row >= range.start.1 as usize && row <= range.end.1 as usize)
                    && (column >= range.start.0 as usize && column <= range.end.0 as usize)
                {
                    Some((row + range.start.1 as usize, column, v.to_owned()))
                } else {
                    None
                }
            })
            .collect();
        self.values = Some(values);
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
            Some(Columns::Row) => {
                api::result_int(context, v.0.try_into().unwrap());
            }
            Some(Columns::Column) => {
                api::result_int(context, v.1.try_into().unwrap());
            }
            Some(Columns::Value) => {
                crate::result_xl_data(context, &v.2)?;
            }
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
