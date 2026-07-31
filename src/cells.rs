use sqlite_loadable::prelude::*;
use sqlite_loadable::{
    api,
    table::{BestIndexError, ConstraintOperator, IndexInfo, VTab, VTabArguments, VTabCursor},
    Result,
};
use std::{mem, os::raw::c_int};

use calamine::{Data, Reader};

use crate::parser::column_idx_to_name;
use crate::sheet_range::{parse_sheet_reference, SheetTarget};

static CREATE_SQL: &str = "CREATE TABLE x(column_name, row_number, value, workbook hidden, range hidden, sheet hidden)";
enum Columns {
  ColumnName,
    RowNumber,
    Value,
    Workbook,
    Range,
    Sheet,
}
fn column(index: i32) -> Option<Columns> {
    match index {
        0 => Some(Columns::ColumnName),
        1 => Some(Columns::RowNumber),
        2 => Some(Columns::Value),
        3 => Some(Columns::Workbook),
        4 => Some(Columns::Range),
        5 => Some(Columns::Sheet),
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
                Some(Columns::Range) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(2);
                        has_range = true;
                    } else {
                        return Err(BestIndexError::Constraint);
                    }
                }
                Some(Columns::Sheet) => {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(3);
                        has_sheet = true;
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
        info.set_idxnum(if has_sheet { 2 } else { 1 });

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
    values: Option<Vec<(usize, usize, Data)>>,
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
        idx_num: c_int,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        let raw = api::value_blob(values.first().expect("1st min constraint is required"));
        let range_str = api::value_text(values.get(1).unwrap()).unwrap();
        let data = raw.to_vec();
        let mut workbook =
            calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data))
                .map_err(|e| crate::Error::new_message(format!("cannot open workbook: {e}")))?;
        let parsed = parse_sheet_reference(range_str)
            .map_err(|e| crate::Error::new_message(format!("invalid range: {e}")))?;

        // Use sheet from parsed reference, then explicit 3rd arg, then default to first sheet
        let sheet_name = if let Some(ref s) = parsed.sheet {
            s.clone()
        } else if idx_num == 2 {
            api::value_text(values.get(2).unwrap())?.to_owned()
        } else {
            workbook.sheet_names().first().unwrap().clone()
        };

        let worksheet_range = workbook.worksheet_range(&sheet_name)
            .map_err(|_| crate::Error::new_message(format!("sheet '{}' not found", sheet_name)))?;

        let all_rows: Vec<&[Data]> = worksheet_range.rows().collect();
        let total_rows = all_rows.len();
        let max_cols = all_rows.iter().map(|r| r.len()).max().unwrap_or(0);

        // Resolve bounds from the parsed target
        let (start_col, start_row, end_col, end_row) = match parsed.target {
            SheetTarget::Range(r) => {
                (r.start.0 as usize, r.start.1 as usize, r.end.0 as usize, r.end.1 as usize)
            }
            SheetTarget::OpenRange(r) => {
                let sc = r.start.col.unwrap_or(0) as usize;
                let sr = r.start.row.unwrap_or(0) as usize;
                let ec = r.end.col.map(|c| c as usize).unwrap_or_else(|| max_cols.saturating_sub(1));
                let er = r.end.row.map(|r| r as usize).unwrap_or_else(|| total_rows.saturating_sub(1));
                (sc, sr, ec, er)
            }
            SheetTarget::Cell(c) => {
                (c.location.0 as usize, c.location.1 as usize, c.location.0 as usize, c.location.1 as usize)
            }
        };

        let mut values: Vec<(usize, usize, Data)> = Vec::new();
        for row_idx in start_row..=end_row {
            if let Some(row_data) = all_rows.get(row_idx) {
                for col_idx in start_col..=end_col {
                    if let Some(cell) = row_data.get(col_idx) {
                        values.push((row_idx, col_idx, cell.to_owned()));
                    }
                }
            }
        }
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
                api::result_int64(context, (v.0 + 1).try_into().unwrap());
            }
            Some(Columns::ColumnName) => {
                api::result_text(context, column_idx_to_name(v.1.try_into().unwrap()))?;
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
