import codecs

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    '''className={px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0 }''',
    '''className={px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0 }'''
)

content = content.replace(
    '''className={p-2 rounded-lg text-sm font-medium transition-colors }''',
    '''className={p-2 rounded-lg text-sm font-medium transition-colors }'''
)

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Fixed className syntax!")
