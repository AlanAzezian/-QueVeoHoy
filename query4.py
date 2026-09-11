import sqlite3

db_path = r'C:\Users\Alan\.queveohoy.db'
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

# Query 1: The exact count query from discovery.rs (corrected)
query1 = '''
SELECT COUNT(*) FROM catalogo_offline co 
LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id 
LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= date('now', 'localtime', '-10 days') 
WHERE uc.estado IS NULL AND os.contenido_id IS NULL
'''
cursor.execute(query1)
discovery_count = cursor.fetchone()[0]

# Query 2: Total in catalogo_offline
cursor.execute("SELECT COUNT(*) FROM catalogo_offline")
total_count = cursor.fetchone()[0]

print(f"Discovery Count (vigente real): {discovery_count}")
print(f"Total Catalogo Offline: {total_count}")
conn.close()
