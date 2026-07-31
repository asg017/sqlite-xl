.load dist/debug/xl0

select xl_version(); -- 'v0.0.1-alpha.4'

-- xl_sheets: list sheets in a workbook
select * from xl_sheets(readfile('tests/sample-abc.xlsx')); -- @snap xl_sheets

select * from xl_sheets(readfile('tests/students.xlsx')); -- @snap xl_sheets_students

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

-- xl_rows: students
select
  xl_at(row, 'A') as id,
  xl_at(row, 'B') as name,
  xl_at(row, 'C') as grade_level,
  xl_at(row, 'D') as email
from xl_rows(readfile('tests/students.xlsx'), 'students')
where row_number > 1; -- @snap xl_rows_students

-- xl_rows: grades from a different sheet
select
  xl_at(row, 'A') as student_id,
  xl_at(row, 'B') as assignment_id,
  xl_at(row, 'C') as score,
  xl_at(row, 'D') as submitted
from xl_rows(readfile('tests/students.xlsx'), 'grades')
where row_number > 1
limit 5; -- @snap xl_rows_grades

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

-- xl_cells: assignments sheet
select * from xl_cells(readfile('tests/students.xlsx'), 'A1:D3', 'assignments'); -- @snap xl_cells_assignments

-- xl_rows: sheet!range syntax with row bounds
select
  xl_at(row, 'A') as student_id,
  xl_at(row, 'D') as submitted,
  xl_at(row, 'E') as time_spent
from xl_rows(readfile('tests/students.xlsx'), 'grades!A2:E4')
limit 5; -- @snap xl_rows_range
