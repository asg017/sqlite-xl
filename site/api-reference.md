# API Reference

A complete reference to all the SQL scalar and table functions inside `sqlite-xl`.

::: warning
sqlite-xl is pre-v1, so expect breaking changes.
:::

<style>
  h3 {font-family: monospace}
  .VPDocOutlineItem.nested {font-family: monospace}
</style>

## Scalar Functions

### `xl_version()` {#xl_version}

Returns the current version of `sqlite-xl`.

```sql
select xl_version();
-- 'v0.0.1-alpha.4'
```

### `xl_at(row, column)` {#xl_at}

Extracts a cell value from a `row` pointer returned by `xl_rows()`. The `column` argument can be a column letter (like `'A'`, `'B'`, `'Z'`) or a 0-based integer index.

```sql
select
  xl_at(row, 'A') as id,
  xl_at(row, 'B') as name,
  xl_at(row, 'C') as grade_level
from xl_rows(readfile('tests/students.xlsx'), 'students')
where row_number > 1
limit 3;
/*
┌────┬───────────────┬─────────────┐
│ id │ name          │ grade_level │
├────┼───────────────┼─────────────┤
│ 1  │ 'Alice Chen'  │ 10          │
│ 2  │ 'Bob Jones'   │ 11          │
│ 3  │ 'Clara Smith' │ 10          │
└────┴───────────────┴─────────────┘
*/
```

You can also use the `->>` operator as a shorthand for `xl_at()` when querying `xl_rows()`:

```sql
select
  row ->> 'A' as id,
  row ->> 'B' as title,
  row ->> 'C' as subject
from xl_rows(readfile('tests/students.xlsx'), 'assignments')
where row_number > 1;
/*
┌─────┬───────────────────────────┬───────────┐
│ id  │ title                     │ subject   │
├─────┼───────────────────────────┼───────────┤
│ 101 │ 'Essay: Modern Poetry'    │ 'English' │
│ 102 │ 'Lab: Chemical Reactions' │ 'Science' │
│ 103 │ 'Problem Set 5'           │ 'Math'    │
│ 104 │ 'History Presentation'    │ 'History' │
└─────┴───────────────────────────┴───────────┘
*/
```

## Table Functions

### `xl_sheets(workbook)` {#xl_sheets}

Lists all sheets in a workbook.

```sql
select * from xl_sheets(readfile('tests/students.xlsx'));
/*
┌───────────────┬─────────┐
│ name          │ visible │
├───────────────┼─────────┤
│ 'students'    │ NULL    │
│ 'assignments' │ NULL    │
│ 'grades'      │ NULL    │
└───────────────┴─────────┘
*/
```

### `xl_rows(workbook)` {#xl_rows}

Returns one row per row in the worksheet. Each row has a `row_number` column and a `row` pointer column. Use `xl_at()` or `->>` to extract cell values from the row.

By default, reads the first sheet.

```sql
select
  row_number,
  xl_at(row, 'A') as id,
  xl_at(row, 'B') as name,
  xl_at(row, 'C') as grade_level,
  xl_at(row, 'D') as email
from xl_rows(readfile('tests/students.xlsx'))
limit 4;
/*
┌────────────┬──────┬───────────────┬───────────────┬────────────────────┐
│ row_number │ id   │ name          │ grade_level   │ email              │
├────────────┼──────┼───────────────┼───────────────┼────────────────────┤
│ 1          │ 'id' │ 'name'        │ 'grade_level' │ 'email'            │
│ 2          │ 1    │ 'Alice Chen'  │ 10            │ 'alice@school.edu' │
│ 3          │ 2    │ 'Bob Jones'   │ 11            │ 'bob@school.edu'   │
│ 4          │ 3    │ 'Clara Smith' │ 10            │ 'clara@school.edu' │
└────────────┴──────┴───────────────┴───────────────┴────────────────────┘
*/
```

To read a specific sheet, pass the sheet name as the second argument:

