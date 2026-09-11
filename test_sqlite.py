import sqlite3
try:
    c = sqlite3.connect(":memory:")
    print("Testing ''now'':")
    print(c.execute("SELECT date(''now'')").fetchall())
except Exception as e:
    print("Error:", e)
