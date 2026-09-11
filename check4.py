import sqlite3
c = sqlite3.connect(r'C:\Users\Alan\.queveohoy.db')
print('schema:', c.execute('PRAGMA table_info(catalogo_offline)').fetchall())
