.load dist/debug/xl0

select xl_version(); -- 'v0.0.1-alpha.4'

-- xl_sheets: list sheets in a workbook
select * from xl_sheets(readfile('tests/sample-abc.xlsx')); -- @snap xl_sheets

-- xl_rows: read rows from the first sheet (default)
select
  rowid,
  row_number,
  row,
  row ->> 'A',
  row ->> 'B',
  row ->> 'C',
  row ->> 'D',
  row ->> 'E',
  row ->> 'F'
from xl_rows(readfile('tests/file-sample.xlsx'))
limit 6; -- @snap xl_rows

-- xl_rows: select a specific sheet by name
select
  row_number,
  xl_at(row, 'A'),
  xl_at(row, 'B')
from xl_rows(readfile('tests/sample-abc.xlsx'), 'bbb')
limit 3; -- @snap xl_rows_sheet

-- xl_at: extract a cell value from a row
select
  xl_at(row, 'A') as by_name,
  xl_at(row, 0) as by_index
from xl_rows(readfile('tests/file-sample.xlsx'))
limit 1; -- @snap xl_at

-- xl_at: null row returns error
select xl_at(null, 0); -- error: 1st argument must be a row

-- xl_cells: unpivoted cell view with range
select rowid, * from xl_cells(readfile('tests/file-sample.xlsx'), 'A1:E2'); -- @snap xl_cells

-- xl_cells: single row range
select rowid, * from xl_cells(readfile('tests/file-sample.xlsx'), 'A1:F1'); -- @snap xl_cells_row

-- xl_cells: select a specific sheet by name
select * from xl_cells(readfile('tests/sample-abc.xlsx'), 'A1:B3', 'bbb'); -- @snap xl_cells_sheet
