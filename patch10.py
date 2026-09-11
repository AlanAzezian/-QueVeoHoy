import codecs
import sys

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\discovery.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('\r\n', '\n')

old_str = '''        if count >= 100 {
            break;
        }'''
        
new_str = '''        if count >= 100 {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            continue;
        }'''

content = content.replace(old_str, new_str)
content = content.replace('\n', '\r\n')

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Patch applied.")
