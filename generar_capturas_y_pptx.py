import sys
import os
import time
import tempfile

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

from PIL import ImageGrab, Image, ImageDraw, ImageFont
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN
import tkinter as tk
import customtkinter as ctk

from app.ui import QueVeoHoyApp
from app.database import init_db
from app import repository

init_db()
root = ctk.CTk()
app = QueVeoHoyApp(root)

screenshots = {}

def get_bbox(window):
    window.update_idletasks()
    x = window.winfo_rootx()
    y = window.winfo_rooty()
    w = window.winfo_width()
    h = window.winfo_height()
    return (x, y, x + w, y + h)

# --- State machine for automated captures ---

def step1_hoy():
    print("Capturando 'Hoy'...")
    app.tabview.set("Hoy")
    root.update_idletasks()
    root.after(3000, step1_grab)

def step1_grab():
    screenshots['hoy'] = ImageGrab.grab(bbox=get_bbox(root))
    
    if app.recomendacion_actual:
        print("Recomendación actual encontrada, abriendo modal...")
        app.on_ya_viendo()
        root.after(2000, step2_modal_grab)
    else:
        print("No hay recomendación, capturando modal simulado...")
        screenshots['modal'] = ImageGrab.grab(bbox=get_bbox(root))
        root.after(500, step3_en_progreso)

def step2_modal_grab():
    print("Capturando Modal completo...")
    # Grab the whole app window (root) which shows the modal centered over it
    screenshots['modal'] = ImageGrab.grab(bbox=get_bbox(root))
    # Close the modal
    for widget in root.winfo_children():
        if isinstance(widget, tk.Toplevel) and widget.title() == "Ya la estoy viendo":
            widget.destroy()
            break
    root.after(1000, step3_en_progreso)

def step3_en_progreso():
    print("Capturando 'En progreso'...")
    app.tabview.set("En progreso")
    root.update_idletasks()
    root.after(1000, step3_grab)

def step3_grab():
    screenshots['en_progreso'] = ImageGrab.grab(bbox=get_bbox(root))
    root.after(500, step4_biblioteca)

def step4_biblioteca():
    print("Capturando 'Biblioteca'...")
    app.tabview.set("Biblioteca")
    root.update_idletasks()
    root.after(1500, step4_grab)

def step4_grab():
    screenshots['biblioteca'] = ImageGrab.grab(bbox=get_bbox(root))
    if hasattr(app, 'opt_filtro_tipo'):
        app.opt_filtro_tipo.set("Anime (Todos)")
    if hasattr(app, 'opt_filtro_estado'):
        app.opt_filtro_estado.set("Terminado/Visto")
    app.refresh_pendientes()
    root.update_idletasks()
    root.after(2000, step5_grab)

def step5_grab():
    print("Capturando 'Biblioteca Filtrada'...")
    screenshots['biblioteca_filtrada'] = ImageGrab.grab(bbox=get_bbox(root))
    root.after(500, step6_historial)

def step6_historial():
    print("Capturando 'Historial'...")
    app.tabview.set("Historial")
    root.update_idletasks()
    root.after(1000, step6_grab)

def step6_grab():
    screenshots['historial'] = ImageGrab.grab(bbox=get_bbox(root))
    print("Capturas finalizadas. Procesando imágenes...")
    root.after(500, procesar_e_imprimir)
    
def procesar_e_imprimir():
    root.destroy()
    build_presentation()

# -----------------
# Draw Annotations
# -----------------
def annotate_image(img, boxes, texts):
    if not boxes and not texts:
        return img.copy()
    annotated = img.copy()
    draw = ImageDraw.Draw(annotated)
    try:
        font = ImageFont.truetype("arialbd.ttf", 20)
    except:
        font = ImageFont.load_default()
        
    for box in boxes:
        draw.rectangle(box, outline="#6D28D9", width=6)
        
    for text_info in texts:
        x, y, text = text_info
        try:
            bbox = draw.textbbox((x, y), text, font=font)
            margin = 5
            draw.rectangle([bbox[0]-margin, bbox[1]-margin, bbox[2]+margin, bbox[3]+margin], fill="#6D28D9", outline="#FFFFFF", width=2)
        except:
            pass
        draw.text((x, y), text, fill="#F3F4F6", font=font)
        
    return annotated

