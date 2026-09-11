use std::collections::HashSet;
use rusqlite::{params, Connection, Result};

fn main() -> Result<()> {
    let db_path = r"C:\Users\Alan\.queveohoy.db";
    let conn = Connection::open(db_path)?;
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    
    let mut stmt = conn.prepare("SELECT contenido_id FROM omitidos_sesion WHERE fecha = ?1")?;
    let mut rows = stmt.query(params![today_str])?;
    
    let mut omitted = HashSet::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    
    println!("Loaded omitted for {}: {:?}", today_str, omitted);
    Ok(())
}
