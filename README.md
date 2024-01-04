# `sqlite-xl`

A SQLite extension for reading spreadsheets (`.xlsx`, `.xls`, `.ods`). Built with [`calamine`](https://github.com/tafia/calamine) and [`sqlite-loadable-rs`](https://github.com/asg017/sqlite-loadable-rs).

A work in progress! Watch this repo for updates.

Once complete, you'll be able to do things like:

```sql
select
  address,
  value
from xl_cells(
  readfile('My Workbook.xlsx'),
  'A1:Z100'
);

select
  row ->> 'A' as student_name,
  row ->> 'B' as student_age,
  row ->> 'C' as grade
from xl_rows(
  readfile('My Workbook.xlsx'),
  'Students!A2:F'
);

create virtual table temp.my_table using xl0('My Workbook.xlsx');
select A, B, C from temp.my_table;
```

In the meantime, check out [`xlite`](https://github.com/x2bool/xlite) and [`libgsqlite`](https://github.com/0x6b/libgsqlite) for alternative spreadsheet SQLite extension.
