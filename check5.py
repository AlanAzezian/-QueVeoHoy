import sqlite3
c = sqlite3.connect(r'C:\Users\Alan\.queveohoy.db')
try:
  print(c.execute('SELECT id, titulo FROM catalogo_offline WHERE titulo LIKE \"%Maalaala%\"').fetchall())
except Exception as e: print('Error catalogo_offline:', e)
print('contenido:', c.execute('SELECT id, titulo FROM contenido WHERE titulo LIKE \"%Maalaala%\"').fetchall())