# -----------------
# Diagram Generators
# -----------------
def create_slide8_diagram():
    img = Image.new('RGB', (800, 600), color="#121217")
    draw = ImageDraw.Draw(img)
    try: font = ImageFont.truetype("arialbd.ttf", 24)
    except: font = ImageFont.load_default()
    try: font_small = ImageFont.truetype("arial.ttf", 18)
    except: font_small = ImageFont.load_default()
    
    def draw_box(coords, text, text2=""):
        draw.rounded_rectangle(coords, radius=15, fill="#1E1E24", outline="#6D28D9", width=4)
        draw.text((coords[0]+30, coords[1]+25), text, fill="#F3F4F6", font=font)
        if text2:
            draw.text((coords[0]+30, coords[1]+60), text2, fill="#B0B0B8", font=font_small)
            
    draw_box([50, 100, 350, 200], "Jikan API", "(Anime)")
    draw_box([50, 350, 350, 450], "TMDB API", "(Películas/Series)")
    
    # Arrows
    draw.line([(350, 150), (450, 275)], fill="#6D28D9", width=4)
    draw.line([(350, 400), (450, 275)], fill="#6D28D9", width=4)
    
    draw_box([450, 225, 750, 325], "Parser / es_anime", "Normalización")
    
    draw.line([(600, 325), (600, 425)], fill="#6D28D9", width=4)
    
    draw_box([450, 425, 750, 525], "SQLite Local", "Catálogo unificado")
    
    return img

def create_slide9_diagram():
    img = Image.new('RGB', (800, 600), color="#121217")
    draw = ImageDraw.Draw(img)
    try: font = ImageFont.truetype("arialbd.ttf", 24)
    except: font = ImageFont.load_default()
    try: font_small = ImageFont.truetype("arial.ttf", 18)
    except: font_small = ImageFont.load_default()
    
    def draw_box(coords, text, text2=""):
        draw.rounded_rectangle(coords, radius=15, fill="#1E1E24", outline="#6D28D9", width=4)
        draw.text((coords[0]+30, coords[1]+20), text, fill="#F3F4F6", font=font)
        if text2:
            draw.text((coords[0]+30, coords[1]+55), text2, fill="#B0B0B8", font=font_small)
            
    draw_box([200, 50, 600, 150], "UI Event", "Click o Tecla")
    draw.line([(400, 150), (400, 200)], fill="#6D28D9", width=4)
    
    draw_box([200, 200, 600, 300], "Debounce Lock", "Previene race conditions")
    draw.line([(400, 300), (400, 350)], fill="#6D28D9", width=4)
    
    draw_box([200, 350, 600, 450], "Worker Thread", "Request ID Token")
    draw.line([(400, 450), (400, 500)], fill="#6D28D9", width=4)
    
    draw_box([200, 500, 600, 580], "Prefetch Buffer", "Respuesta < 100ms")
    
    return img

def create_slide10_diagram():
    img = Image.new('RGB', (800, 600), color="#121217")
    draw = ImageDraw.Draw(img)
    try: font = ImageFont.truetype("arialbd.ttf", 24)
    except: font = ImageFont.load_default()
    try: font_small = ImageFont.truetype("arial.ttf", 18)
    except: font_small = ImageFont.load_default()
    
    def draw_box(coords, text, text2=""):
        draw.rounded_rectangle(coords, radius=15, fill="#1E1E24", outline="#6D28D9", width=4)
        draw.text((coords[0]+30, coords[1]+20), text, fill="#F3F4F6", font=font)
        if text2:
            draw.text((coords[0]+30, coords[1]+55), text2, fill="#B0B0B8", font=font_small)
            
    draw_box([100, 80, 700, 180], "1. Integración Letterboxd", "Sincronización bidireccional")
    draw.line([(400, 180), (400, 240)], fill="#6D28D9", width=4)
    
    draw_box([100, 240, 700, 340], "2. Calendario Semanal", "Emisiones en tiempo real")
    draw.line([(400, 340), (400, 400)], fill="#6D28D9", width=4)
    
    draw_box([100, 400, 700, 500], "3. Ruleta Interactiva", "Selección aleatoria con animación")
    
    return img