```sql
select
  xl_at(row, 'A') as student_id,
  xl_at(row, 'B') as assignment_id,
  xl_at(row, 'C') as score,
  xl_at(row, 'D') as submitted
from xl_rows(readfile('tests/students.xlsx'), 'grades')
where row_number > 1
limit 5;
/*
┌────────────┬───────────────┬───────┬───────────────────────┐
│ student_id │ assignment_id │ score │ submitted             │
├────────────┼───────────────┼───────┼───────────────────────┤
│ 1          │ 101           │ 92    │ '2025-03-10 14:30:00' │
│ 1          │ 102           │ 47    │ '2025-03-12 09:15:00' │
│ 1          │ 103           │ 71    │ '2025-03-15 22:00:00' │
│ 2          │ 101           │ 85    │ '2025-03-11 16:45:00' │
│ 2          │ 102           │ 44    │ '2025-03-12 10:00:00' │
└────────────┴───────────────┴───────┴───────────────────────┘
*/
```

You can also use `Sheet!Range` syntax to select a sheet and filter rows in one argument:

```sql
select
  xl_at(row, 'A') as student_id,
  xl_at(row, 'D') as submitted,
  xl_at(row, 'E') as time_spent
from xl_rows(readfile('tests/students.xlsx'), 'grades!A2:E4');
/*
┌────────────┬───────────────────────┬────────────┐
│ student_id │ submitted             │ time_spent │
├────────────┼───────────────────────┼────────────┤
│ 1          │ '2025-03-10 14:30:00' │ '01:45:00' │
│ 1          │ '2025-03-12 09:15:00' │ '00:50:00' │
│ 1          │ '2025-03-15 22:00:00' │ '02:10:00' │
└────────────┴───────────────────────┴────────────┘
*/
```

### `xl_cells(workbook, range)` {#xl_cells}

Returns individual cells in an unpivoted format, filtered to a given range. Each row contains `column_name`, `row_number`, and `value`.

```sql
select * from xl_cells(readfile('tests/students.xlsx'), 'A1:D3');
/*
┌─────────────┬────────────┬────────────────────┐
│ column_name │ row_number │ value              │
├─────────────┼────────────┼────────────────────┤
│ 'A'         │ 1          │ 'id'               │
│ 'B'         │ 1          │ 'name'             │
│ 'C'         │ 1          │ 'grade_level'      │
│ 'D'         │ 1          │ 'email'            │
│ 'A'         │ 2          │ 1                  │
│ 'B'         │ 2          │ 'Alice Chen'       │
│ 'C'         │ 2          │ 10                 │
│ 'D'         │ 2          │ 'alice@school.edu' │
│ 'A'         │ 3          │ 2                  │
│ 'B'         │ 3          │ 'Bob Jones'        │
│ 'C'         │ 3          │ 11                 │
│ 'D'         │ 3          │ 'bob@school.edu'   │
└─────────────┴────────────┴────────────────────┘
*/
```

To read from a specific sheet, pass the sheet name as the third argument:

```sql
select * from xl_cells(readfile('tests/students.xlsx'), 'A1:C3', 'assignments');
/*
┌─────────────┬────────────┬───────────────────────────┐
│ column_name │ row_number │ value                     │
├─────────────┼────────────┼───────────────────────────┤
│ 'A'         │ 1          │ 'id'                      │
│ 'B'         │ 1          │ 'title'                   │
│ 'C'         │ 1          │ 'subject'                 │
│ 'A'         │ 2          │ 101                       │
│ 'B'         │ 2          │ 'Essay: Modern Poetry'    │
│ 'C'         │ 2          │ 'English'                 │
│ 'A'         │ 3          │ 102                       │
│ 'B'         │ 3          │ 'Lab: Chemical Reactions' │
│ 'C'         │ 3          │ 'Science'                 │
└─────────────┴────────────┴───────────────────────────┘
*/
```
