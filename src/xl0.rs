use sqlite_loadable::prelude::*;
use sqlite_loadable::vtab_argparse::{parse_argument, Argument, ColumnDeclaration, ConfigOptionValue};
use sqlite_loadable::{
    api,
    api::ColumnAffinity,
    table::{BestIndexError, ConstraintOperator, IndexInfo, VTab, VTabArguments, VTabCursor},
    Error, Result,
};
use std::{mem, os::raw::c_int};

use calamine::{Data, DataType, Reader};

use crate::parser::column_idx_to_name;
use crate::sheet_range::{parse_sheet_reference, SheetTarget};

/// Apply column affinity to a calamine Data value.
/// Coerces the value to match the declared type when possible.
fn result_xl_data_with_affinity(
    context: *mut sqlite3_context,
    data: &Data,
    affinity: &ColumnAffinity,
) -> Result<()> {
    match affinity {
        ColumnAffinity::Integer => match data {
            Data::Int(v) => api::result_int64(context, *v),
            Data::Float(v) => api::result_int64(context, *v as i64),
            Data::Bool(v) => api::result_int64(context, if *v { 1 } else { 0 }),
            Data::String(s) => {
                if let Ok(v) = s.parse::<i64>() {
                    api::result_int64(context, v);
                } else {
                    api::result_text(context, s)?;
                }
            }
            _ => crate::result_xl_data(context, data)?,
        },
        ColumnAffinity::Real => match data {
            Data::Float(v) => api::result_double(context, *v),
            Data::Int(v) => api::result_double(context, *v as f64),
            Data::String(s) => {
                if let Ok(v) = s.parse::<f64>() {
                    api::result_double(context, v);
                } else {
                    api::result_text(context, s)?;
                }
            }
            _ => crate::result_xl_data(context, data)?,
        },
        ColumnAffinity::Text => match data {
            Data::String(s) => api::result_text(context, s)?,
            Data::Int(v) => api::result_text(context, v.to_string())?,
            Data::Float(v) => api::result_text(context, v.to_string())?,
            Data::Bool(v) => api::result_text(context, if *v { "true" } else { "false" })?,
            Data::Empty => api::result_null(context),
            _ => crate::result_xl_data(context, data)?,
        },
        _ => crate::result_xl_data(context, data)?,
    }
    Ok(())
}

#[repr(C)]
pub struct XL0Table {
    base: sqlite3_vtab,
    filename: Option<String>,
    sheet_name: Option<String>,
    start_row: u32,
    end_row: Option<u32>,
    start_col: u32,
    end_col: Option<u32>,
    num_columns: usize,
    has_headers: bool,
    declared_types: Vec<Option<String>>,
    /// Index of the hidden "source" column (only set when filename is omitted)
    source_column_idx: Option<usize>,
}

impl<'vtab> VTab<'vtab> for XL0Table {
    type Aux = ();
    type Cursor = XL0Cursor;

