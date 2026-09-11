import re
import io

db_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with io.open(db_path, 'r', encoding='utf8') as f:
    code = f.read()

code = code.replace(
'''pub const DIAS_EXCLUSION_SKIP: i64 = 10;
        .expect("Could not find home directory");''',
'''pub const DIAS_EXCLUSION_SKIP: i64 = 10;
use std::path::PathBuf;
use crate::models::{Contenido, UsuarioContenido, ProgresoSerie, RecomendacionHoy};
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct DbState {
    pub omitted_in_session: Mutex<HashSet<i64>>,
    pub omitted_en_progreso_session: Mutex<HashSet<i64>>,
    pub ultimo_reset_fecha: Mutex<String>,
}

pub fn get_db_path() -> PathBuf {
    let home_dir = dirs::home_dir()
        .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
        .expect("Could not find home directory");'''
)

# Apply FIX 3 for omit_clause in get_recomendacion_de_hoy
old_omit_clause = '''    let omitted = state.omitted_in_session.lock().unwrap();

    // Filter omitted
    let omitted_list = omitted.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    let omit_clause = if omitted.is_empty() {
        "".to_string()
    } else {
        format!("AND c.id NOT IN ({})", omitted_list)
    };'''

new_omit_clause = '''    let omitted_en_progreso = state.omitted_en_progreso_session.lock().unwrap();
    let omitted_en_progreso_list = omitted_en_progreso.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    let omit_progreso_clause = if omitted_en_progreso.is_empty() {
        "".to_string()
    } else {
        format!("AND c.id NOT IN ({})", omitted_en_progreso_list)
    };

    let omitted = state.omitted_in_session.lock().unwrap();
    let omitted_list = omitted.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    let omit_clause = if omitted.is_empty() {
        "".to_string()
    } else {
        format!("AND c.id NOT IN ({})", omitted_list)
    };'''

code = code.replace(old_omit_clause, new_omit_clause)

# Change the where clause in the first statement
old_where = '''         WHERE uc.estado = 'en_progreso' {}",
         omit_clause'''
new_where = '''         WHERE uc.estado = 'en_progreso' {}",
         omit_progreso_clause'''

code = code.replace(old_where, new_where)

# Fix calendar reset of omitted_en_progreso_session
old_reset = '''            let mut omitted = state.omitted_in_session.lock().unwrap();
            if let Ok((nuevos_omitidos, _)) = get_omitidos_recientes(DIAS_EXCLUSION_SKIP) {
                *omitted = nuevos_omitidos;
            }'''
new_reset = '''            let mut omitted = state.omitted_in_session.lock().unwrap();
            if let Ok((nuevos_omitidos, _)) = get_omitidos_recientes(DIAS_EXCLUSION_SKIP) {
                *omitted = nuevos_omitidos;
            }
            let mut omitted_progreso = state.omitted_en_progreso_session.lock().unwrap();
            omitted_progreso.clear();'''

code = code.replace(old_reset, new_reset)

# Make sure omitir_todos_en_progreso was actually updated (from my first replacement attempt)
# Wait, let's just make sure it's correct.
old_omitir = '''    let mut omitted = state.omitted_in_session.lock().unwrap();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    Ok(())
}

pub fn abandonar_multiples'''
new_omitir = '''    let mut omitted = state.omitted_en_progreso_session.lock().unwrap();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    Ok(())
}

pub fn abandonar_multiples'''
if old_omitir in code:
    code = code.replace(old_omitir, new_omitir)

# And if get_estado_contenido is not there, add it:
if 'pub fn get_estado_contenido' not in code:
    code += '''\npub fn get_estado_contenido(id: i64) -> Result<String> {
    let conn = get_connection()?;
    let estado: String = conn.query_row(
        "SELECT estado FROM usuario_contenido WHERE contenido_id = ?1",
        rusqlite::params![id],
        |row| row.get(0)
    ).unwrap_or_else(|_| "".to_string());
    Ok(estado)
}\n'''

with io.open(db_path, 'w', encoding='utf8') as f:
    f.write(code)

lib_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\lib.rs'
with io.open(lib_path, 'r', encoding='utf8') as f:
    lib_code = f.read()

# Fix 1: init DbState
old_init = '''    tauri::Builder::default()
        .manage(DbState {
            omitted_in_session: Mutex::new(omitted),
            ultimo_reset_fecha: Mutex::new(last_date),
        })'''
new_init = '''    tauri::Builder::default()
        .manage(DbState {
            omitted_in_session: Mutex::new(omitted),
            omitted_en_progreso_session: Mutex::new(std::collections::HashSet::new()),
            ultimo_reset_fecha: Mutex::new(last_date),
        })'''
lib_code = lib_code.replace(old_init, new_init)

# Fix 4: mark_as_omitted
old_mark = '''#[tauri::command]
fn mark_as_omitted(id: i64, registrar_en_bitacora: bool, state: State<'_, DbState>) -> Result<(), String> {
    let mut omitted = state.omitted_in_session.lock().unwrap();
    omitted.insert(id);
    let _ = db::add_omitido_sesion(id);
    if registrar_en_bitacora {
        db::registrar_accion(id, "SIGUIENTE").map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}'''
new_mark = '''#[tauri::command]
fn mark_as_omitted(id: i64, registrar_en_bitacora: bool, state: State<'_, DbState>) -> Result<(), String> {
    let estado = db::get_estado_contenido(id).unwrap_or_else(|_| "".to_string());
    
    if estado != "en_progreso" {
        let mut omitted = state.omitted_in_session.lock().unwrap();
        omitted.insert(id);
        let _ = db::add_omitido_sesion(id);
    }
    
    if registrar_en_bitacora {
        db::registrar_accion(id, "SIGUIENTE").map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}'''
lib_code = lib_code.replace(old_mark, new_mark)

with io.open(lib_path, 'w', encoding='utf8') as f:
    f.write(lib_code)

print("ALL FIXES APPLIED")
