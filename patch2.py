import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()
content = content.replace('\r\n', '\n')

old_str = '''        if *last_date != today_str {
            // Cambi\u00f3 el d\u00eda -> reseteamos estado
            let mut omitted = state.omitted_in_session.lock().unwrap();
            omitted.clear();
            let _ = conn.execute("DELETE FROM omitidos_sesion WHERE fecha != ?", params![today_str.clone()]);
            *last_date = today_str;
        }'''

# Note: because of accents, we should match carefully. I will match with regex.
import re

pattern = re.compile(r'if \*last_date != today_str \{[^\}]+omitted\.clear\(\);\n\s*let _ = conn\.execute\("DELETE FROM omitidos_sesion WHERE fecha != \?", params!\[today_str\.clone\(\)\]\);\n\s*\*last_date = today_str;\n\s*\}')

match = pattern.search(content)
if match:
    new_str = '''if *last_date != today_str {
            let threshold_date: String = conn.query_row("SELECT date('now', 'localtime', ?1)", params![format!("-{} days", DIAS_EXCLUSION_SKIP)], |row| row.get(0)).unwrap_or_else(|_| today_str.clone());
            let _ = conn.execute("DELETE FROM omitidos_sesion WHERE fecha < ?", params![threshold_date]);
            
            let mut omitted = state.omitted_in_session.lock().unwrap();
            if let Ok((nuevos_omitidos, _)) = get_omitidos_recientes(DIAS_EXCLUSION_SKIP) {
                *omitted = nuevos_omitidos;
            }
            
            *last_date = today_str;
        }'''
    content = content[:match.start()] + new_str + content[match.end():]
    print('Replaced daily reset block in db.rs')
else:
    print('Could not find daily reset block in db.rs')

content = content.replace('\n', '\r\n')
with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
