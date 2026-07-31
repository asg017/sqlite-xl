# /// script
# requires-python = ">=3.10"
# dependencies = ["xlsxwriter"]
# ///

import xlsxwriter
import os
from datetime import datetime, date, time

DIR = os.path.dirname(os.path.abspath(__file__))

# ── sample-abc.xlsx ──
wb = xlsxwriter.Workbook(os.path.join(DIR, "sample-abc.xlsx"))

ws = wb.add_worksheet("aaa")
ws.write(0, 0, "alex one")
ws.write(1, 0, "alex two")

ws = wb.add_worksheet("bbb")
ws.write(0, 0, "brian one")
ws.write(1, 0, "brian two")

ws = wb.add_worksheet("ccc")
ws.write(0, 0, "craig one")
ws.write(1, 0, "craig two")

wb.close()

# ── students.xlsx ──
wb = xlsxwriter.Workbook(os.path.join(DIR, "students.xlsx"))

date_fmt = wb.add_format({"num_format": "yyyy-mm-dd"})
datetime_fmt = wb.add_format({"num_format": "yyyy-mm-dd hh:mm:ss"})
time_fmt = wb.add_format({"num_format": "hh:mm:ss"})

# Students sheet — now includes enrollment_date and birth_date
ws = wb.add_worksheet("students")
ws.write_row(0, 0, ["id", "name", "grade_level", "email", "enrollment_date", "birth_date"])
students = [
    (1, "Alice Chen",  10, "alice@school.edu", date(2023, 8, 21), date(2009, 4, 15)),
    (2, "Bob Jones",   11, "bob@school.edu",   date(2022, 8, 22), date(2008, 11, 3)),
    (3, "Clara Smith", 10, "clara@school.edu", date(2023, 8, 21), date(2009, 7, 28)),
    (4, "David Kim",   12, "david@school.edu", date(2021, 8, 23), date(2007, 12, 1)),
    (5, "Eva Lopez",   11, "eva@school.edu",   date(2022, 8, 22), date(2008, 6, 19)),
]
for i, (sid, name, grade, email, enroll, birth) in enumerate(students, 1):
    ws.write_number(i, 0, sid)
    ws.write_string(i, 1, name)
    ws.write_number(i, 2, grade)
    ws.write_string(i, 3, email)
    ws.write_datetime(i, 4, enroll, date_fmt)
    ws.write_datetime(i, 5, birth, date_fmt)

# Assignments sheet — now includes due_date
ws = wb.add_worksheet("assignments")
ws.write_row(0, 0, ["id", "title", "subject", "max_score", "due_date"])
assignments = [
    (101, "Essay: Modern Poetry",    "English", 100, datetime(2025, 3, 14, 23, 59, 0)),
    (102, "Lab: Chemical Reactions", "Science",  50, datetime(2025, 3, 15, 15, 30, 0)),
    (103, "Problem Set 5",           "Math",     75, datetime(2025, 3, 18, 23, 59, 0)),
    (104, "History Presentation",    "History",  100, datetime(2025, 3, 20, 9, 0, 0)),
]
for i, (aid, title, subject, max_score, due) in enumerate(assignments, 1):
    ws.write_number(i, 0, aid)
    ws.write_string(i, 1, title)
    ws.write_string(i, 2, subject)
    ws.write_number(i, 3, max_score)
    ws.write_datetime(i, 4, due, datetime_fmt)

# Grades sheet — submitted is now a real datetime, added a time_spent column
ws = wb.add_worksheet("grades")
ws.write_row(0, 0, ["student_id", "assignment_id", "score", "submitted", "time_spent"])
grades = [
    (1, 101, 92, datetime(2025, 3, 10, 14, 30, 0), time(1, 45, 0)),
    (1, 102, 47, datetime(2025, 3, 12, 9, 15, 0),  time(0, 50, 0)),
    (1, 103, 71, datetime(2025, 3, 15, 22, 0, 0),  time(2, 10, 0)),
    (2, 101, 85, datetime(2025, 3, 11, 16, 45, 0), time(1, 30, 0)),
    (2, 102, 44, datetime(2025, 3, 12, 10, 0, 0),  time(0, 40, 0)),
    (2, 103, 68, datetime(2025, 3, 14, 20, 30, 0), time(1, 55, 0)),
    (3, 101, 97, datetime(2025, 3, 10, 8, 0, 0),   time(2, 30, 0)),
    (3, 102, 50, datetime(2025, 3, 13, 14, 0, 0),  time(0, 45, 0)),
    (3, 104, 88, datetime(2025, 3, 18, 11, 30, 0), time(1, 20, 0)),
    (4, 101, 78, datetime(2025, 3, 11, 19, 0, 0),  time(1, 15, 0)),
    (4, 103, 72, datetime(2025, 3, 16, 21, 45, 0), time(2, 0, 0)),
    (4, 104, 95, datetime(2025, 3, 17, 10, 15, 0), time(1, 45, 0)),
    (5, 102, 41, datetime(2025, 3, 12, 11, 30, 0), time(0, 35, 0)),
    (5, 103, 60, datetime(2025, 3, 15, 23, 55, 0), time(1, 40, 0)),
    (5, 104, 82, datetime(2025, 3, 19, 8, 45, 0),  time(1, 10, 0)),
]
for i, (sid, aid, score, submitted, spent) in enumerate(grades, 1):
    ws.write_number(i, 0, sid)
    ws.write_number(i, 1, aid)
    ws.write_number(i, 2, score)
    ws.write_datetime(i, 3, submitted, datetime_fmt)
    ws.write_datetime(i, 4, spent, time_fmt)

wb.close()

print("Generated sample-abc.xlsx and students.xlsx")
