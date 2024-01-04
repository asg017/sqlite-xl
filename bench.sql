
.load target/release/libsqlite_xl sqlite3_xl_init
.timer on

--select count(*)
--from xl_cells(readfile('file.xlsx'), 'A1:Z9999999');
create table t as
select
  xl_at(row, 0) as A,
  xl_at(row, 1) as B,
  xl_at(row, 2) as C,
  xl_at(row, 3) as D,
  xl_at(row, 4) as E
from xl_rows(readfile('file.xlsx'));

select count(*) from t;
select max(A) from t;
