import sqlite3
c = sqlite3.connect(r'C:\Users\Alan\.queveohoy.db')
print('catalogo:', c.execute('SELECT * FROM catalogo_offline WHERE contenido_id IN (SELECT id FROM contenido WHERE titulo LIKE \"%Maalaala%\")').fetchall())
print('contenido:', c.execute('SELECT id, titulo FROM contenido WHERE titulo LIKE \"%Maalaala%\"').fetchall())
