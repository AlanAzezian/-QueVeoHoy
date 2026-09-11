import os

replacements = {
    'Ã°Å¸Å½Â¬ AcciÃƒÂ³n & Aventura...': '🎬 Acción & Aventura...',
    'Ã°Å¸Â Â¿ Comedia RomÃƒÂ¡ntica...': '🍿 Comedia Romántica...',
    'Ã¢Å¡â€  Anime Shonen...': '⚔️ Anime Shonen...',
    'Ã°Å¸Å¡â‚¬ Ciencia FicciÃƒÂ³n...': '🚀 Ciencia Ficción...',
    'Ã°Å¸Â§Â  Thriller PsicolÃƒÂ³gico...': '🧠 Thriller Psicológico...',
    'Ã°Å¸â€˜Â» Terror & Suspenso...': '👻 Terror & Suspenso...',
    'Ã°Å¸â€¢ÂµÃ¯Â¸Â Ã¢â‚¬Â Ã¢â„¢â€šÃ¯Â¸Â  Crimen & Misterio...': '🕵️‍♂️ Crimen & Misterio...',
    'Ã¢Å“Â¨ FantasÃƒÂ­a Ãƒâ€°pica...': '✨ Fantasía Épica...',
    'Ã°Å¸Å½Â­ Drama Intenso...': '🎭 Drama Intenso...',
    'Ã°Å¸Â¤Â  Western Moderno...': '🤠 Western Moderno...',
    
    'pÃƒÂ³sters dinÃƒÂ¡micos': 'pósters dinámicos',
    'estÃ¡': 'está',
    'PelÃ­cula': 'Película',
    'Ã‚Â¡': '¡',
    'Â¡': '¡',
    'encontrÃ³': 'encontró',
    'IngresÃ¡': 'Ingresá',
    'estÃƒÂ¡ vacÃƒÂ­o': 'está vacío',
    'DINÃ MICO': 'DINÁMICO',
    'mÃƒÂ¡s': 'más',
    'BotÃƒÂ³n': 'Botón',
    'TÃ‰CNICA': 'TÉCNICA',
    'â€¢': '•',
    'DinÃƒÂ¡mico': 'Dinámico',
    'Â·': '·',
    'lÃƒÂ­nea': 'línea',
    'CapÃ­tulo': 'Capítulo',
    'bÃºsqueda': 'búsqueda',
    'tardÃ³': 'tardó',
    'â€”': '—',
    'pÃ³ster': 'póster',
}

files_to_fix = [
    r'c:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\HoyView.tsx',
    r'c:\Proyectos Antigravity\QueVeoHoy\queveohoy-v2\src\components\ContentGrid.tsx'
]

for f in files_to_fix:
    with open(f, 'r', encoding='utf-8') as file:
        content = file.read()
    
    for corrupted, fixed in replacements.items():
        content = content.replace(corrupted, fixed)
        
    with open(f, 'w', encoding='utf-8', newline='') as file:
        file.write(content)
        
    print(f'Fixed {f}')
