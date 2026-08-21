import customtkinter as ctk

# Colores Base (sacados de ui.py)
COLOR_BG = "#0D0D12"
COLOR_BG_CARD = "#1A1A1F"
COLOR_PRIMARY = "#7F4FE0"
COLOR_HOVER = "#A78BFA"
COLOR_DARK = "#6D28D9"
COLOR_TEXT = "#FFFFFF"
COLOR_TEXT_SEC = "#B0B0B8"

def get_colores_estado(estado: str):
    """Retorna (text_color, fg_color, icon, formatted_text) basado en el estado"""
    e = estado.lower().replace("_", " ").strip()
    
    # Verde Neón (Terminada / Ya la vi)
    if e in ('terminada', 'terminado', 'vista', 'visto', 'ya la vi'):
        return "#10B981", "#064E3B", "✔", "TERMINADA" if 'terminad' in e else "YA LA VI"
    
    # Ámbar/Amarillo (Para después / Pendiente)
    elif e in ('para despues', 'para después', 'pendiente'):
        return "#F59E0B", "#78350F", "🕒", "PARA DESPUÉS" if 'despu' in e else "PENDIENTE"
        
    # Rojo Carmín (Abandonada / Cancelada)
    elif e in ('abandonada', 'abandonado', 'cancelado', 'cancelada'):
        return "#EF4444", "#7F1D1D", "⊘", "ABANDONADA"
        
    # Cyan Eléctrico (En progreso / Viendo)
    elif e in ('en progreso', 'viendo', 'empezar en progreso'):
        return "#06B6D4", "#164E63", "⟳", "EN PROGRESO" if 'progreso' in e else "VIENDO"
        
    # Azul/Púrpura (Pausada)
    elif e in ('pausada', 'pausado'):
        return "#818CF8", "#1E1B4B", "⏸", "PAUSADA"
        
    # Violeta Neón (Recomendada / Siguiente en Historial)
    elif e in ('recomendada', 'recomendada hoy'):
        return "#C084FC", "#3B0764", "★", "RECOMENDADA"
    elif e in ('siguiente',):
        return "#C084FC", "#3B0764", "➜", "SIGUIENTE"
    
    # Fallbacks para Tipos de contenido u otros (sin icono específico)
    elif e in ('anime', 'anime (serie)', 'anime (película)'):
        return "#F472B6", "#831843", "", e.upper() # Rosa
    elif e in ('película', 'pelicula'):
        return "#38BDF8", "#0C4A6E", "", "PELÍCULA" # Azul claro
    elif e in ('serie',):
        return "#A78BFA", "#2E1065", "", "SERIE" # Morado claro
    
    # Default (Gris)
    return "#94A3B8", "#1E293B", "", e.upper()

def crear_badge_estado(parent, estado: str):
    fg, bg, icon, formatted_text = get_colores_estado(estado)
    
    # Formato final con o sin ícono
    texto_mostrar = f"{icon} {formatted_text}" if icon else formatted_text
    
    badge = ctk.CTkLabel(
        parent,
        text=f" {texto_mostrar} ",
        text_color=fg,
        fg_color=bg,
        corner_radius=12,
        font=("Segoe UI", 11, "bold") # Usamos Segoe UI, consistente con el resto de la app
    )
    return badge
