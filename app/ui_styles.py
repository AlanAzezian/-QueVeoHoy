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
    """Retorna (text_color, fg_color) basado en el estado"""
    e = estado.lower()
    
    if e in ('terminada', 'terminado', 'vista', 'visto'):
        return "#10B981", "#064E3B" # Verde
    elif e in ('para_despues', 'para después', 'para despues'):
        return "#F59E0B", "#78350F" # Amarillo/Ámbar
    elif e in ('abandonada', 'abandonado', 'cancelado', 'cancelada'):
        return "#EF4444", "#7F1D1D" # Rojo
    elif e in ('en_progreso', 'en progreso', 'viendo', 'pausada', 'pausado'):
        return "#06B6D4", "#164E63" # Cyan/Azul
    
    # Default (Gris)
    return "#A0A0B0", "#30303A"

def crear_badge_estado(parent, estado: str):
    fg, bg = get_colores_estado(estado)
    # Limpiamos el texto para mostrar: "en_progreso" -> "EN PROGRESO"
    texto_mostrar = estado.replace("_", " ").upper()
    
    badge = ctk.CTkLabel(
        parent,
        text=f" {texto_mostrar} ",
        text_color=fg,
        fg_color=bg,
        corner_radius=12,
        font=("Helvetica", 10, "bold") # Usamos Helvetica por consistencia si no está instalada Plus Jakarta Sans
    )
    return badge
