import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\discovery.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()
for i, line in enumerate(lines):
    if 'unwrap_or' in line and ('query' in line or 'execute' in line):
        print(f"discovery.rs:{i+1}: {line.strip()}")

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()
for i, line in enumerate(lines):
    if 'unwrap_or' in line and ('query' in line or 'execute' in line):
        print(f"db.rs:{i+1}: {line.strip()}")
