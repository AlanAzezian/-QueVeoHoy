import codecs
import re

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

# Remove empty lines that were added by mistake (double spacing)
new_lines = []
for line in lines:
    if line.strip() == '' and (len(new_lines) > 0 and new_lines[-1].strip() == ''):
        continue
    new_lines.append(line.rstrip('\r\n'))

content = '\n'.join(new_lines)
# Remove all empty lines for a cleaner start and re-format? No, just remove completely blank lines that are consecutive.
content = re.sub(r'\n\s*\n', '\n\n', content)

# Fix the backticks issue in the template literal
content = content.replace(
    '''className={p-2 rounded-lg text-sm font-medium transition-colors  + "$" + {isCurrent ? "bg-violet-600 text-white border border-violet-400 shadow-[0_0_10px_rgba(139,92,246,0.5)]" : "bg-zinc-800 text-zinc-300 hover:bg-zinc-700 hover:text-white"}}''',
    '''className={p-2 rounded-lg text-sm font-medium transition-colors }'''
)
content = content.replace(
    '''className={px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0  + "$" + {editTemporada === s ? "bg-violet-600 text-white" : "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"}}''',
    '''className={px-4 py-2 rounded-xl text-sm font-semibold transition-colors flex-shrink-0 }'''
)

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Fixed!")
