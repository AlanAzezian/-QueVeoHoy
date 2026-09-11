import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('\r\n', '\n')

old_func = '''pub fn get_omitidos_hoy() -> Result<(std::collections::HashSet<i64>, String)> {
    let conn = get_connection()?;
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    
    let mut stmt = conn.prepare("SELECT contenido_id FROM omitidos_sesion WHERE fecha = ?1")?;
    let mut rows = stmt.query(params![today_str])?;
    
    let mut omitted = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    
    Ok((omitted, today_str))
}'''

new_func = '''pub fn get_omitidos_recientes(dias: i64) -> Result<(std::collections::HashSet<i64>, String)> {
    let conn = get_connection()?;
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    let threshold_date: String = conn.query_row("SELECT date('now', 'localtime', ?1)", params![format!("-{} days", dias)], |row| row.get(0))?;
    
    let mut stmt = conn.prepare("SELECT DISTINCT contenido_id FROM omitidos_sesion WHERE fecha >= ?1")?;
    let mut rows = stmt.query(params![threshold_date])?;
    
    let mut omitted = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    
    Ok((omitted, today_str))
}'''

if old_func in content:
    content = content.replace(old_func, new_func)
    print('Replaced get_omitidos_hoy')
else:
    print('old_func not found')

content = content.replace('\n', '\r\n')
with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
