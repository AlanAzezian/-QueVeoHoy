import sqlite3

db_path = r'C:\Users\Alan\.queveohoy.db'
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

cursor.execute("SELECT COUNT(*) FROM omitidos_sesion WHERE fecha >= date('now', 'localtime', '-10 days')")
in_window = cursor.fetchone()[0]

cursor.execute("SELECT COUNT(*) FROM omitidos_sesion WHERE fecha < date('now', 'localtime', '-10 days')")
out_window = cursor.fetchone()[0]

print(f'En ventana (10 dias): {in_window}')
print(f'Fuera de ventana: {out_window}')
conn.close()
