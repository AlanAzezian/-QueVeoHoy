import sqlite3
c = sqlite3.connect(r'C:\Users\Alan\.queveohoy.db')
print('contenido:', c.execute('SELECT id, titulo, tmdb_id, mal_id, es_anime FROM contenido WHERE id=1377').fetchone())
