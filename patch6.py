import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('use rusqlite::{Connection, Result, params};', 'use rusqlite::{Connection, Result, params};\n\npub const DIAS_EXCLUSION_SKIP: i64 = 10;')

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Added const')