    fn connect(
        _db: *mut sqlite3,
        _aux: Option<&Self::Aux>,
        args: VTabArguments,
    ) -> Result<(String, XL0Table)> {
        let mut filename: Option<String> = None;
        let mut range_str: Option<String> = None;
        let mut has_headers = false;
        let mut explicit_columns: Vec<ColumnDeclaration> = Vec::new();

        for arg_str in &args.arguments {
            match parse_argument(arg_str).map_err(|e| Error::new_message(e))? {
                Argument::Config(config) => match config.key.as_str() {
                    "filename" => {
                        filename = Some(match config.value {
                            ConfigOptionValue::Quoted(s) => s,
                            ConfigOptionValue::Bareword(s) => s,
                            _ => {
                                return Err(Error::new_message(
                                    "filename must be a string",
                                ))
                            }
                        });
                    }
                    "range" => {
                        range_str = Some(match config.value {
                            ConfigOptionValue::Quoted(s) => s,
                            ConfigOptionValue::Bareword(s) => s,
                            _ => {
                                return Err(Error::new_message("range must be a string"))
                            }
                        });
                    }
                    "headers" => {
                        has_headers = match config.value {
                            ConfigOptionValue::Bareword(s) => {
                                s == "1" || s.eq_ignore_ascii_case("true")
                            }
                            _ => false,
                        };
                    }
                    other => {
                        return Err(Error::new_message(format!(
                            "unknown option: '{other}'"
                        )));
                    }
                },
                Argument::Column(col) => {
                    explicit_columns.push(col);
                }
            }
        }

        // Parse range to get sheet + bounds
        let (sheet_name, start_row, end_row, start_col, end_col) = match &range_str {
            Some(r) => {
                let parsed = parse_sheet_reference(r)
                    .map_err(|e| Error::new_message(format!("invalid range: {e}")))?;
                let sheet = parsed.sheet;
                match parsed.target {
                    SheetTarget::Range(r) => {
                        (sheet, r.start.1, Some(r.end.1), r.start.0, Some(r.end.0))
                    }
                    SheetTarget::OpenRange(r) => {
                        let sr = r.start.row.unwrap_or(0);
                        let sc = r.start.col.unwrap_or(0);
                        (sheet, sr, r.end.row, sc, r.end.col)
                    }
                    SheetTarget::Cell(c) => {
                        (sheet, c.location.1, Some(c.location.1), c.location.0, Some(c.location.0))
                    }
                }
            }
            None => (None, 0, None, 0, None),
        };

        if filename.is_some() {
            // ── filename provided: resolve columns at CREATE time ──
            let data = std::fs::read(filename.as_ref().unwrap())
                .map_err(|e| Error::new_message(format!("cannot read '{}': {e}", filename.as_ref().unwrap())))?;
            let mut workbook =
                calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data))
                    .map_err(|e| Error::new_message(format!("cannot open workbook: {e}")))?;

            let sheet = match &sheet_name {
                Some(name) => name.clone(),
                None => workbook.sheet_names().first().unwrap().clone(),
            };

            let worksheet = workbook
                .worksheet_range(&sheet)
                .map_err(|_| Error::new_message(format!("sheet '{sheet}' not found")))?;

            let actual_end_col = end_col.unwrap_or_else(|| {
                worksheet
                    .rows()
                    .skip(start_row as usize)
                    .map(|r| r.len() as u32)
                    .max()
                    .unwrap_or(0)
                    .saturating_sub(1)
            });
            let num_columns = (actual_end_col - start_col + 1) as usize;

            let (create_sql, declared_types) = build_create_sql(
                &explicit_columns, has_headers, num_columns,
                start_col, actual_end_col, &worksheet, start_row, None,
            )?;