def build_presentation():
    print("Anotando imágenes y generando diagramas...")
    ann_images = {}
    
    if 'hoy' in screenshots:
        w, h = screenshots['hoy'].size
        boxes = [(w*0.1, h*0.82, w*0.9, h*0.92)]
        texts = []
        ann_images['hoy'] = annotate_image(screenshots['hoy'], boxes, texts)
        
    if 'modal' in screenshots:
        # Full app window is captured, no extra annotations needed to see it,
        # but maybe a subtle box around the modal if desired. We'll leave it clean as requested "se aprecie la ventana emergente".
        ann_images['modal'] = annotate_image(screenshots['modal'], [], [])
        
    if 'en_progreso' in screenshots:
        w, h = screenshots['en_progreso'].size
        boxes = [(w*0.05, h*0.25, w*0.95, h*0.45)] # enmarcar solo filas
        texts = []
        ann_images['en_progreso'] = annotate_image(screenshots['en_progreso'], boxes, texts)
        
    if 'biblioteca' in screenshots:
        ann_images['biblioteca'] = annotate_image(screenshots['biblioteca'], [], [])
        
    if 'biblioteca_filtrada' in screenshots:
        ann_images['biblioteca_filtrada'] = annotate_image(screenshots['biblioteca_filtrada'], [], [])
        
    if 'historial' in screenshots:
        ann_images['historial'] = annotate_image(screenshots['historial'], [], [])
        
    # Generate Diagram Images
    ann_images['diagram_api'] = create_slide8_diagram()
    ann_images['diagram_perf'] = create_slide9_diagram()
    ann_images['diagram_roadmap'] = create_slide10_diagram()


    # -----------------
    # Build PPTX
    # -----------------
    print("Generando presentación PPTX...")
    def hex_to_rgb(hex_str):
        hex_str = hex_str.lstrip('#')
        return tuple(int(hex_str[i:i+2], 16) for i in (0, 2, 4))

    BG_COLOR = RGBColor(*hex_to_rgb("#121217"))
    TITLE_COLOR = RGBColor(*hex_to_rgb("#6D28D9"))
    TEXT_COLOR = RGBColor(*hex_to_rgb("#F3F4F6"))
    CONTAINER_COLOR = RGBColor(*hex_to_rgb("#1E1E24"))

    prs = Presentation()
    prs.slide_width = Inches(13.333)
    prs.slide_height = Inches(7.5)

    blank_slide_layout = prs.slide_layouts[6]

    slides_data = [
        {
            "title": "QuéVeoHoy\nGestor Personal de Contenidos",
            "content": [
                "Propósito: Centralizar y recomendar películas, series y anime.",
                "Stack Tecnológico:",
                " • Python & CustomTkinter",
                " • SQLite",
                " • TMDB API",
                " • Jikan API"
            ],
            "is_title_slide": True,
            "image": None
        },
        {
            "title": "2. Pantalla Principal (\"Hoy\")",
            "content": [
                "Tarjeta de recomendación:\nDestaca el contenido ideal para ver hoy.",
                "Póster centrado:\nInterfaz atractiva y visual.",
                "Título ajustado:\nAjuste multilínea automático.",
                "Acciones dinámicas:\nAdaptadas por tipo de contenido."
            ],
            "image": "hoy"
        },
        {
            "title": "3. Gestión de Series",
            "content": [
                "Modal Centrado:\nSelector intuitivo de temporadas y capítulos reales.",
                "Información Precisa:\nDatos extraídos de las APIs.",
                "Flujo Sin Fricción:\nOpción directa 'Ya la terminé'."
            ],
            "image": "modal"
        },
        {
            "title": "4. Pestaña \"En Progreso\"",
            "content": [
                "Seguimiento Episódico:\nControl exacto de lo que estás viendo.",
                "Tabla Simplificada:\nVista clara y sin elementos redundantes.",
                "Reanudación Inmediata:\nEnlace a la vista 'Hoy'."
            ],
            "image": "en_progreso"
        },
        {
            "title": "5. Pestaña \"Biblioteca\"",
            "content": [
                "Catálogo Unificado:",
                " • Pendientes",
                " • Terminados / Vistos",
                " • Abandonados",
                "\nInterfaz consolidada en una sola vista."
            ],
            "image": "biblioteca"
        },
        {
            "title": "6. Doble Filtro Combinado",
            "content": [
                "Selectores Independientes:\nFiltro simultáneo por Tipo y Estado.",
                "Búsqueda Real-time:\nResultados instantáneos.",
                "Fluidez:\nRespuesta inmediata sin bloqueos."
            ],
            "image": "biblioteca_filtrada"
        },
        {
            "title": "7. Pestaña \"Historial\"",
            "content": [
                "Auditoría Cronológica:\nRegistro de interacciones recientes.",
                "Formato Limpio:\nSin horas ni IDs técnicos visibles.",
                "Layout Estable:\nEvita saltos visuales en la zona inferior."
            ],
            "image": "historial"
        },
        {
            "title": "8. Arquitectura Multi-API",
            "content": [
                "Normalización de Anime:",
                " • Unificación de esquemas entre Jikan y TMDB.",
                " • Bandera interna 'es_anime' para enrutamiento.",
                " • Interfaz única para fuentes distintas."
            ],
            "image": "diagram_api"
        },
        {
            "title": "9. Rendimiento y Concurrencia",
            "content": [
                "Buffer de Precarga:\nRespuesta en menos de 100ms.",
                "Debounce:\nOptimización de peticiones de búsqueda.",
                "Request ID Token:\nProtección contra race conditions."
            ],
            "image": "diagram_perf"
        },
        {
            "title": "10. Estado Actual y Roadmap",
            "content": [
                "Hitos Alcanzados:\nSistema base estable, interfaz CustomTkinter, doble API funcional.",
                "\nPróximos Pasos:\nRevisa el esquema a la derecha para conocer los próximos 3 módulos."
            ],
            "image": "diagram_roadmap"
        }
    ]

    for slide_data in slides_data:
        slide = prs.slides.add_slide(blank_slide_layout)
        
        background = slide.background
        fill = background.fill
        fill.solid()
        fill.fore_color.rgb = BG_COLOR
        
        is_title = slide_data.get("is_title_slide", False)
        image_key = slide_data.get("image")
        
        has_image = (image_key is not None) and (image_key in ann_images)
        
        if is_title:
            # Clean layout for Title slide
            title_width = Inches(10)
            title_left = Inches(1.66)
            
            title_box = slide.shapes.add_textbox(title_left, Inches(2), title_width, Inches(1.5))
            title_frame = title_box.text_frame
            title_p = title_frame.paragraphs[0]
            title_p.text = slide_data["title"]
            title_p.font.bold = True
            title_p.font.size = Pt(48)
            title_p.font.color.rgb = TITLE_COLOR
            title_p.alignment = PP_ALIGN.CENTER
            
            content_box = slide.shapes.add_textbox(title_left, Inches(4), title_width, Inches(3))
            content_frame = content_box.text_frame
            content_frame.word_wrap = True
            for i, line in enumerate(slide_data["content"]):
                if i == 0: p = content_frame.paragraphs[0]
                else: p = content_frame.add_paragraph()
                p.text = line
                p.font.size = Pt(24)
                p.font.color.rgb = TEXT_COLOR
                p.alignment = PP_ALIGN.CENTER
        else:
            # 50/50 Layout
            title_width = Inches(5.5)
            content_width = Inches(5.5)
            title_left = Inches(0.5)
            content_left = Inches(0.5)
            image_left = Inches(6.2)
            
            title_box = slide.shapes.add_textbox(title_left, Inches(0.5), title_width, Inches(1))
            title_frame = title_box.text_frame
            title_p = title_frame.paragraphs[0]
            title_p.text = slide_data["title"]
            title_p.font.bold = True
            title_p.font.size = Pt(36)
            title_p.font.color.rgb = TITLE_COLOR
            title_p.alignment = PP_ALIGN.LEFT
            
            content_box = slide.shapes.add_textbox(title_left, Inches(2), content_width, Inches(4.5))
            content_frame = content_box.text_frame
            content_frame.word_wrap = True
            
            for i, line in enumerate(slide_data["content"]):
                if i == 0: p = content_frame.paragraphs[0]
                else: p = content_frame.add_paragraph()
                p.text = line
                # Make subtext slightly smaller
                p.font.size = Pt(22)
                p.font.color.rgb = TEXT_COLOR
                p.alignment = PP_ALIGN.LEFT
                
            if has_image:
                img = ann_images[image_key]
                with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp:
                    img.save(tmp, format="PNG")
                    tmp_path = tmp.name
                    
                slide.shapes.add_picture(tmp_path, image_left, Inches(0.5), height=Inches(6.5))
                os.remove(tmp_path)

    prs.save('presentacion_anotada.pptx')
    print("Archivo presentacion_anotada.pptx generado con éxito.")

if __name__ == "__main__":
    root.after(1000, step1_hoy)
    print("Iniciando aplicación para capturas automáticas...")
    root.mainloop()
