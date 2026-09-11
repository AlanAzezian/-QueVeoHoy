import os

replacements = {
    'ÃƒÂ¡': 'á',
    'ÃƒÂ­': 'í',
    'ÃƒÂ³': 'ó',
    'ÃƒÂ©': 'é',
    'ÃƒÂ±': 'ñ',
    'ÃƒÂ': 'í', 
    'Ãƒâ€°': 'É',
    'Ã‚Â': '',
    'Ã¢Å¡â€ ': '⚔️',
    'Ã°Å¸Â Â¿': '🍿',
    'Romǟntica': 'Romántica',
    'estǟ': 'está',
    'vacǟo': 'vacío',
    'DIN?MICO': 'DINÁMICO',
    'mǟs': 'más',
    'Dinǟmico': 'Dinámico',
    
    # Actually let's just use the exact strings from the remaining output
    '\"Ã°Å¸Â Â¿ Comedia RomÃƒÂ¡ntica...\"': '\"🍿 Comedia Romántica...\"',
    '\"Ã¢Å¡â€  Anime Shonen...\"': '\"⚔️ Anime Shonen...\"',
    '\"Ã°Å¸â€¢ÂµÃ¯Â¸Â Ã¢â‚¬Â Ã¢â„¢â€šÃ¯Â¸Â  Crimen & Misterio...\"': '\"🕵️‍♂️ Crimen & Misterio...\"',
    '\"Ã¢Å“Â¨ FantasÃƒÂ­a Ãƒâ€°pica...\"': '\"✨ Fantasía Épica...\"',
    'estÃƒÂ¡ vacÃƒÂ­o': 'está vacío',
    'DINÃ MICO': 'DINÁMICO',
    'mÃƒÂ¡s': 'más',
    'DinÃƒÂ¡mico': 'Dinámico',
    
    # Just generic fix for the triple encoded vowels:
    'ÃƒÂ¡': 'á',
    'ÃƒÂ©': 'é',
    'ÃƒÂ­': 'í',
    'ÃƒÂ³': 'ó',
    'ÃƒÂº': 'ú',
    'ÃƒÂ±': 'ñ',
    'Ãƒâ€°': 'É',
    
    'Ã°Å¸Â Â¿ Comedia Romá': '🍿 Comedia Romá',
    'Ã¢Å¡â€  Anime': '⚔️ Anime',
    'Ã°Å¸â€¢ÂµÃ¯Â¸Â Ã¢â‚¬Â Ã¢â„¢â€šÃ¯Â¸Â  Crimen': '🕵️‍♂️ Crimen',
    'Ã¢Å“Â¨ Fantasí': '✨ Fantasí',
    'está vací': 'está vací',
    'DINÃ MICO': 'DINÁMICO',
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
