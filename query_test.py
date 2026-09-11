import sqlite3

db_path = r'C:\Users\Alan\.queveohoy.db'
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

try:
    cursor.execute("SELECT date(''now'', ''localtime'')")
    print("Success:", cursor.fetchone())
except Exception as e:
    print("Error:", e)

conn.close()
