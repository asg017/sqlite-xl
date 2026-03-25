# /// script
# requires-python = ">=3.10"
# dependencies = ["xlsxwriter"]
# ///

import xlsxwriter
import os

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

# Students sheet
ws = wb.add_worksheet("students")
ws.write_row(0, 0, ["id", "name", "grade_level", "email"])
ws.write_row(1, 0, [1, "Alice Chen", 10, "alice@school.edu"])
ws.write_row(2, 0, [2, "Bob Jones", 11, "bob@school.edu"])
ws.write_row(3, 0, [3, "Clara Smith", 10, "clara@school.edu"])
ws.write_row(4, 0, [4, "David Kim", 12, "david@school.edu"])
ws.write_row(5, 0, [5, "Eva Lopez", 11, "eva@school.edu"])

# Assignments sheet
ws = wb.add_worksheet("assignments")
ws.write_row(0, 0, ["id", "title", "subject", "max_score"])
ws.write_row(1, 0, [101, "Essay: Modern Poetry", "English", 100])
ws.write_row(2, 0, [102, "Lab: Chemical Reactions", "Science", 50])
ws.write_row(3, 0, [103, "Problem Set 5", "Math", 75])
ws.write_row(4, 0, [104, "History Presentation", "History", 100])

# Grades sheet
ws = wb.add_worksheet("grades")
ws.write_row(0, 0, ["student_id", "assignment_id", "score", "submitted"])
ws.write_row(1, 0, [1, 101, 92, "2025-03-10"])
ws.write_row(2, 0, [1, 102, 47, "2025-03-12"])
ws.write_row(3, 0, [1, 103, 71, "2025-03-15"])
ws.write_row(4, 0, [2, 101, 85, "2025-03-11"])
ws.write_row(5, 0, [2, 102, 44, "2025-03-12"])
ws.write_row(6, 0, [2, 103, 68, "2025-03-14"])
ws.write_row(7, 0, [3, 101, 97, "2025-03-10"])
ws.write_row(8, 0, [3, 102, 50, "2025-03-13"])
ws.write_row(9, 0, [3, 104, 88, "2025-03-18"])
ws.write_row(10, 0, [4, 101, 78, "2025-03-11"])
ws.write_row(11, 0, [4, 103, 72, "2025-03-16"])
ws.write_row(12, 0, [4, 104, 95, "2025-03-17"])
ws.write_row(13, 0, [5, 102, 41, "2025-03-12"])
ws.write_row(14, 0, [5, 103, 60, "2025-03-15"])
ws.write_row(15, 0, [5, 104, 82, "2025-03-19"])

wb.close()

print("Generated sample-abc.xlsx and students.xlsx")
