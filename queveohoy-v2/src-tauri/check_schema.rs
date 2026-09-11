use rusqlite::{Connection, Result};

fn main() -> Result<()> {
    let home_dir = dirs::home_dir().unwrap();
    let db_path = home_dir.join(".queveohoy.db");
    let conn = Connection::open(db_path)?;
    
    let mut stmt = conn.prepare(\"SELECT sql FROM sqlite_master WHERE type='table' AND name='historial_recomendaciones'\")?;
    let mut rows = stmt.query([])?;
    
    if let Some(row) = rows.next()? {
        let sql: String = row.get(0)?;
        println!(\"SCHEMA:\\n{}\", sql);
    }
    
    Ok(())
}
