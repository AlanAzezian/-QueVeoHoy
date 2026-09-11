import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\discovery.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

bad_str = "date(now, localtime, -{} days)"
good_str = "date(''now'', ''localtime'', ''-{} days'')"

if bad_str in content:
    count = content.count(bad_str)
    content = content.replace(bad_str, good_str)
    print(f"Fixed {count} instances in discovery.rs")
else:
    print("bad_str not found in discovery.rs")

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
