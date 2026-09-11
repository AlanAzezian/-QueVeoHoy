import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

start1 = content.find('pub fn get_omitidos_recientes')
print(content[start1:start1+500])
