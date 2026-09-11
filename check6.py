import sqlite3
c = sqlite3.connect(r'C:\Users\Alan\.queveohoy.db')
print('temporadas count:', c.execute('SELECT COUNT(*) FROM tv_temporadas WHERE contenido_id = 1377').fetchone()[0])
print('episodios count:', c.execute('SELECT COUNT(*) FROM tv_episodios WHERE temporada_id IN (SELECT id FROM tv_temporadas WHERE contenido_id = 1377)').fetchone()[0])