            let vtab = XL0Table {
                base: unsafe { mem::zeroed() },
                filename,
                sheet_name: Some(sheet),
                start_row,
                end_row,
                start_col,
                end_col: Some(actual_end_col),
                num_columns,
                has_headers,
                declared_types,
                source_column_idx: None,
            };
            Ok((create_sql, vtab))
        } else {
            // ── no filename: require explicit columns, add hidden "source" column ──
            if explicit_columns.is_empty() && !has_headers {
                return Err(Error::new_message(
                    "either filename or explicit column declarations are required"
                ));
            }

            let num_columns = if !explicit_columns.is_empty() {
                explicit_columns.len()
            } else {
                // headers mode without filename: we can't know column count yet,
                // but we need explicit columns in this mode
                return Err(Error::new_message(
                    "headers=1 requires filename; use explicit column names instead"
                ));
            };

            // Infer end_col from start_col + num_columns
            let actual_end_col = end_col.unwrap_or(start_col + num_columns as u32 - 1);

            let types: Vec<Option<String>> = explicit_columns
                .iter()
                .map(|c| c.declared_type.clone())
                .collect();
            let cols_sql: Vec<String> =
                explicit_columns.iter().map(|c| c.vtab_declaration()).collect();
            let source_idx = num_columns;
            let create_sql = format!(
                "CREATE TABLE x({}, source hidden)",
                cols_sql.join(", ")
            );

            let vtab = XL0Table {
                base: unsafe { mem::zeroed() },
                filename: None,
                sheet_name,
                start_row,
                end_row,
                start_col,
                end_col: Some(actual_end_col),
                num_columns,
                has_headers,
                declared_types: types,
                source_column_idx: Some(source_idx),
            };
            Ok((create_sql, vtab))
        }
    }

    fn destroy(&self) -> Result<()> {
        Ok(())
    }

    fn best_index(&self, mut info: IndexInfo) -> core::result::Result<(), BestIndexError> {
        if let Some(source_idx) = self.source_column_idx {
            // Source-at-query-time mode: require source column constraint
            let mut has_source = false;
            for mut constraint in info.constraints() {
                if constraint.column_idx() == source_idx as i32 {
                    if constraint.usable() && constraint.op() == Some(ConstraintOperator::EQ) {
                        constraint.set_omit(true);
                        constraint.set_argv_index(1);
                        has_source = true;
                    } else {
                        return Err(BestIndexError::Constraint);
                    }
                }
            }
            if !has_source {
                return Err(BestIndexError::Error);
            }
            info.set_idxnum(1);
        } else {
            info.set_idxnum(0);
        }
        info.set_estimated_cost(100000.0);
        info.set_estimated_rows(100000);
        Ok(())
    }

    fn open(&mut self) -> Result<XL0Cursor> {
        Ok(XL0Cursor::new())
    }
}

