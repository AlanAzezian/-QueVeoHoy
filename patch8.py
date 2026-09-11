import codecs

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\discovery.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('\r\n', '\n')

content = content.replace(
    '        if count >= 100 {\n            break;\n        }',
    '        if count >= 100 {\n            continue;\n        }'
)

content = content.replace('\n', '\r\n')

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Patched discovery.rs!")
