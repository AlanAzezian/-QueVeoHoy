import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\discovery.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('\r\n', '\n')

old_str1 = 'conn.query_row("SELECT COUNT(*) FROM catalogo_offline co LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= ?1 WHERE uc.estado IS NULL AND os.contenido_id IS NULL", rusqlite::params![format!("-{} days", crate::db::DIAS_EXCLUSION_SKIP)], |row| row.get::<_, i64>(0)).unwrap_or(0)'

new_str1 = 'conn.query_row(&format!("SELECT COUNT(*) FROM catalogo_offline co LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= date(''now'', ''localtime'', ''-{} days'') WHERE uc.estado IS NULL AND os.contenido_id IS NULL", crate::db::DIAS_EXCLUSION_SKIP), [], |row| row.get::<_, i64>(0)).unwrap_or(0)'

count = content.count(old_str1)
if count > 0:
    content = content.replace(old_str1, new_str1)
    print(f"Fixed {count} instances in discovery.rs")
else:
    print("old_str1 not found")

content = content.replace('\n', '\r\n')
with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
