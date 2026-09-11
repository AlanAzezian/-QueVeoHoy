import codecs

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

idx1 = content.find('handleAnteriorLocal : undefined}')
idx2 = content.find('style={{ transform: "rotateY(14deg)', idx1)

if idx1 != -1 and idx2 != -1:
    new_class = '''
                className={
                  showSlotMachine 
                    ? "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-0 pointer-events-none"
                    : "flex flex-col items-center shrink-0 origin-right transition-opacity duration-300 opacity-40 hover:opacity-80 cursor-pointer"
                }
                '''
    content = content[:idx1+32] + new_class + content[idx2:]
    
    idx3 = content.find('<div className="h-[440px] md:h-[480px] aspect-[2/3] rounded-2xl overflow-hidden border border-white/5 shadow-xl bg-neutral-900/50">')
    if idx3 != -1:
        new_inner = '<div className="h-[440px] md:h-[480px] aspect-[2/3] rounded-2xl overflow-hidden border border-white/5 shadow-xl bg-neutral-900/50 hover:scale-[1.02] transition-all duration-300">'
        content = content[:idx3] + new_inner + content[idx3+133:]
    
    with codecs.open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print("Slicing replacement successful!")
else:
    print("Could not find indices", idx1, idx2)
