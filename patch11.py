import codecs
import re

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace wrapper
content = re.sub(
    r'(onClick=\{prevItem && !showSlotMachine \? handleAnteriorLocal : undefined\}\s+)className=\{lex flex-col items-center shrink-0 origin-right transition-opacity duration-300 \$\{\s*showSlotMachine \? \'opacity-0 pointer-events-none\' : \'opacity-40 hover:opacity-80 cursor-pointer\'\s*\}\\}(\s+style=\{\{\s*transform: "rotateY\(14deg\) translateZ\(-40px\) scale\(0.82\)"\s*\}\})',
    r'\g<1>className={\n                  showSlotMachine \n                    ? "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-0 pointer-events-none"\n                    : "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-40 hover:opacity-80 cursor-pointer"\n                }\g<2>',
    content
)

# Replace inner div
content = re.sub(
    r'(<div\s+className="h-\[440px\] md:h-\[480px\] aspect-\[2\/3\] rounded-2xl overflow-hidden border border-white\/5 shadow-xl bg-neutral-900\/50)\"(>)',
    r'\g<1> hover:scale-[1.02] transition-all duration-300"\g<2>',
    content
)

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Regex replace done!")
