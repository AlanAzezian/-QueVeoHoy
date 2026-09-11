import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

start1 = content.find('get_omitidos_recientes')
print(content[start1:start1+350])

print('---')

start2 = content.find("SELECT date('now', 'localtime', ?1)")
print(content[start2-150:start2+150])
