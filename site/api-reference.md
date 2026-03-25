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
  xl_at(row, 'A') as by_name,
  xl_at(row, 0) as by_index
from xl_rows(readfile('tests/file-sample.xlsx'))
limit 1;
-- 'number'
```

You can also use the `->>` operator as a shorthand for `xl_at()` when querying `xl_rows()`:

```sql
select
  row ->> 'A',
  row ->> 'B'
from xl_rows(readfile('tests/file-sample.xlsx'))
limit 1;
-- 'number'
```

## Table Functions

### `xl_sheets(workbook)` {#xl_sheets}

Lists all sheets in a workbook.

```sql
select * from xl_sheets(readfile('tests/sample-abc.xlsx'));
/*
┌───────┬─────────┐
│ name  │ visible │
├───────┼─────────┤
│ 'aaa' │ NULL    │
│ 'bbb' │ NULL    │
│ 'ccc' │ NULL    │
└───────┴─────────┘
*/
```

### `xl_rows(workbook)` {#xl_rows}

Returns one row per row in the worksheet. Each row has a `row_number` column and a `row` pointer column. Use `xl_at()` or `->>` to extract cell values from the row.

By default, reads the first sheet.

```sql
select
  row_number,
  xl_at(row, 'A'),
  xl_at(row, 'B'),
  xl_at(row, 'C')
from xl_rows(readfile('tests/file-sample.xlsx'))
limit 3;
/*
┌────────────┬─────────────────┬─────────────────┬───────────────────────┐
│ row_number │ xl_at(row, 'A') │ xl_at(row, 'B') │ xl_at(row, 'C')       │
├────────────┼─────────────────┼─────────────────┼───────────────────────┤
│ 1          │ 'number'        │ 'decimal'       │ 'date'                │
│ 2          │ 1               │ 1.1             │ '2000-01-01 00:00:00' │
│ 3          │ 2               │ 1.2             │ '2000-01-02 00:00:00' │
└────────────┴─────────────────┴─────────────────┴───────────────────────┘
*/
```

To read a specific sheet, pass the sheet name as the second argument:

```sql
select
  row_number,
  xl_at(row, 'A')
from xl_rows(readfile('tests/sample-abc.xlsx'), 'bbb')
limit 3;
/*
┌────────────┬─────────────────┐
│ row_number │ xl_at(row, 'A') │
├────────────┼─────────────────┤
│ 1          │ 'brian one'     │
│ 2          │ 'brian two'     │
└────────────┴─────────────────┘
*/
```

### `xl_cells(workbook, range)` {#xl_cells}

Returns individual cells in an unpivoted format, filtered to a given range. Each row contains `column_name`, `row_number`, and `value`.

```sql
select * from xl_cells(readfile('tests/file-sample.xlsx'), 'A1:C2');
/*
┌─────────────┬────────────┬───────────────────────┐
│ column_name │ row_number │ value                 │
├─────────────┼────────────┼───────────────────────┤
│ 'A'         │ 1          │ 'number'              │
│ 'B'         │ 1          │ 'decimal'             │
│ 'C'         │ 1          │ 'date'                │
│ 'A'         │ 2          │ 1                     │
│ 'B'         │ 2          │ 1.1                   │
│ 'C'         │ 2          │ '2000-01-01 00:00:00' │
└─────────────┴────────────┴───────────────────────┘
*/
```

To read from a specific sheet, pass the sheet name as the third argument:

```sql
select * from xl_cells(readfile('tests/sample-abc.xlsx'), 'A1:B3', 'bbb');
/*
┌─────────────┬────────────┬─────────────┐
│ column_name │ row_number │ value       │
├─────────────┼────────────┼─────────────┤
│ 'A'         │ 1          │ 'brian one' │
│ 'A'         │ 2          │ 'brian two' │
└─────────────┴────────────┴─────────────┘
*/
```
