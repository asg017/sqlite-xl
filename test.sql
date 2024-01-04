.load target/debug/libsqlite_xl sqlite3_xl_init

.mode box
.header on
.timer on

/*select
  name,
  length(data),
  (select json_group_array(name) from xl_sheets(data)) as sheet_names,
  (
    select json_group_array(value)
    from xl_cells(data, 'I3:Z3')
  ) as candidates
from zipfile(readfile('examples/4300_svc_excel.zip'))
limit 10;
*/


select *
from xl_cells(
  readfile('examples/2016_general/PRES_AND_VICE_PRES_11-08-16_by_Precinct_3496-4802.xls'),
  'I3:ZZ3'
) limit 10;

select
  xl_at(row, 0)
from xl_rows(
  readfile('file-sample.xlsx')
);
/*
A1 - Area
B1 - Election
A2 - Contest
C1 - Date
C2 - ?? "VOTE FOR: 3" ???
B2 - candidarte type
I3:3 - candidates


A3 -  LOCATION
B3 -  PRECINCT
C3 -  SERIAL
D3 -  BALLOT GROUP
E3 -  VOTE BY MAIL ONLY
F3 -  REGISTRATION
G3 -  TYPE
H3 -  BALLOTS CAST

I3:3 - candidates
*/
