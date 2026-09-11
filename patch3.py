import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\lib.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('\r\n', '\n')

old_str = 'let (omitted, last_date) = db::get_omitidos_hoy().unwrap_or_else(|_| (std::collections::HashSet::new(), "".to_string()));'
new_str = 'let (omitted, last_date) = db::get_omitidos_recientes(db::DIAS_EXCLUSION_SKIP).unwrap_or_else(|_| (std::collections::HashSet::new(), "".to_string()));'

if old_str in content:
    content = content.replace(old_str, new_str)
    print("Replaced in lib.rs")
else:
    print("old_str not found in lib.rs")

content = content.replace('\n', '\r\n')
with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