/// Build the CREATE TABLE SQL and declared_types from column declarations.
fn build_create_sql(
    explicit_columns: &[ColumnDeclaration],
    has_headers: bool,
    num_columns: usize,
    start_col: u32,
    end_col: u32,
    worksheet: &calamine::Range<Data>,
    start_row: u32,
    source_suffix: Option<&str>,
) -> Result<(String, Vec<Option<String>>)> {
    let suffix = source_suffix.unwrap_or("");

    if !explicit_columns.is_empty() {
        if explicit_columns.len() != num_columns {
            return Err(Error::new_message(format!(
                "expected {} column names but got {} (range spans columns {}-{})",
                num_columns,
                explicit_columns.len(),
                column_idx_to_name(start_col),
                column_idx_to_name(end_col),
            )));
        }
        let types: Vec<Option<String>> = explicit_columns
            .iter()
            .map(|c| c.declared_type.clone())
            .collect();
        let cols_sql: Vec<String> =
            explicit_columns.iter().map(|c| c.vtab_declaration()).collect();
        let sql = format!("CREATE TABLE x({}{})", cols_sql.join(", "), suffix);
        Ok((sql, types))
    } else if has_headers {
        let header_row = worksheet
            .rows()
            .nth(start_row as usize)
            .ok_or_else(|| Error::new_message("header row is out of range"))?;
        let names: Vec<String> = (start_col..=end_col)
            .map(|c| {
                header_row
                    .get(c as usize)
                    .and_then(|d| match d {
                        Data::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| column_idx_to_name(c))
            })
            .collect();
        let types = vec![None; num_columns];
        let cols_sql: Vec<String> = names
            .iter()
            .map(|name| format!("'{}'", name.replace('\'', "''")))
            .collect();
        let sql = format!("CREATE TABLE x({}{})", cols_sql.join(", "), suffix);
        Ok((sql, types))
    } else {
        let names: Vec<String> = (start_col..=end_col)
            .map(column_idx_to_name)
            .collect();
        let types = vec![None; num_columns];
        let cols_sql: Vec<String> = names
            .iter()
            .map(|name| format!("'{}'", name.replace('\'', "''")))
            .collect();
        let sql = format!("CREATE TABLE x({}{})", cols_sql.join(", "), suffix);
        Ok((sql, types))
    }
}

#[repr(C)]
pub struct XL0Cursor {
    base: sqlite3_vtab_cursor,
    rowid: i64,
    rows: Option<Vec<Vec<Data>>>,
}

impl XL0Cursor {
    fn new() -> XL0Cursor {
        let base: sqlite3_vtab_cursor = unsafe { mem::zeroed() };
        XL0Cursor {
            base,
            rowid: 0,
            rows: None,
        }
    }
}

impl VTabCursor for XL0Cursor {
    fn filter(
        &mut self,
        idx_num: c_int,
        _idx_str: Option<&str>,
        values: &[*mut sqlite3_value],
    ) -> Result<()> {
        let vtab = unsafe { &*(self.base.pVtab as *const XL0Table) };

        let data = if idx_num == 1 {
            // Source provided at query time via hidden column
            api::value_blob(values.first().expect("source argument is required")).to_vec()
        } else {
            // Read from filename set at CREATE time
            std::fs::read(vtab.filename.as_ref().unwrap())
                .map_err(|e| Error::new_message(format!("cannot read '{}': {e}", vtab.filename.as_ref().unwrap())))?
        };

        let mut workbook =
            calamine::open_workbook_auto_from_rs(std::io::Cursor::new(data))
                .map_err(|e| Error::new_message(format!("cannot open workbook: {e}")))?;

        let sheet_name = match vtab.sheet_name.as_ref() {
            Some(name) => name.clone(),
            None => workbook.sheet_names().first().unwrap().clone(),
        };
        let worksheet = workbook
            .worksheet_range(&sheet_name)
            .map_err(|_| Error::new_message(format!("sheet '{sheet_name}' not found")))?;

        let data_start = if vtab.has_headers {
            vtab.start_row as usize + 1
        } else {
            vtab.start_row as usize
        };

        let start_col = vtab.start_col as usize;
        let end_col = vtab.end_col.unwrap() as usize;

        let all_rows: Vec<&[Data]> = worksheet.rows().collect();

        let end_idx = vtab
            .end_row
            .map(|er| (er as usize) + 1)
            .unwrap_or(all_rows.len());

        let mut rows: Vec<Vec<Data>> = Vec::new();
        for row_idx in data_start..end_idx {
            if let Some(row_data) = all_rows.get(row_idx) {
                let cells: Vec<Data> = (start_col..=end_col)
                    .map(|c| {
                        row_data
                            .get(c)
                            .cloned()
                            .unwrap_or(Data::Empty)
                    })
                    .collect();
                rows.push(cells);
            }
        }

        self.rows = Some(rows);
        self.rowid = 0;
        Ok(())
    }

    fn next(&mut self) -> Result<()> {
        self.rowid += 1;
        Ok(())
    }

    fn eof(&self) -> bool {
        self.rows
            .as_ref()
            .unwrap()
            .get(self.rowid as usize)
            .is_none()
    }

    fn column(&self, context: *mut sqlite3_context, i: c_int) -> Result<()> {
        let vtab = unsafe { &*(self.base.pVtab as *const XL0Table) };

        // If this is the hidden source column, return null
        if Some(i as usize) == vtab.source_column_idx {
            api::result_null(context);
            return Ok(());
        }

        let row = self
            .rows
            .as_ref()
            .unwrap()
            .get(self.rowid as usize)
            .unwrap();
        match row.get(i as usize) {
            Some(data) => {
                let affinity = vtab.declared_types[i as usize]
                    .as_ref()
                    .map(|dt| ColumnAffinity::from_declared_type(dt))
                    .unwrap_or(ColumnAffinity::Blob);
                result_xl_data_with_affinity(context, data, &affinity)?;
            }
            None => api::result_null(context),
        }
        Ok(())
    }

    fn rowid(&self) -> Result<i64> {
        Ok(self.rowid)
    }
}
