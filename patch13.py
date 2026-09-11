import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

old_str = '''LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE uc.estado IS NULL {}
         LIMIT 50",'''
         
new_str = '''LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE uc.estado IS NULL {}",'''

if old_str in content:
    content = content.replace(old_str, new_str)
    with codecs.open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("LIMIT 50 removed successfully!")
else:
    print("Could not find the exact string to replace.")
