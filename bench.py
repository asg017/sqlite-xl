import sqlite3

db = sqlite3.connect(":memory:")
db.enable_load_extension(True)
db.execute("select load_extension('target/release/libsqlite_spreadsheets', 'sqlite3_spreadsheets_init')")
db.enable_load_extension(False)


import time

start = time.perf_counter()
results = db.execute("""
select
  xl_at(row, 0) as A,
  xl_at(row, 1) as B,
  xl_at(row, 2) as C,
  xl_at(row, 3) as D,
  xl_at(row, 4) as E
from xl_rows(?);
""", [open("file.xlsx", "rb").read()])

for row in results:
  pass
elapsed = time.perf_counter() - start
print(elapsed)
from typing import IO, Iterator

import python_calamine
def iter_excel_calamine(file: IO[bytes]):
    workbook = python_calamine.CalamineWorkbook.from_filelike(file)  # type: ignore[arg-type]
    rows = iter(workbook.get_sheet_by_index(0).to_python())
    headers = list(map(str, next(rows)))
    for row in rows:
        yield dict(zip(headers, row))

file =  open('file.xlsx', 'rb')

for row in iter_excel_calamine(file):
  pass
elapsed = time.perf_counter() - start
print(elapsed)
