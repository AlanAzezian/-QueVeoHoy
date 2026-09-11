import codecs

file_path = r'C:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx'
with codecs.open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

old_gap = '<div className="relative flex items-center justify-center gap-6 lg:gap-10 w-full max-w-6xl mx-auto overflow-hidden [perspective:1200px] min-h-[500px] shrink-0">'
new_gap = '<div className="relative flex items-center justify-center gap-10 lg:gap-16 w-full max-w-6xl mx-auto overflow-hidden [perspective:1200px] min-h-[500px] shrink-0">'

content = content.replace(old_gap, new_gap)

old_mystery = '''<div className="h-[440px] md:h-[480px] aspect-[2/3] bg-neutral-900/80 backdrop-blur-md border-2 border-dashed border-purple-500/40 hover:border-purple-400/80 transition-colors duration-300 rounded-2xl flex flex-col items-center justify-center p-6 shadow-xl">
                  <HelpCircle className="text-purple-400 w-20 h-20 animate-pulse scale-105" style={{ animationDuration: '1000ms' }} />
                </div>'''

new_mystery = '''<div className="h-[440px] md:h-[480px] aspect-[2/3] bg-neutral-900/80 backdrop-blur-md border-2 border-dashed border-purple-500/40 hover:border-purple-400/80 transition-colors duration-300 rounded-2xl flex flex-col items-center justify-center p-6 shadow-xl">
                  <div
                    className="w-[80px] h-[80px] rounded-full flex items-center justify-center"
                    style={{
                      border: '1.5px solid rgba(192, 132, 252, 0.55)',
                      background: 'linear-gradient(135deg, rgba(76, 29, 149, 0.65) 0%, rgba(24, 10, 42, 0.88) 100%)',
                      boxShadow: '0 0 28px 6px rgba(147, 51, 234, 0.28), 0 0 16px 3px rgba(168, 85, 247, 0.42), 0 0 8px 1px rgba(192, 132, 252, 0.55), inset 0 0 10px 2px rgba(168, 85, 247, 0.30)'
                    }}
                  >
                    <HelpCircle 
                      className="text-purple-400 w-20 h-20 animate-pulse scale-105" 
                      style={{ 
                        animationDuration: '1000ms',
                        filter: 'drop-shadow(0 0 6px rgba(255, 255, 255, 0.75)) drop-shadow(0 0 13px rgba(192, 132, 252, 0.65))'
                      }} 
                    />
                  </div>
                </div>'''

content = content.replace(old_mystery, new_mystery)

with codecs.open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)
print("Changes applied!")
