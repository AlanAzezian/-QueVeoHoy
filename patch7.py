import codecs
import sys

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\lib.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# Normalize line endings to \n to make replacements easier
content = content.replace('\r\n', '\n')

content = content.replace(
    '    omitted.insert(id);\n',
    '    omitted.insert(id);\n    let _ = db::add_omitido_sesion(id);\n'
)

content = content.replace(
    '    tauri::Builder::default()\n        .manage(DbState {\n            omitted_in_session: Mutex::new(HashSet::new()),\n            ultimo_reset_fecha: Mutex::new("".to_string()),\n        })',
    '    let (omitted, last_date) = db::get_omitidos_hoy().unwrap_or_else(|_| (std::collections::HashSet::new(), "".to_string()));\n\n    tauri::Builder::default()\n        .manage(DbState {\n            omitted_in_session: Mutex::new(omitted),\n            ultimo_reset_fecha: Mutex::new(last_date),\n        })'
)

# Convert back to \r\n just in case
content = content.replace('\n', '\r\n')

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch done!")
