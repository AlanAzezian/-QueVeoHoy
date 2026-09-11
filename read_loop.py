import codecs
file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src-tauri\src\db.rs'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()
    start = content.find('let mut available_contents = Vec::new();')
    end = content.find('println!("Found {} contents', start)
    print(content[start:end])
