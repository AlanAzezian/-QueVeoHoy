import tkinter as tk
from tkinter import ttk, messagebox
import customtkinter as ctk
import urllib.request
import io
import threading
import os
import queue

try:
    from PIL import Image, ImageTk
    HAS_PILLOW = True
except ImportError:
    HAS_PILLOW = False

from . import recommendation
from .models import (
    TIPO_SERIE, TIPO_PELICULA,
    ESTADO_PARA_DESPUES, ESTADO_EN_PROGRESO, ESTADO_PAUSADA, ESTADO_ABANDONADA,
    ESTADO_TERMINADA, ESTADO_VISTA, ESTADOS_LEGIBLES
)
from . import repository
from .database import init_db
from . import ui_styles

# Configure Appearance and Theme
ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")

COLOR_BG = "#121218"
COLOR_BG_CARD = "#1E1E28"
COLOR_PRIMARY = "#7F4FE0"
COLOR_HOVER = "#A78BFA"
COLOR_DARK = "#6D28D9"
COLOR_TEXT = "#FFFFFF"
COLOR_TEXT_SEC = "#94A3B8"

FONT_MAIN = "Segoe UI"
FONT_TITLE = (FONT_MAIN, 18, "bold")
FONT_CARD = (FONT_MAIN, 14, "bold")
FONT_SUB = (FONT_MAIN, 12)

# --- CONFIGURACIÓN MANUAL DE VENTANA ---
# Editá estos valores a mano para probar el tamaño que mejor se ajuste a tu pantalla.
WINDOW_WIDTH = 800
WINDOW_HEIGHT = 880
TOP_PADDING = 20
BOTTOM_PADDING = 20
Y_OFFSET = -40
# ---------------------------------------

class ModalProgresoSerie(ctk.CTkToplevel):
    def __init__(self, master, contenido, uc_id, on_success):
        super().__init__(master)
        self.title("Ya la estoy viendo")
        self.geometry("400x300")
        self.resizable(False, False)
        
        # Center the modal
        self.update_idletasks()
        x = master.winfo_rootx() + (master.winfo_width() - 400) // 2
        y = master.winfo_rooty() + (master.winfo_height() - 300) // 2
        self.geometry(f"+{x}+{y}")
        
        self.contenido = contenido
        self.uc_id = uc_id
        self.on_success = on_success
        
        self.temporadas_dict = {}
        
        # UI
        self.lbl_status = ctk.CTkLabel(self, text="Cargando temporadas...", font=("Helvetica", 14))
        self.lbl_status.pack(pady=20)
        
        self.opt_temporada = ctk.CTkOptionMenu(self, values=["Cargando..."], command=self.on_temporada_change)
        self.opt_temporada.pack(pady=10)
        self.opt_temporada.set("Cargando...")
        self.opt_temporada.configure(state="disabled")
        
        self.opt_capitulo = ctk.CTkOptionMenu(self, values=["Cargando..."])
        self.opt_capitulo.pack(pady=10)
        self.opt_capitulo.set("Cargando...")
        self.opt_capitulo.configure(state="disabled")
        
        self.btn_confirmar = ctk.CTkButton(self, text="Confirmar", command=self.on_confirmar, state="disabled")
        self.btn_confirmar.pack(pady=20)
        
        self.transient(master)
        self.grab_set()
        
        threading.Thread(target=self.load_metadata, daemon=True).start()
        
    def load_metadata(self):
        try:
            from app.providers.tmdb import TMDBProvider
            from app.providers.jikan import JikanProvider
            import json
            
            meta = repository.obtener_tv_metadata(self.contenido.id)
            if not meta:
                if self.contenido.es_anime and self.contenido.mal_id:
                    provider = JikanProvider()
                    temporadas = provider.obtener_temporadas(self.contenido.mal_id)
                else:
                    provider = TMDBProvider()
                    temporadas = provider.obtener_temporadas(self.contenido.tmdb_id)
                repository.upsert_tv_metadata(self.contenido.id, len(temporadas), json.dumps(temporadas))
                meta = repository.obtener_tv_metadata(self.contenido.id)
                
            if meta:
                self.temporadas_dict = json.loads(meta['temporadas_json'])
                
            self.after(0, self.populate_temporadas)
        except Exception as e:
            self.after(0, lambda: self.lbl_status.configure(text=f"Error: {str(e)}"))
            
    def populate_temporadas(self):
        self.lbl_status.configure(text="Seleccioná en dónde vas:")
        
        if not self.temporadas_dict:
            # Fallback
            self.temporadas_dict = {"1": 1000}
            
        t_keys = sorted(self.temporadas_dict.keys(), key=lambda x: int(x))
        t_values = [f"Temporada {k}" for k in t_keys]
        
        self.opt_temporada.configure(values=t_values, state="normal")
        if t_values:
            self.opt_temporada.set(t_values[0])
            self.on_temporada_change(t_values[0])
            
    def on_temporada_change(self, choice):
        t_num = choice.replace("Temporada ", "")
        max_eps = self.temporadas_dict.get(t_num, 0)
        
        if max_eps == 0:
            max_eps = 1000 # Anime en emisión fallback
            
        ep_values = [f"Capítulo {i}" for i in range(1, max_eps + 1)]
        self.opt_capitulo.configure(values=ep_values, state="normal")
        if ep_values:
            self.opt_capitulo.set(ep_values[0])
            
        self.btn_confirmar.configure(state="normal")
        
    def on_confirmar(self):
        t_str = self.opt_temporada.get().replace("Temporada ", "")
        ep_str = self.opt_capitulo.get().replace("Capítulo ", "")
        
        if not t_str.isdigit() or not ep_str.isdigit():
            return
            
        temp = int(t_str)
        ep = int(ep_str)
        
        self.btn_confirmar.configure(text="Guardando...", state="disabled")
        
        def _process():
            try:
                p, fin = recommendation.importar_progreso_serie(
                    self.uc_id,
                    temp,
                    ep,
                    marcar_anteriores=True
                )
                self.after(0, lambda: self.on_success(p, fin))
                self.after(0, self.destroy)
            except Exception as e:
                self.after(0, lambda: messagebox.showerror("Error", str(e)))
                self.after(0, lambda: self.btn_confirmar.configure(text="Confirmar", state="normal"))
                
        threading.Thread(target=_process, daemon=True).start()

class ModalAjusteProgreso(ctk.CTkToplevel):
    def __init__(self, master, contenido, uc_id, on_success):
        super().__init__(master)
        self.title("Ajustar progreso")
        self.geometry("400x300")
        self.resizable(False, False)
        
        self.update_idletasks()
        x = master.winfo_rootx() + (master.winfo_width() - 400) // 2
        y = master.winfo_rooty() + (master.winfo_height() - 300) // 2
        self.geometry(f"+{x}+{y}")
        
        self.contenido = contenido
        self.uc_id = uc_id
        self.on_success = on_success
        
        self.temporadas_dict = {}
        
        self.lbl_status = ctk.CTkLabel(self, text="Cargando temporadas...", font=("Helvetica", 14))
        self.lbl_status.pack(pady=20)
        
        self.opt_temporada = ctk.CTkOptionMenu(self, values=["Cargando..."], command=self.on_temporada_change)
        self.opt_temporada.pack(pady=10)
        self.opt_temporada.set("Cargando...")
        self.opt_temporada.configure(state="disabled")
        
        self.opt_capitulo = ctk.CTkOptionMenu(self, values=["Cargando..."])
        self.opt_capitulo.pack(pady=10)
        self.opt_capitulo.set("Cargando...")
        self.opt_capitulo.configure(state="disabled")
        
        self.btn_confirmar = ctk.CTkButton(self, text="Confirmar", command=self.on_confirmar, state="disabled")
        self.btn_confirmar.pack(pady=20)
        
        self.transient(master)
        self.grab_set()
        
        threading.Thread(target=self.load_metadata, daemon=True).start()
        
    def load_metadata(self):
        try:
            from app.providers.tmdb import TMDBProvider
            from app.providers.jikan import JikanProvider
            import json
            
            meta = repository.obtener_tv_metadata(self.contenido.id)
            if not meta:
                if self.contenido.es_anime and self.contenido.mal_id:
                    provider = JikanProvider()
                    temporadas = provider.obtener_temporadas(self.contenido.mal_id)
                else:
                    provider = TMDBProvider()
                    temporadas = provider.obtener_temporadas(self.contenido.tmdb_id)
                repository.upsert_tv_metadata(self.contenido.id, len(temporadas), json.dumps(temporadas))
                meta = repository.obtener_tv_metadata(self.contenido.id)
                
            if meta:
                self.temporadas_dict = json.loads(meta['temporadas_json'])
                
            self.after(0, self.populate_temporadas)
        except Exception as e:
            self.after(0, lambda: self.lbl_status.configure(text=f"Error: {str(e)}"))
            
    def populate_temporadas(self):
        self.lbl_status.configure(text="Ajustá la temporada y el capítulo:")
        
        if not self.temporadas_dict:
            self.temporadas_dict = {"1": 1000}
            
        t_keys = sorted(self.temporadas_dict.keys(), key=lambda x: int(x))
        t_values = [f"Temporada {k}" for k in t_keys]
        
        self.opt_temporada.configure(values=t_values, state="normal")
        if t_values:
            self.opt_temporada.set(t_values[0])
            self.on_temporada_change(t_values[0])
            
    def on_temporada_change(self, choice):
        t_num = choice.replace("Temporada ", "")
        max_eps = self.temporadas_dict.get(t_num, 0)
        
        if max_eps == 0:
            max_eps = 1000
            
        ep_values = [f"Capítulo {i}" for i in range(1, max_eps + 1)]
        self.opt_capitulo.configure(values=ep_values, state="normal")
        if ep_values:
            self.opt_capitulo.set(ep_values[0])
            
        self.btn_confirmar.configure(state="normal")
        
    def on_confirmar(self):
        t_str = self.opt_temporada.get().replace("Temporada ", "")
        ep_str = self.opt_capitulo.get().replace("Capítulo ", "")
        
        if not t_str.isdigit() or not ep_str.isdigit():
            return
            
        temp = int(t_str)
        ep = int(ep_str)
        
        self.btn_confirmar.configure(text="Guardando...", state="disabled")
        
        def _process():
            try:
                p, fin = recommendation.corregir_progreso_serie(
                    self.uc_id,
                    temp,
                    ep
                )
                self.after(0, lambda: self.on_success(p, fin))
                self.after(0, self.destroy)
            except Exception as e:
                self.after(0, lambda e=e: messagebox.showerror("Error", str(e), parent=self))
                self.after(0, lambda: self.btn_confirmar.configure(text="Confirmar", state="normal"))
        threading.Thread(target=_process, daemon=True).start()

class ModalCelebracion(ctk.CTkToplevel):
    def __init__(self, master, titulo_serie, on_close):
        super().__init__(master)
        self.title("¡Felicitaciones!")
        self.geometry("450x250")
        self.resizable(False, False)
        
        self.update_idletasks()
        x = master.winfo_rootx() + (master.winfo_width() - 450) // 2
        y = master.winfo_rooty() + (master.winfo_height() - 250) // 2
        self.geometry(f"+{x}+{y}")
        
        self.on_close = on_close
        
        self.lbl_icon = ctk.CTkLabel(self, text="🏆 🎉", font=("Helvetica", 40))
        self.lbl_icon.pack(pady=(20, 10))
        
        self.lbl_titulo = ctk.CTkLabel(self, text=f"¡Felicidades! Completaste {titulo_serie}", font=("Helvetica", 18, "bold"))
        self.lbl_titulo.pack(pady=5)
        
        self.lbl_sub = ctk.CTkLabel(self, text="La serie ha sido movida a tu Biblioteca como 'Terminada'.", font=("Helvetica", 12))
        self.lbl_sub.pack(pady=(0, 15))
        
        self.btn_genial = ctk.CTkButton(self, text="¡Genial!", fg_color="#581C87", hover_color="#4C1D95", command=self._on_btn_click)
        self.btn_genial.pack(pady=10)
        
        self.protocol("WM_DELETE_WINDOW", self._on_btn_click)
        self.transient(master)
        self.grab_set()

    def _on_btn_click(self):
        if self.on_close:
            self.on_close()
        self.destroy()

class QueVeoHoyApp:
    def __init__(self, root):
        self.root = root
        
        self.ui_queue = queue.Queue()
        self._process_ui_queue()
        
        self.root.title("QuéVeoHoy")
        
        base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        icon_path = os.path.join(base_dir, "assets", "icon.ico")
        try:
            self.root.iconbitmap(icon_path)
        except Exception as e:
            print(f"Error cargando el icono: {e}")
        
        screen_width = self.root.winfo_screenwidth()
        screen_height = self.root.winfo_screenheight()
        
        x = (screen_width - WINDOW_WIDTH) // 2
        y = (screen_height - WINDOW_HEIGHT) // 2 + Y_OFFSET
        
        self.root.geometry(f"{WINDOW_WIDTH}x{WINDOW_HEIGHT}+{x}+{y}")
        self.root.resizable(False, False)
        
        self.root.configure(fg_color=COLOR_BG)
        
        # Header / Settings frame
        self.header_frame = ctk.CTkFrame(self.root, fg_color="transparent")
        self.header_frame.pack(fill='x', padx=20, pady=(20, 0))
        
        # Center the title in the header since there are no other buttons
        self.title_lbl = ctk.CTkLabel(self.header_frame, text="QuéVeoHoy", font=("Helvetica", 24, "bold"), text_color=COLOR_PRIMARY)
        self.title_lbl.pack(expand=True, pady=10)
        
        # Custom Navigation Bar
        self.nav_frame = ctk.CTkFrame(self.root, fg_color="transparent")
        self.nav_frame.pack(fill='x', padx=20, pady=(10, 0))
        
        self.nav_capsule = ctk.CTkFrame(self.nav_frame, fg_color="#1E1B2E", corner_radius=20, border_width=1, border_color="#262335")
        self.nav_capsule.pack(anchor="center")
        
        self.content_frame = ctk.CTkFrame(self.root, fg_color="transparent")
        self.content_frame.pack(expand=True, fill='both', padx=20, pady=10)
        self.content_frame.grid_rowconfigure(0, weight=1)
        self.content_frame.grid_columnconfigure(0, weight=1)
        
        self.tabs = {}
        self.nav_buttons = {}
        self.current_tab = None
        
        tab_names = ["Hoy", "En progreso", "Biblioteca", "Historial", "Buscar"]
        for t_name in tab_names:
            btn = ctk.CTkButton(self.nav_capsule, text=t_name, font=("Segoe UI", 14, "bold"),
                                corner_radius=16, fg_color="transparent", text_color="#94A3B8", hover_color="#1F1B2E",
                                border_width=0, command=lambda name=t_name: self.set_tab(name))
            btn.pack(side='left', padx=4, pady=4)
            self.nav_buttons[t_name] = btn
            
            frame = ctk.CTkFrame(self.content_frame, fg_color="transparent")
            frame.grid(row=0, column=0, sticky="nsew")
            self.tabs[t_name] = frame
            
        self.tab_hoy = self.tabs["Hoy"]
        self.tab_en_progreso = self.tabs["En progreso"]
        self.tab_pendientes = self.tabs["Biblioteca"]
        self.tab_historial = self.tabs["Historial"]
        self.tab_buscar = self.tabs["Buscar"]
        
        self.tab_needs_refresh = {
            "Hoy": True,
            "En progreso": True,
            "Biblioteca": True,
            "Historial": True,
            "Buscar": True
        }
        
        self.poster_cache = {}
        self.recomendacion_actual = None
        self._next_rec_cache = None
        self._is_prefetching = False

        self.setup_tab_hoy()
        self.setup_tab_en_progreso()
        self.setup_tab_pendientes()
        self.setup_tab_historial()
        self.setup_tab_buscar()
        
        self.set_tab("Hoy")
        
        self.apply_treeview_style()
        
        if not HAS_PILLOW:
            messagebox.showwarning("Falta Pillow", "La librería Pillow no está instalada. No se mostrarán los pósters.")

        self.root.after(100, self.iniciar_pre_renderizado)
        
    def iniciar_pre_renderizado(self):
        self.refresh_hoy()
        self.root.after(100, self.refresh_en_progreso)
        self.root.after(300, self.refresh_pendientes)
        self.root.after(500, self.refresh_historial)
        
        for tab in self.tab_needs_refresh:
            self.tab_needs_refresh[tab] = False
        
    def _process_ui_queue(self):
        try:
            while True:
                func = self.ui_queue.get_nowait()
                try:
                    func()
                except Exception as e:
                    print(f"Error procesando tarea en UI queue: {e}")
        except queue.Empty:
            pass
        finally:
            self.root.after(50, self._process_ui_queue)
        
    def _cumple_filtro(self, c, filtro_val):
        is_anime = getattr(c, 'es_anime', False) or getattr(c, 'tipo', '') == 'ANIME' or getattr(c, 'mal_id', None) is not None
        
        if filtro_val in ("Todos", "Tipo"):
            return True
        elif filtro_val == "Películas":
            return c.tipo == "MOVIE" and not is_anime
        elif filtro_val == "Series":
            return c.tipo == "TV" and not is_anime
        elif filtro_val == "Anime (Todos)":
            return is_anime
        elif filtro_val == "Anime (Series)":
            return is_anime and c.tipo == "TV"
        elif filtro_val == "Anime (Películas)":
            return is_anime and c.tipo == "MOVIE"
        return True

    def _get_tipo_legible(self, c):
        if not c:
            return "N/A"
        is_anime = getattr(c, 'es_anime', False) or getattr(c, 'tipo', '') == 'ANIME' or getattr(c, 'mal_id', None) is not None
        if is_anime:
            if c.tipo == "MOVIE":
                return "Anime (Película)"
            else:
                return "Anime (Serie)"
        else:
            if c.tipo == "MOVIE":
                return "Película"
            elif c.tipo == "TV":
                return "Serie"
            else:
                return c.tipo

    def show_toast(self, message, parent=None):
        if parent is None:
            parent = self.card_frame
        toast = ctk.CTkLabel(parent, text=message, font=("Helvetica", 14, "bold"),
                             fg_color="#A78BFA", text_color="#0D0D12", corner_radius=10, padx=20, pady=10)
        toast.place(relx=0.5, rely=0.85, anchor="center")
        self.root.after(2000, toast.destroy)

    def apply_treeview_style(self):
        style = ttk.Style(self.root)
        style.theme_use("default")
        style.configure("Treeview", 
                        background=COLOR_BG_CARD,
                        foreground=COLOR_TEXT,
                        fieldbackground=COLOR_BG_CARD,
                        borderwidth=0,
                        font=("Helvetica", 10))
        style.configure("Treeview.Heading",
                        background=COLOR_BG,
                        foreground=COLOR_TEXT_SEC,
                        borderwidth=0,
                        font=("Helvetica", 11, "bold"))
        style.map("Treeview", background=[("selected", COLOR_PRIMARY)])
        style.map("Treeview.Heading", background=[("active", COLOR_DARK)])

    def set_tab(self, tab_name):
        if self.current_tab == tab_name:
            return
            
        if self.current_tab:
            self.nav_buttons[self.current_tab].configure(fg_color="transparent", border_width=0, text_color="#94A3B8")
            
        self.current_tab = tab_name
        self.tabs[tab_name].tkraise()
        self.nav_buttons[tab_name].configure(fg_color="#3B185F", border_width=1, border_color="#7C3AED", text_color="#FFFFFF")

    def refrescar_todo(self):
        self.iniciar_pre_renderizado()
        
    def refrescar_final_serie(self):
        """Refresca las vistas de manera síncrona luego de terminar una serie."""
        self._next_rec_cache = None
        self.recomendacion_actual = None
        self._is_fetching_rec = False
        
        # Limpiar flags de tabs porque vamos a forzar su renderizado atómico
        for tab in self.tab_needs_refresh:
            self.tab_needs_refresh[tab] = False
            
        self.refresh_hoy()
        self.refresh_en_progreso()
        self.refresh_pendientes()
        self.refresh_historial()
        
    def _marcar_tabs_sucias(self):
        self.root.after(100, self.refresh_en_progreso)
        self.root.after(300, self.refresh_pendientes)
        self.root.after(500, self.refresh_historial)

    def setup_tab_hoy(self):
        # Frame central tipo tarjeta
        self.card_frame = ctk.CTkFrame(self.tab_hoy, fg_color=COLOR_BG_CARD, corner_radius=20)
        self.card_frame.pack(expand=True, fill='both', padx=40, pady=(0, 5))

        # State Machine Containers
        self.msg_frame = ctk.CTkFrame(self.card_frame, fg_color="transparent")
        self.lbl_msg_titulo = ctk.CTkLabel(self.msg_frame, text="", font=("Helvetica", 22, "bold"), text_color=COLOR_TEXT)
        self.lbl_msg_titulo.pack(side='top', padx=25, pady=(40, 2))
        self.lbl_msg_sub = ctk.CTkLabel(self.msg_frame, text="", font=("Segoe UI", 15), text_color="#CBD5E1")
        self.lbl_msg_sub.pack(side='top', padx=30, pady=(12, 16))
        self.btn_msg_accion = ctk.CTkButton(self.msg_frame, text="⏭ Intentar de nuevo", command=self.on_siguiente, corner_radius=14, font=("Segoe UI", 12, "bold"))
        self.btn_msg_accion.pack(side='top', pady=20)

        self.content_container = ctk.CTkFrame(self.card_frame, fg_color="transparent")
        
        # Action Buttons Bottom
        self.btn_frame_sec = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.btn_frame_sec.pack(side='bottom', pady=(5, 15))
        
        self.btn_frame_main = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.btn_frame_main.pack(side='bottom', pady=(0, 5))
        
        self.btn_visto = ctk.CTkButton(self.btn_frame_main, text="✔ Ya la vi", command=self.on_marcar_visto, 
                                       width=280, corner_radius=16, fg_color="#059669", hover_color="#10B981", 
                                       text_color="#FFFFFF", border_width=0, font=("Segoe UI", 13, "bold"))
        self.btn_visto.pack(side='top')
        
        self.btn_para_despues = ctk.CTkButton(self.btn_frame_sec, text="🕒 Dejar para después", command=self.on_para_despues, 
                                              corner_radius=14, fg_color="#78350F", hover_color="#451A03", 
                                              text_color="#F59E0B", border_color="#D97706", border_width=2, font=("Segoe UI", 12, "bold"))
                                              
        self.btn_siguiente = ctk.CTkButton(self.btn_frame_sec, text="⏭ Siguiente", command=self.on_siguiente, 
                                           corner_radius=14, fg_color="#4C1D95", hover_color="#3B0764", 
                                           text_color="#DDD6FE", border_color="#DDD6FE", border_width=2, font=("Segoe UI", 12, "bold"))
        
        self.btn_pausar = ctk.CTkButton(self.btn_frame_sec, text="⏸ Pausar", command=self.on_pausar, 
                                        corner_radius=14, fg_color="#78350F", hover_color="#451A03", 
                                        text_color="#F59E0B", border_color="#F59E0B", border_width=2, font=("Segoe UI", 12, "bold"))
                                        
        self.btn_abandonar = ctk.CTkButton(self.btn_frame_sec, text="✕ Abandonar", command=self.on_abandonar, 
                                           corner_radius=14, fg_color="#7F1D1D", hover_color="#450A0A", 
                                           text_color="#FCA5A5", border_color="#FCA5A5", border_width=2, font=("Segoe UI", 12, "bold"))
                                           
        self.btn_ya_viendo = ctk.CTkButton(self.btn_frame_sec, text="▶ Ya la estoy viendo", command=self.on_ya_viendo, 
                                           corner_radius=14, fg_color="#164E63", hover_color="#083344", 
                                           text_color="#06B6D4", border_color="#0891B2", border_width=2, font=("Segoe UI", 12, "bold"))
                                           
        self.btn_ya_termine = ctk.CTkButton(self.btn_frame_sec, text="✔ Ya la terminé", command=self.on_ya_termine, 
                                            corner_radius=14, fg_color="#064E3B", hover_color="#042F2E", 
                                            text_color="#10B981", border_color="#059669", border_width=2, font=("Segoe UI", 12, "bold"))
                                            
        self.btn_ajustar_progreso = ctk.CTkButton(self.btn_frame_sec, text="✏ Ajustar", command=self.on_ajustar_progreso, 
                                                  width=80, corner_radius=14, fg_color="#3B0764", hover_color="#581C87", 
                                                  text_color="#D8B4FE", border_color="#D8B4FE", border_width=2, font=("Segoe UI", 12, "bold"))

        # Poster
        self.shadow_frame = ctk.CTkFrame(self.content_container, fg_color="transparent", 
                                         corner_radius=16, border_width=3, border_color="#A855F7")
        self.shadow_frame.pack(side='top', pady=(10, 2))
        self.lbl_poster = ctk.CTkLabel(self.shadow_frame, text="")
        self.lbl_poster.pack(padx=10, pady=10)
        
        # Info
        self.lbl_titulo = ctk.CTkLabel(self.content_container, text="", font=("Helvetica", 22, "bold"), text_color=COLOR_TEXT, wraplength=480, justify="center")
        self.lbl_titulo.pack(side='top', padx=25, pady=(2, 2))
        
        self.detalle_frame = ctk.CTkFrame(self.content_container, fg_color="transparent")
        self.detalle_frame.pack(side='top', pady=(8, 8))
        self.lbl_badge_tipo = ctk.CTkLabel(self.detalle_frame, text="", fg_color="#3B0764", text_color="#C084FC", corner_radius=8, font=("Segoe UI", 10, "bold"))
        self.lbl_badge_tipo.pack(side="left", padx=(0, 8), ipadx=6, ipady=1)
        self.lbl_anio = ctk.CTkLabel(self.detalle_frame, text="", text_color="#F1F5F9", font=("Segoe UI", 14, "bold"))
        self.lbl_anio.pack(side="left")
        
        # Sinopsis
        self.lbl_sinopsis = ctk.CTkTextbox(self.content_container, font=("Segoe UI", 15, "normal"), text_color="#CBD5E1", 
                                           fg_color="transparent", border_width=0, wrap="word", activate_scrollbars=False, height=240)
        self.lbl_sinopsis.pack(side='top', padx=30, pady=(12, 16), fill='both', expand=True)

        self._set_state_loading()

    def _download_poster(self, url, size=(180, 270)):
        if not HAS_PILLOW or not url:
            return None
            
        cache_key = (url, size)
        if cache_key in self.poster_cache:
            return self.poster_cache[cache_key]
            
        try:
            import hashlib
            import urllib.parse
            import os
            cache_dir = os.path.join(".", "cache", "posters")
            os.makedirs(cache_dir, exist_ok=True)
            
            url_hash = hashlib.md5(url.encode('utf-8')).hexdigest()
            ext = os.path.splitext(urllib.parse.urlparse(url).path)[1]
            if not ext: ext = ".jpg"
            file_path = os.path.join(cache_dir, f"{url_hash}{ext}")
            
            if os.path.exists(file_path):
                im = Image.open(file_path)
            else:
                req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
                raw_data = urllib.request.urlopen(req, timeout=5).read()
                im = Image.open(io.BytesIO(raw_data))
                with open(file_path, "wb") as f:
                    f.write(raw_data)
            
            img = ctk.CTkImage(light_image=im, dark_image=im, size=size)
            self.poster_cache[cache_key] = img
            return img
        except Exception as e:
            print("Error loading image:", e)
            return None

    def _load_poster(self, url):
        img = self._download_poster(url)
        if img:
            self.lbl_poster.configure(image=img, text="")
        else:
            self.lbl_poster.configure(image=None, text="Error/No Image")

    def _set_state_loading(self):
        self.content_container.pack_forget()
        self.msg_frame.pack(expand=True, fill='both')
        self.lbl_msg_titulo.configure(text="Cargando...")
        self.lbl_msg_sub.configure(text="Buscando la mejor recomendación...")
        self.btn_msg_accion.pack_forget()

    def _set_state_error(self, error):
        self.content_container.pack_forget()
        self.msg_frame.pack(expand=True, fill='both')
        self.lbl_msg_titulo.configure(text="Error")
        self.lbl_msg_sub.configure(text=error)
        self.btn_msg_accion.configure(text="⏭ Intentar de nuevo")
        self.btn_msg_accion.pack(side='top', pady=20)

    def _set_state_empty(self):
        self.content_container.pack_forget()
        self.msg_frame.pack(expand=True, fill='both')
        self.lbl_msg_titulo.configure(text="¡No hay nada nuevo para ver!")
        self.lbl_msg_sub.configure(text="No se encontraron recomendaciones. Intentá más tarde.")
        self.btn_msg_accion.configure(text="⏭ Intentar de nuevo")
        self.btn_msg_accion.pack(side='top', pady=20)

    def _set_state_content(self):
        self.msg_frame.pack_forget()
        self.content_container.pack(expand=True, fill='both')

    def _prefetch_next(self):
        if self._is_prefetching or self._next_rec_cache is not None:
            return
        self._is_prefetching = True
        
        def _fetch():
            try:
                curr_id = self.recomendacion_actual.id if self.recomendacion_actual else None
                rec = recommendation.recomendacion_de_hoy(ignorar_id=curr_id, es_prefetch=True)
                if not rec:
                    return
                img = self._download_poster(rec.poster_url)
                
                uc = repository.obtener_usuario_contenido_por_contenido_id(rec.id)
                progreso = None
                if rec.tipo == TIPO_SERIE and uc and uc.estado in (ESTADO_EN_PROGRESO, ESTADO_PAUSADA):
                    progreso = repository.obtener_progreso_serie(uc.id)
                    
                self._next_rec_cache = (rec, img, uc, progreso)
            except Exception as e:
                print("Error prefetching:", e)
            finally:
                self._is_prefetching = False
                
        threading.Thread(target=_fetch, daemon=True).start()

    def refresh_hoy(self, forzar_aleatoria=False):
        if not hasattr(self, '_rec_request_id'):
            self._rec_request_id = 0
            
        self._rec_request_id += 1
        current_id = self._rec_request_id
        
        if hasattr(self, 'btn_visto') and self.btn_visto.winfo_exists():
            self.btn_visto.configure(state="disabled")
        if hasattr(self, 'btn_para_despues') and self.btn_para_despues.winfo_exists():
            self.btn_para_despues.configure(state="disabled")
        if hasattr(self, 'btn_siguiente') and self.btn_siguiente.winfo_exists():
            self.btn_siguiente.configure(state="disabled")
            
        if not self.recomendacion_actual:
            if self._next_rec_cache and not forzar_aleatoria:
                rec, img, uc, progreso = self._next_rec_cache
                self._next_rec_cache = None
                self._apply_rec(rec, img=img, uc=uc, progreso=progreso, current_id=current_id)
                return
                
            self._set_state_loading()
            
            def _fetch():
                try:
                    rec = recommendation.recomendacion_de_hoy(forzar_aleatoria=forzar_aleatoria)
                    if current_id != self._rec_request_id: return
                    
                    img = None
                    uc = None
                    progreso = None
                    
                    if rec:
                        img = self._download_poster(rec.poster_url)
                        uc = repository.obtener_usuario_contenido_por_contenido_id(rec.id)
                        if rec.tipo == TIPO_SERIE and uc and uc.estado in (ESTADO_EN_PROGRESO, ESTADO_PAUSADA):
                            progreso = repository.obtener_progreso_serie(uc.id)
                            
                    if current_id != self._rec_request_id: return
                    
                    self.ui_queue.put(lambda r=rec, i=img, u=uc, p=progreso: self._apply_rec(r, img=i, uc=u, progreso=p, current_id=current_id))
                    self.ui_queue.put(self._prefetch_next)
                except Exception as e:
                    err_msg = str(e)
                    if current_id != self._rec_request_id: return
                    self.ui_queue.put(lambda msg=err_msg: self._apply_rec(None, error=msg, current_id=current_id))
            threading.Thread(target=_fetch, daemon=True).start()
        else:
            if hasattr(self, 'poster_img_actual') and self.poster_img_actual:
                def _fetch_state():
                    uc = repository.obtener_usuario_contenido_por_contenido_id(self.recomendacion_actual.id)
                    progreso = None
                    if self.recomendacion_actual.tipo == TIPO_SERIE and uc and uc.estado in (ESTADO_EN_PROGRESO, ESTADO_PAUSADA):
                        progreso = repository.obtener_progreso_serie(uc.id)
                    if current_id != self._rec_request_id: return
                    self.ui_queue.put(lambda u=uc, p=progreso: self._apply_rec(self.recomendacion_actual, img=self.poster_img_actual, uc=u, progreso=p, current_id=current_id))
                threading.Thread(target=_fetch_state, daemon=True).start()
            else:
                def _fetch_poster_state():
                    img = self._download_poster(self.recomendacion_actual.poster_url)
                    uc = repository.obtener_usuario_contenido_por_contenido_id(self.recomendacion_actual.id)
                    progreso = None
                    if self.recomendacion_actual.tipo == TIPO_SERIE and uc and uc.estado in (ESTADO_EN_PROGRESO, ESTADO_PAUSADA):
                        progreso = repository.obtener_progreso_serie(uc.id)
                    if current_id != self._rec_request_id: return
                    self.ui_queue.put(lambda u=uc, p=progreso: self._apply_rec(self.recomendacion_actual, img=img, uc=u, progreso=p, current_id=current_id))
                    self.ui_queue.put(self._prefetch_next)
                threading.Thread(target=_fetch_poster_state, daemon=True).start()

    def _apply_rec(self, rec, error=None, img=None, uc=None, progreso=None, current_id=None):
        if current_id is not None and current_id != getattr(self, '_rec_request_id', -1):
            return
            
        if error:
            self._set_state_error(error)
            return
            
        self.recomendacion_actual = rec
        self.poster_img_actual = img
        
        if rec:
            def _upsert_historial():
                try:
                    repository.upsert_historial_recomendacion(rec.id, "RECOMENDADA_HOY")
                except Exception:
                    pass
            threading.Thread(target=_upsert_historial, daemon=True).start()
        else:
            self._set_state_empty()
            return

        c = rec
        
        self._set_state_content()
        
        self.btn_visto.configure(state="normal")
        self.btn_para_despues.configure(state="normal")
        self.btn_siguiente.configure(state="normal")
        self.btn_pausar.configure(state="normal")
        self.btn_abandonar.configure(state="normal")
        self.btn_ya_viendo.configure(state="normal")
        self.btn_ya_termine.configure(state="normal")
        self.btn_ajustar_progreso.configure(state="normal")
        
        self.btn_para_despues.pack_forget()
        self.btn_siguiente.pack_forget()
        self.btn_pausar.pack_forget()
        self.btn_abandonar.pack_forget()
        self.btn_ya_viendo.pack_forget()
        self.btn_ya_termine.pack_forget()
        self.btn_ajustar_progreso.pack_forget()
        
        if c.tipo == TIPO_SERIE:
            if uc and uc.estado == ESTADO_EN_PROGRESO:
                self.btn_visto.pack_forget()
                self.btn_ajustar_progreso.pack(side='left', padx=5)
                
                is_last_episode = False
                if meta:
                    import json
                    temporadas_dict = json.loads(meta['temporadas_json'])
                    keys = [int(k) for k in temporadas_dict.keys() if int(k) > 0]
                    if not keys:
                        keys = [int(k) for k in temporadas_dict.keys()]
                    if keys:
                        max_t = max(keys)
                        max_ep = temporadas_dict.get(str(max_t), 0)
                        if progreso and progreso.temporada_actual == max_t and progreso.episodio_actual >= max_ep - 1:
                            is_last_episode = True
                            
                if is_last_episode:
                    self.btn_siguiente.configure(
                        text="🎉 Terminar Serie", 
                        fg_color="#059669", 
                        hover_color="#047857",
                        text_color="#FFFFFF",
                        border_color="#10B981",
                        command=self.on_marcar_visto
                    )
                else:
                    self.btn_siguiente.configure(
                        text="⏭ Siguiente", 
                        fg_color="#4C1D95", 
                        hover_color="#3B0764",
                        text_color="#DDD6FE",
                        border_color="#DDD6FE",
                        command=self.on_siguiente
                    )
                    
                self.btn_siguiente.pack(side='left', padx=5)
                self.btn_abandonar.pack(side='left', padx=5)
            else:
                self.btn_visto.pack(side='top', pady=5)
                self.btn_visto.configure(text="▶ Empezar Serie")
                self.btn_para_despues.pack(side='left', padx=5)
                self.btn_siguiente.configure(
                    text="⏭ Siguiente", fg_color="#4C1D95", hover_color="#3B0764", text_color="#DDD6FE", border_color="#DDD6FE", command=self.on_siguiente
                )
                self.btn_siguiente.pack(side='left', padx=5)
                if not uc or uc.estado == ESTADO_PARA_DESPUES:
                    self.btn_ya_viendo.pack(side='left', padx=5)
                    self.btn_ya_termine.pack(side='left', padx=5)
        else:
            self.btn_visto.pack(side='top', pady=5)
            self.btn_visto.configure(text="✔ Ya la vi")
            self.btn_para_despues.pack(side='left', padx=5)
            self.btn_siguiente.configure(
                text="⏭ Siguiente", fg_color="#4C1D95", hover_color="#3B0764", text_color="#DDD6FE", border_color="#DDD6FE", command=self.on_siguiente
            )
            self.btn_siguiente.pack(side='left', padx=5)

        if img:
            self.lbl_poster.configure(image=img, text="")
        else:
            self.lbl_poster.configure(image=None, text="Sin Imagen" if not c.poster_url else "Cargando...")
            
        self.lbl_titulo.configure(text=c.titulo)
        
        if c.tipo == TIPO_PELICULA:
            tipo_texto = "PELÍCULA"
        elif c.es_anime:
            tipo_texto = "ANIME • SERIE" if c.tipo == TIPO_SERIE else "ANIME • PELÍCULA"
        else:
            tipo_texto = "SERIE"
            
        self.lbl_badge_tipo.configure(text=f" {tipo_texto} ")
        
        anio = ""
        if c.fecha_estreno and len(c.fecha_estreno) >= 4:
            anio = c.fecha_estreno[:4]
            
        if c.tipo == TIPO_SERIE and progreso:
            anio += f" • T{progreso.temporada_actual} C{progreso.episodio_actual}"
            
        self.lbl_anio.configure(text=f"• {anio}" if anio else "")
        
        self.lbl_sinopsis.configure(state="normal")
        self.lbl_sinopsis.delete("1.0", "end")
        self.lbl_sinopsis.insert("1.0", c.sinopsis if c.sinopsis else "Sin sinopsis disponible.")
        self.lbl_sinopsis.tag_config("center", justify="center")
        self.lbl_sinopsis.tag_add("center", "1.0", "end")
        self.lbl_sinopsis.configure(state="disabled")

        self.card_frame.update_idletasks()

    def on_marcar_visto(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            uc = recommendation.agregar_a_biblioteca(c.id)
            
            self.btn_visto.configure(text="Guardando...", state="disabled")
            
            def _process():
                try:
                    if c.tipo == TIPO_SERIE:
                        progreso = repository.obtener_progreso_serie(uc.id)
                        if progreso:
                            p, u, fin = recommendation.avanzar_progreso_serie(uc.id)
                            if fin:
                                self.ui_queue.put(lambda: ModalCelebracion(self.root, c.titulo, self.refrescar_final_serie))
                                self.recomendacion_actual = None
                            else:
                                self.ui_queue.put(lambda p=p: self.show_toast(f"¡Visto! Ahora estás en T{p.temporada_actual} C{p.episodio_actual}"))
                        else:
                            p = recommendation.empezar_serie(uc.id)
                            self.ui_queue.put(lambda: self.show_toast("¡Serie empezada! T1 C1"))
                    else:
                        recommendation.marcar_vista_pelicula(uc.id)
                        self.recomendacion_actual = None
                        self.ui_queue.put(lambda: self.show_toast("¡Película marcada como vista!"))
                    
                    self.ui_queue.put(self.refrescar_todo)
                except Exception as e:
                    self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
                    self.ui_queue.put(self.refrescar_todo)

            threading.Thread(target=_process, daemon=True).start()

    def on_para_despues(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            self.recomendacion_actual = None
            
            def _process():
                uc = recommendation.agregar_a_biblioteca(c.id)
                recommendation.guardar_para_despues(uc.id)
                self.ui_queue.put(self.refrescar_todo)
                self.ui_queue.put(self._prefetch_next)
                
            threading.Thread(target=_process, daemon=True).start()
            self.refresh_hoy()

    def on_siguiente(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            self.recomendacion_actual = None
            
            uc = recommendation.repository.obtener_usuario_contenido_por_contenido_id(c.id)
            was_series_in_progress = c.tipo == "TV" and uc and uc.estado == "en_progreso"
            
            if was_series_in_progress:
                self._next_rec_cache = None
            
            def _process():
                recommendation.registrar_siguiente(c.id)
                self.ui_queue.put(lambda: self.refresh_current_tab(forzar_aleatoria=was_series_in_progress))
                self.ui_queue.put(self._marcar_tabs_sucias)
                self.ui_queue.put(self._prefetch_next)
                
            threading.Thread(target=_process, daemon=True).start()
            
            if was_series_in_progress:
                self.lbl_titulo.configure(text="Cargando...")
                self.lbl_poster.configure(image=None, text="Cargando...")
                self.btn_siguiente.configure(state="disabled")
            else:
                self.refresh_hoy()

    def on_pausar(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            self.recomendacion_actual = None
            
            uc = recommendation.repository.obtener_usuario_contenido_por_contenido_id(c.id)
            was_series_in_progress = c.tipo == "TV" and uc and uc.estado == "en_progreso"
            
            if was_series_in_progress:
                self._next_rec_cache = None
                
            def _process():
                if uc:
                    recommendation.pausar_serie(uc.id)
                self.ui_queue.put(lambda: self.show_toast("Serie pausada."))
                self.ui_queue.put(lambda: self.refresh_current_tab(forzar_aleatoria=was_series_in_progress))
                self.ui_queue.put(self._marcar_tabs_sucias)
                
            threading.Thread(target=_process, daemon=True).start()
            
            if was_series_in_progress:
                self.lbl_titulo.configure(text="Cargando...")
                self.lbl_poster.configure(image=None, text="Cargando...")
                self.btn_pausar.configure(state="disabled")
            else:
                self.refresh_hoy()
                
    def on_abandonar(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            uc = repository.obtener_usuario_contenido_por_contenido_id(c.id)
            if uc:
                recommendation.abandonar(uc.id)
                self.show_toast("Contenido abandonado.")
                self.recomendacion_actual = None
                self.refrescar_todo()
                
    def on_ya_viendo(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            uc = repository.obtener_usuario_contenido_por_contenido_id(c.id)
            if not uc:
                uc = recommendation.agregar_a_biblioteca(c.id)
            
            def _on_success(p, fin):
                if fin:
                    ModalCelebracion(self.root, c.titulo, self.refrescar_final_serie)
                else:
                    self.show_toast(f"Importada. Vas en T{p.temporada_actual} C{p.episodio_actual}")
                    # Remove self.recomendacion_actual = None so it stays on screen
                    self.refrescar_todo()
                
            ModalProgresoSerie(self.root, c, uc.id if hasattr(uc, 'id') else uc.id, _on_success)
            
    def on_ajustar_progreso(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            uc = repository.obtener_usuario_contenido_por_contenido_id(c.id)
            if not uc:
                return
            
            def _on_success(p, fin):
                if fin:
                    ModalCelebracion(self.root, c.titulo, self.refrescar_final_serie)
                else:
                    self.show_toast(f"Progreso ajustado a T{p.temporada_actual} C{p.episodio_actual}")
                    self.refrescar_todo()
                
            ModalAjusteProgreso(self.root, c, uc.id, _on_success)

    def on_ya_termine(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            if not c or not getattr(c, 'id', None):
                return
                
            uc = repository.obtener_usuario_contenido_por_contenido_id(c.id)
            if not uc:
                uc = recommendation.agregar_a_biblioteca(c.id)
                
            self.btn_ya_termine.configure(text="Procesando...", state="disabled")
            
            def _process():
                try:
                    p, _ = recommendation.marcar_serie_terminada(uc.id if hasattr(uc, 'id') else uc.id)
                    
                    def _on_success():
                        self.show_toast(f"Serie terminada en T{p.temporada_actual} C{p.episodio_actual}")
                        self.recomendacion_actual = None
                        self.refrescar_todo()
                        
                    self.ui_queue.put(_on_success)
                except Exception as e:
                    print(f"Error al terminar serie: {e}")
                    self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
                    self.ui_queue.put(self.refrescar_todo)
                
            threading.Thread(target=_process, daemon=True).start()

    # --- PESTAÑA EN PROGRESO ---
    def setup_tab_en_progreso(self):
        lbl = ctk.CTkLabel(self.tab_en_progreso, text="Series en progreso o pausadas", font=("Helvetica", 16, "bold"))
        lbl.pack(pady=10)
        
        # --- NUEVA LISTA CUSTOM CON SCROLL ---
        self.prog_list_container = ctk.CTkFrame(self.tab_en_progreso, fg_color=COLOR_BG_CARD)
        self.prog_list_container.pack(expand=True, fill='both', padx=10, pady=10)
        
        # Header Row
        header_frame = ctk.CTkFrame(self.prog_list_container, fg_color=COLOR_BG, corner_radius=8)
        header_frame.pack(fill='x', padx=5, pady=5)
        
        header_frame.grid_columnconfigure(0, minsize=430, weight=0)
        header_frame.grid_columnconfigure(1, minsize=110, weight=0)
        header_frame.grid_columnconfigure(2, minsize=140, weight=1)
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="SERIE / TÍTULO", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_titulo.grid(row=0, column=0, sticky="w", padx=(20, 10), pady=8)
        
        lbl_h_avance = ctk.CTkLabel(header_frame, text="AVANCE", font=("Segoe UI", 12, "bold"), text_color="#64748B", anchor="center")
        lbl_h_avance.grid(row=0, column=1, sticky="nsew", pady=8)
        
        lbl_h_estado = ctk.CTkLabel(header_frame, text="ESTADO", font=("Segoe UI", 12, "bold"), text_color="#64748B", anchor="center")
        lbl_h_estado.grid(row=0, column=2, sticky="nsew", pady=8)
        
        # Scrollable Area
        self.scroll_progreso = ctk.CTkScrollableFrame(self.prog_list_container, fg_color="transparent",
                                                      scrollbar_button_color="#2A2A38", scrollbar_button_hover_color="#3F3F50")
        self.scroll_progreso.pack(expand=True, fill='both', padx=0, pady=0)
        
        # Variables de selección
        self.selected_prog_uc_id = None
        self.selected_prog_row_frame = None
        
        btn_frame = ctk.CTkFrame(self.tab_en_progreso, fg_color="transparent")
        btn_frame.pack(pady=10)
        
        btn_pausar = ctk.CTkButton(btn_frame, text="⏸ Pausar", command=self.on_progreso_pausar, corner_radius=18, fg_color="#78350F", hover_color="#451A03", text_color="#F59E0B", border_color="#F59E0B", border_width=2, font=("Segoe UI", 12, "bold"))
        btn_reanudar = ctk.CTkButton(btn_frame, text="▶ Reanudar", command=self.on_progreso_reanudar, corner_radius=18, fg_color="#164E63", hover_color="#083344", text_color="#06B6D4", border_color="#06B6D4", border_width=2, font=("Segoe UI", 12, "bold"))
        btn_abandonar = ctk.CTkButton(btn_frame, text="✕ Abandonar", command=self.on_progreso_abandonar, corner_radius=18, fg_color="#7F1D1D", hover_color="#450A0A", text_color="#EF4444", border_color="#EF4444", border_width=2, font=("Segoe UI", 12, "bold"))
        btn_corregir = ctk.CTkButton(btn_frame, text="✏ Corregir Progreso", command=self.on_progreso_corregir, corner_radius=18, fg_color="#3B0764", hover_color="#2E054E", text_color="#C084FC", border_color="#C084FC", border_width=2, font=("Segoe UI", 12, "bold"))
        
        btn_pausar.pack(side='left', padx=5)
        btn_reanudar.pack(side='left', padx=5)
        btn_abandonar.pack(side='left', padx=5)
        btn_corregir.pack(side='left', padx=5)

    def select_progreso_row(self, row_frame, uc_id):
        if self.selected_prog_row_frame:
            self.selected_prog_row_frame.configure(fg_color="transparent")
            
        self.selected_prog_row_frame = row_frame
        self.selected_prog_uc_id = uc_id
        
        # Resaltado sutil
        row_frame.configure(fg_color="#2A2A35")

    def refresh_en_progreso(self):
        self._render_prog_gen = getattr(self, '_render_prog_gen', 0) + 1
        gen = self._render_prog_gen
        
        self.selected_prog_row_frame = None
        self.selected_prog_uc_id = None
        
        items = recommendation.obtener_series_activas()
        
        new_container = ctk.CTkFrame(self.scroll_progreso, fg_color="transparent")
        self._render_progreso_batch(items, 0, new_container, batch_size=15, gen=gen)
        
    def _render_progreso_batch(self, items, start_idx, container, batch_size=15, gen=None):
        if gen is not None and getattr(self, '_render_prog_gen', None) != gen:
            return
            
        end_idx = min(start_idx + batch_size, len(items))
        for idx in range(start_idx, end_idx):
            item = items[idx]
            avance = f"T{item.temporada_actual} C{item.episodio_actual}"
            estado_legible = "Pausada" if item.estado == "pausada" else "En progreso"
            tipo_legible = "Anime (Serie)" if item.es_anime else "Serie"
            
            # Crear la fila
            row_frame = ctk.CTkFrame(container, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(side="top", fill='x', anchor="n", padx=5, pady=0)
            
            row_frame.grid_columnconfigure(0, minsize=430, weight=0)
            row_frame.grid_columnconfigure(1, minsize=110, weight=0)
            row_frame.grid_columnconfigure(2, minsize=140, weight=1)
            
            # --- COLUMNA 0: TÍTULO Y PÓSTER ---
            tit_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            tit_frame.grid(row=0, column=0, sticky="w", padx=(20, 10), pady=10)
            
            # Póster (65x100)
            lbl_poster = ctk.CTkLabel(tit_frame, text="🎬", width=65, height=100, fg_color="#2B1A4A", corner_radius=6, cursor="hand2")
            lbl_poster.pack(side="left", padx=(0, 15))
            
            c = repository.obtener_contenido_por_id(item.contenido_id)
            if c and c.poster_url:
                def _load_img(url, lbl):
                    img = self._download_poster(url, size=(65, 100))
                    if img:
                        self.ui_queue.put(lambda l=lbl, i=img: l.configure(image=i, text="") if l.winfo_exists() else None)
                threading.Thread(target=_load_img, args=(c.poster_url, lbl_poster), daemon=True).start()
            
            # Contenedor para título y badge
            text_container = ctk.CTkFrame(tit_frame, fg_color="transparent", cursor="hand2")
            text_container.pack(side="left", anchor="center")
            
            lbl_tit = ctk.CTkLabel(text_container, text=item.titulo, font=("Segoe UI", 15, "bold"), text_color="#F8FAFC", anchor="w", cursor="hand2")
            lbl_tit.pack(anchor="w")
            
            if item.es_anime:
                anime_badge = ctk.CTkFrame(text_container, fg_color="#3B0764", corner_radius=4, cursor="hand2")
                anime_badge.pack(anchor="w", pady=(4, 0))
                ctk.CTkLabel(anime_badge, text="ANIME", font=("Segoe UI", 9, "bold"), text_color="#C084FC", cursor="hand2").pack(padx=6, pady=2)
            
            # --- COLUMNA 1: AVANCE ---
            avance_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            avance_frame.grid(row=0, column=1, sticky="nsew", pady=10)
            
            # Formatear el texto de avance en cyan claro y ubicarlo alineado
            avance_row = ctk.CTkFrame(avance_frame, fg_color="transparent")
            avance_row.pack(side="top", anchor="center")
            
            lbl_avance = ctk.CTkLabel(avance_row, text=avance, font=("Segoe UI", 14, "bold"), text_color="#38BDF8", anchor="center", cursor="hand2")
            lbl_avance.pack(side="left")
            
            btn_ajustar = ctk.CTkButton(avance_row, text="✏", width=20, height=20, fg_color="transparent", text_color="#C084FC", hover_color="#3B0764", command=lambda u=item.usuario_contenido_id, c_id=item.contenido_id: self._abrir_ajuste_modal(c_id, u))
            btn_ajustar.pack(side="left", padx=(5, 0))
            
            porcentaje = 0.0
            meta = repository.obtener_tv_metadata(item.contenido_id)
            if meta and meta['temporadas_json']:
                import json
                try:
                    t_dict = json.loads(meta['temporadas_json'])
                    max_eps = t_dict.get(str(item.temporada_actual), 0)
                    if max_eps > 0:
                        porcentaje = min(item.episodio_actual / max_eps, 1.0)
                except Exception:
                    pass
            
            prog_bar = ctk.CTkProgressBar(avance_frame, width=90, height=6, corner_radius=3, progress_color="#06B6D4", fg_color="#374151")
            prog_bar.pack(side="top", pady=(4, 0), anchor="center")
            prog_bar.set(porcentaje)
            
            # --- COLUMNA 2: ESTADO ---
            badge_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            badge_frame.grid(row=0, column=2, sticky="nsew", pady=10)
            
            badge = ui_styles.crear_badge_estado(badge_frame, estado_legible)
            badge.pack(anchor="center", pady=12)
            
            # Separador sutil
            if idx < len(items) - 1:
                sep = ctk.CTkFrame(container, height=1, fg_color="#232330")
                sep.pack(side="top", fill='x', anchor="n", padx=15)
            
            # Evento de selección
            def on_click(evt, r=row_frame, u=item.usuario_contenido_id):
                self.select_progreso_row(r, u)
                
            row_frame.bind("<Button-1>", on_click)
            tit_frame.bind("<Button-1>", on_click)
            lbl_poster.bind("<Button-1>", on_click)
            text_container.bind("<Button-1>", on_click)
            lbl_tit.bind("<Button-1>", on_click)
            if item.es_anime:
                anime_badge.bind("<Button-1>", on_click)
            avance_frame.bind("<Button-1>", on_click)
            lbl_avance.bind("<Button-1>", on_click)
            prog_bar.bind("<Button-1>", on_click)
            badge_frame.bind("<Button-1>", on_click)
            badge.bind("<Button-1>", on_click)
            
        if end_idx < len(items):
            self.ui_queue.put(lambda: self._render_progreso_batch(items, end_idx, container, batch_size, gen))
        else:
            for widget in self.scroll_progreso.winfo_children():
                if widget != container:
                    widget.destroy()
            container.pack(side="top", fill="x", anchor="n")

    def _abrir_ajuste_modal(self, contenido_id, uc_id):
        c = repository.obtener_contenido_por_id(contenido_id)
        if not c: return
        
        def _on_success(p, fin):
            if fin:
                ModalCelebracion(self.root, c.titulo, self.refrescar_final_serie)
            else:
                self.show_toast(f"Progreso ajustado a T{p.temporada_actual} C{p.episodio_actual}")
                self.refrescar_todo()
            
        ModalAjusteProgreso(self.root, c, uc_id, _on_success)

    def on_progreso_reanudar(self):
        if not self.selected_prog_uc_id:
            messagebox.showwarning("Aviso", "Seleccione una serie haciendo clic en la fila.")
            return
            
        uc_id = self.selected_prog_uc_id
        try:
            recommendation.reanudar_serie(uc_id)
            self.show_toast("Serie reanudada y lista en Hoy")
            
            # Fetch content to push to Hoy
            uc = recommendation.repository.obtener_usuario_contenido_por_id(uc_id)
            if uc:
                c = recommendation.repository.obtener_contenido_por_id(uc.contenido_id)
                if c:
                    self.recomendacion_actual = c
                    self._next_rec_cache = None
                    self.set_tab("Hoy")
            
            self.refrescar_todo()
        except recommendation.LimiteSeriesEnProgresoAlcanzado as e:
            messagebox.showerror("Límite Alcanzado", str(e))
        except Exception as e:
            messagebox.showerror("Error", str(e))

    def on_progreso_pausar(self):
        if not self.selected_prog_uc_id:
            messagebox.showwarning("Aviso", "Seleccione una serie haciendo clic en la fila.")
            return
        
        uc_id = self.selected_prog_uc_id
        try:
            recommendation.pausar_serie(uc_id)
            self.show_toast("Serie pausada.")
            self.refrescar_todo()
        except Exception as e:
            messagebox.showerror("Error", str(e))

    def on_progreso_abandonar(self):
        if not self.selected_prog_uc_id:
            messagebox.showwarning("Aviso", "Seleccione una serie haciendo clic en la fila.")
            return
            
        uc_id = self.selected_prog_uc_id
        try:
            recommendation.abandonar(uc_id)
            self.show_toast("Serie abandonada.")
            self.refrescar_todo()
        except Exception as e:
            messagebox.showerror("Error", str(e))

    def on_progreso_corregir(self):
        if not self.selected_prog_uc_id:
            messagebox.showwarning("Aviso", "Seleccione una serie haciendo clic en la fila.")
            return
            
        uc_id = self.selected_prog_uc_id
        
        dialog = ctk.CTkInputDialog(text="¿En qué temporada vas?", title="Corregir Progreso")
        temp = dialog.get_input()
        if not temp or not temp.isdigit(): return
        
        dialog = ctk.CTkInputDialog(text="¿En qué capítulo vas?", title="Corregir Progreso")
        ep = dialog.get_input()
        if not ep or not ep.isdigit(): return
        
        def _process():
            try:
                p = recommendation.corregir_progreso_serie(uc_id, int(temp), int(ep))
                self.ui_queue.put(lambda: self.show_toast(f"Progreso corregido: T{p.temporada_actual} C{p.episodio_actual}"))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
            self.ui_queue.put(self.refrescar_todo)
            
        threading.Thread(target=_process, daemon=True).start()

    # --- PESTAÑA PARA DESPUÉS ---
    def setup_tab_pendientes(self):
        # Fila Superior (Controles de Biblioteca):
        controls_frame = ctk.CTkFrame(self.tab_pendientes, fg_color="transparent")
        controls_frame.pack(fill='x', padx=10, pady=(10, 5))
        
        self.filtro_tipo_pendientes = ctk.CTkSegmentedButton(controls_frame, values=["Todas (0)", "Películas (0)", "Series (0)", "Anime (0)"],
                                                             command=lambda _: self.refresh_pendientes(),
                                                             corner_radius=14, fg_color="#1E1B2E", height=32, font=("Segoe UI", 12, "bold"),
                                                             selected_color="#6B21A8", selected_hover_color="#581C87",
                                                             unselected_color="#1E1B2E", unselected_hover_color="#2A2640",
                                                             text_color="#C084FC")
        self.filtro_tipo_pendientes.set("Todas (0)")
        self.filtro_tipo_pendientes.pack(side='left', padx=5)

        self.entry_buscar_pendientes = ctk.CTkEntry(controls_frame, placeholder_text="Buscar en biblioteca...", corner_radius=14, height=32)
        self.entry_buscar_pendientes.pack(side='left', fill='x', expand=True, padx=5)
        self.entry_buscar_pendientes.bind("<KeyRelease>", lambda event: self.refresh_pendientes())
        
        self.filtro_estado_pendientes = ctk.CTkOptionMenu(controls_frame, values=["Todos", "Terminadas / Vistos", "Para después", "Abandonadas"], command=lambda _: self.refresh_pendientes(),
                                                   corner_radius=14, height=32, fg_color="#1E1B2E", button_color="#1E1B2E", button_hover_color="#2A2640",
                                                   dropdown_fg_color="#1E1B2E", dropdown_hover_color="#2A2640",
                                                   dropdown_text_color="#C084FC", text_color="#C084FC")
        self.filtro_estado_pendientes.set("Todos")
        self.filtro_estado_pendientes.pack(side='right', padx=5)
        
        # --- NUEVA LISTA CUSTOM CON SCROLL ---
        self.pend_list_container = ctk.CTkFrame(self.tab_pendientes, fg_color=COLOR_BG_CARD)
        self.pend_list_container.pack(expand=True, fill='both', padx=10, pady=(10, 0))
        
        # Header Row
        header_frame = ctk.CTkFrame(self.pend_list_container, fg_color=COLOR_BG, corner_radius=8)
        header_frame.pack(fill='x', padx=5, pady=5)
        
        header_frame.grid_columnconfigure(0, minsize=320, weight=0)
        header_frame.grid_columnconfigure(1, minsize=140, weight=0)
        header_frame.grid_columnconfigure(2, minsize=160, weight=1)
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="TÍTULO", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_titulo.grid(row=0, column=0, sticky="w", padx=15, pady=5)
        
        lbl_h_tipo = ctk.CTkLabel(header_frame, text="TIPO", font=("Segoe UI", 12, "bold"), text_color="#64748B", anchor="center")
        lbl_h_tipo.grid(row=0, column=1, sticky="nsew", padx=5, pady=5)
        
        lbl_h_estado = ctk.CTkLabel(header_frame, text="ESTADO", font=("Segoe UI", 12, "bold"), text_color="#64748B", anchor="center")
        lbl_h_estado.grid(row=0, column=2, sticky="nsew", padx=5, pady=5)
        
        # Scrollable Area
        self.scroll_pendientes = ctk.CTkScrollableFrame(self.pend_list_container, fg_color="transparent")
        self.scroll_pendientes.pack(expand=True, fill='both', padx=0, pady=0)
        
        # Variables de selección
        self.selected_pend_uc_id = None
        self.selected_pend_c_id = None
        self.selected_pend_tipo = None
        self.selected_pend_row_frame = None
        
        btn_frame = ctk.CTkFrame(self.tab_pendientes, fg_color="transparent")
        btn_frame.pack(pady=10)
        
        btn_reanudar = ctk.CTkButton(btn_frame, text="▶ Reanudar Seleccionado", command=self.on_reanudar,
                                     corner_radius=16, fg_color="#164E63", hover_color="#083344", text_color="#06B6D4", border_color="#06B6D4", border_width=2, font=("Segoe UI", 12, "bold"))
        btn_eliminar_sel = ctk.CTkButton(btn_frame, text="✕ Eliminar Seleccionado", command=self.on_eliminar_seleccionados,
                                         corner_radius=16, fg_color="#7F1D1D", hover_color="#450A0A", text_color="#EF4444", border_color="#EF4444", border_width=2, font=("Segoe UI", 12, "bold"))
        btn_eliminar_todos = ctk.CTkButton(btn_frame, text="🗑 Eliminar Todos", command=self.on_eliminar_todos,
                                           corner_radius=16, fg_color="transparent", hover_color="#450A0A", border_width=2, border_color="#7F1D1D", text_color="#EF4444", font=("Segoe UI", 12, "bold"))
                                           
        btn_reanudar.pack(side='left', padx=5)
        btn_eliminar_sel.pack(side='left', padx=5)
        btn_eliminar_todos.pack(side='left', padx=5)

    def select_pendientes_row(self, row_frame, uc_id, c_id, tipo):
        if self.selected_pend_row_frame:
            self.selected_pend_row_frame.configure(fg_color="transparent")
            
        self.selected_pend_row_frame = row_frame
        self.selected_pend_uc_id = uc_id
        self.selected_pend_c_id = c_id
        self.selected_pend_tipo = tipo
        
        # Resaltado sutil
        row_frame.configure(fg_color="#2A2A35")

    def refresh_pendientes(self):
        self._render_pend_gen = getattr(self, '_render_pend_gen', 0) + 1
        gen = self._render_pend_gen
        
        self.selected_pend_row_frame = None
        self.selected_pend_uc_id = None
        self.selected_pend_c_id = None
        self.selected_pend_tipo = None
            
        items = recommendation.obtener_biblioteca_inactiva()
        
        # Calcular contadores (solo contenido terminado/visto)
        total_mem = 0
        pelis_mem = series_mem = anime_mem = 0
        for item in items:
            c = item.contenido
            e = item.usuario_contenido
            is_terminado = str(e.estado).lower() in ("terminada", "terminado", "vista", "visto", "ya la vi")
            
            if is_terminado:
                total_mem += 1
                is_anime = getattr(c, 'es_anime', False) or getattr(c, 'tipo', '') == 'ANIME' or getattr(c, 'mal_id', None) is not None
                if is_anime:
                    anime_mem += 1
                elif c.tipo.upper() == 'MOVIE':
                    pelis_mem += 1
                elif c.tipo.upper() == 'TV':
                    series_mem += 1
                
        # Actualizar Segmented Button
        if hasattr(self, "filtro_tipo_pendientes"):
            current_raw = self.filtro_tipo_pendientes.get()
            self.filtro_tipo_pendientes.configure(values=[f"Todas ({total_mem})", f"Películas ({pelis_mem})", f"Series ({series_mem})", f"Anime ({anime_mem})"])
            
            if "Películas" in current_raw:
                self.filtro_tipo_pendientes.set(f"Películas ({pelis_mem})")
            elif "Series" in current_raw:
                self.filtro_tipo_pendientes.set(f"Series ({series_mem})")
            elif "Anime" in current_raw:
                self.filtro_tipo_pendientes.set(f"Anime ({anime_mem})")
            else:
                self.filtro_tipo_pendientes.set(f"Todas ({total_mem})")
                
            current_new = self.filtro_tipo_pendientes.get()
            for val, btn in self.filtro_tipo_pendientes._buttons_dict.items():
                if val == current_new:
                    btn.configure(text_color="#FFFFFF")
                else:
                    btn.configure(text_color="#C084FC")
        
        filtro_tipo_raw = getattr(self, "filtro_tipo_pendientes", None)
        filtro_tipo_val_raw = filtro_tipo_raw.get() if filtro_tipo_raw else "Todas"
        
        if "Películas" in filtro_tipo_val_raw: filtro_tipo_val = "Películas"
        elif "Series" in filtro_tipo_val_raw: filtro_tipo_val = "Series"
        elif "Anime" in filtro_tipo_val_raw: filtro_tipo_val = "Anime (Todos)"
        else: filtro_tipo_val = "Todos"
        
        filtro_estado = getattr(self, "filtro_estado_pendientes", None)
        filtro_estado_val = filtro_estado.get() if filtro_estado else "Estado"
        
        entry_busq = getattr(self, "entry_buscar_pendientes", None)
        texto_busq = entry_busq.get().strip().lower() if entry_busq else ""
        
        filtered_items = []
        for item in items:
            c = item.contenido
            e = item.usuario_contenido
            
            if texto_busq and texto_busq not in c.titulo.lower():
                continue
                
            # 1. Aplicar filtro de tipo
            if not self._cumple_filtro(c, filtro_tipo_val):
                continue
                
            # 2. Aplicar filtro de estado
            is_terminado = str(e.estado).lower() in ("terminada", "terminado", "vista", "visto", "ya la vi")
            is_para_despues = str(e.estado).lower() in ("para después", "para despues", "pendiente")
            is_abandonada = str(e.estado).lower() in ("abandonada", "cancelada")
            
            if filtro_estado_val == "Para después" and not is_para_despues:
                continue
            elif filtro_estado_val == "Terminadas / Vistos" and not is_terminado:
                continue
            elif filtro_estado_val == "Abandonadas" and not is_abandonada:
                continue
                
            filtered_items.append(item)
            
        new_container = ctk.CTkFrame(self.scroll_pendientes, fg_color="transparent")
        self._render_pendientes_batch(filtered_items, 0, new_container, batch_size=15, gen=gen)

    def _render_pendientes_batch(self, filtered_items, start_idx, container, batch_size=15, gen=None):
        if gen is not None and getattr(self, '_render_pend_gen', None) != gen:
            return
            
        end_idx = min(start_idx + batch_size, len(filtered_items))
        for idx in range(start_idx, end_idx):
            item = filtered_items[idx]
            c = item.contenido
            e = item.usuario_contenido
                
            tipo_legible = self._get_tipo_legible(c)
            estado_legible = ESTADOS_LEGIBLES.get(e.estado, e.estado)
            
            # Crear la fila
            row_frame = ctk.CTkFrame(container, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(side="top", fill='x', anchor="n", padx=5, pady=2)
            
            row_frame.grid_columnconfigure(0, minsize=320, weight=0)
            row_frame.grid_columnconfigure(1, minsize=140, weight=0)
            row_frame.grid_columnconfigure(2, minsize=160, weight=1)
            
            titulo_trunc = c.titulo[:33] + "..." if len(c.titulo) > 35 else c.titulo
            
            lbl_tit = ctk.CTkLabel(row_frame, text=titulo_trunc, font=("Segoe UI", 13, "normal"), text_color="#F1F5F9", anchor="w", cursor="hand2")
            lbl_tit.grid(row=0, column=0, sticky="w", padx=15, pady=8)
            
            lbl_tipo = ctk.CTkLabel(row_frame, text=tipo_legible, font=("Segoe UI", 13, "normal"), text_color="#94A3B8", anchor="w", cursor="hand2")
            lbl_tipo.grid(row=0, column=1, sticky="w", padx=5, pady=8)
            
            badge_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            badge_frame.grid(row=0, column=2, padx=5, pady=8)
            
            badge = ui_styles.crear_badge_estado(badge_frame, estado_legible)
            badge.pack()
            
            # Evento de selección (Bind a la fila y todos sus hijos)
            def on_click(evt, r=row_frame, u=e.id, cid=c.id, t=c.tipo):
                self.select_pendientes_row(r, u, cid, t)
                
            row_frame.bind("<Button-1>", on_click)
            lbl_tit.bind("<Button-1>", on_click)
            lbl_tipo.bind("<Button-1>", on_click)
            badge_frame.bind("<Button-1>", on_click)
            badge.bind("<Button-1>", on_click)
            
        if end_idx < len(filtered_items):
            self.root.after(10, lambda: self._render_pendientes_batch(filtered_items, end_idx, container, batch_size, gen))
        else:
            for widget in self.scroll_pendientes.winfo_children():
                if widget != container:
                    widget.destroy()
            container.pack(side="top", fill="x", anchor="n")

    def on_reanudar(self):
        if not self.selected_pend_uc_id:
            messagebox.showwarning("Aviso", "Seleccione un elemento de la lista haciendo clic en la fila.")
            return
        
        contenido_id = self.selected_pend_c_id
        uc_id = self.selected_pend_uc_id
        tipo = self.selected_pend_tipo
        
        c = recommendation.repository.obtener_contenido_por_id(contenido_id)
        if not c:
            return
            
        if tipo == "TV":
            try:
                recommendation.reanudar_serie(uc_id)
                self.show_toast("Serie reanudada y lista en Hoy")
                
                self.recomendacion_actual = c
                self._next_rec_cache = None
                self.refrescar_todo()
                self.set_tab("Hoy")
                
            except recommendation.LimiteSeriesEnProgresoAlcanzado:
                messagebox.showerror("Límite Alcanzado", "No podés tener más de 4 series en progreso.")
            except Exception as e:
                messagebox.showerror("Error", str(e))
        else:
            self.show_toast("Película seleccionada para ver hoy")
            self.recomendacion_actual = c
            self._next_rec_cache = None
            self.refrescar_todo()
            self.set_tab("Hoy")

    def on_eliminar_seleccionados(self):
        if not self.selected_pend_uc_id:
            messagebox.showwarning("Aviso", "Seleccione un elemento de la lista haciendo clic en la fila.")
            return
            
        if messagebox.askyesno("Confirmar", "¿Seguro que desea eliminar el elemento seleccionado de la biblioteca?"):
            recommendation.eliminar_de_biblioteca([self.selected_pend_uc_id])
            self.refrescar_todo()
            
    def on_eliminar_todos(self):
        # We need to get all uc_ids from the current filtered items.
        # It's easier to just re-fetch the items matching the current filter, or pull from the UI.
        items_in_list = []
        for child in self.scroll_pendientes.winfo_children():
            # Find the binded uc_id ... this is tricky in Tkinter.
            # Let's just do a clean query or trust the backend for "Eliminar Todos".
            pass
            
        if messagebox.askyesno("Confirmar", "¿Seguro que desea vaciar la biblioteca? Esto puede afectar a los elementos visibles."):
            # We will grab the inactiva items and delete them all
            items = recommendation.obtener_biblioteca_inactiva()
            uc_ids = [item.usuario_contenido.id for item in items]
            recommendation.eliminar_de_biblioteca(uc_ids)
            self.refrescar_todo()

    # --- PESTAÑA HISTORIAL ---
    def setup_tab_historial(self):
        top_frame = ctk.CTkFrame(self.tab_historial, fg_color="transparent")
        top_frame.pack(side='top', fill='x', padx=10, pady=5)
        
        self.filtro_tipo_historial = ctk.CTkSegmentedButton(top_frame, values=["Todas (0)", "Películas (0)", "Series (0)", "Anime (0)"],
                                                            command=lambda _: self.refresh_historial(),
                                                            corner_radius=14, fg_color="#161420", height=32, font=("Segoe UI", 12, "bold"),
                                                            selected_color="#581C87", selected_hover_color="#6B21A8",
                                                            unselected_color="#161420", unselected_hover_color="#2A2640",
                                                            text_color="#C084FC")
        self.filtro_tipo_historial.set("Todas (0)")
        self.filtro_tipo_historial.pack(side='left', padx=5)
        
        self.entry_buscar_historial = ctk.CTkEntry(top_frame, placeholder_text="Buscar en historial...", corner_radius=14, height=32)
        self.entry_buscar_historial.pack(side='left', fill='x', expand=True, padx=5)
        self.entry_buscar_historial.bind("<KeyRelease>", lambda e: self.refresh_historial())
        
        # Container
        self.hist_list_container = ctk.CTkFrame(self.tab_historial, fg_color=COLOR_BG_CARD)
        self.hist_list_container.pack(expand=True, fill='both', padx=10, pady=(5, 0))
        
        # Header Row
        header_frame = ctk.CTkFrame(self.hist_list_container, fg_color=COLOR_BG, corner_radius=8)
        header_frame.pack(fill='x', padx=5, pady=5)
        
        header_frame.grid_columnconfigure(0, minsize=100) # Fecha
        header_frame.grid_columnconfigure(1, weight=1)    # Titulo
        header_frame.grid_columnconfigure(2, minsize=120) # Tipo
        header_frame.grid_columnconfigure(3, minsize=140) # Evento
        
        lbl_h_fecha = ctk.CTkLabel(header_frame, text="FECHA", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_fecha.grid(row=0, column=0, sticky="w", padx=15, pady=5)
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="TÍTULO", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_titulo.grid(row=0, column=1, sticky="w", padx=5, pady=5)
        
        lbl_h_tipo = ctk.CTkLabel(header_frame, text="TIPO", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_tipo.grid(row=0, column=2, sticky="w", padx=5, pady=5)
        
        lbl_h_evento = ctk.CTkLabel(header_frame, text="EVENTO", font=("Segoe UI", 12, "bold"), text_color="#64748B")
        lbl_h_evento.grid(row=0, column=3, sticky="w", padx=5, pady=5)
        
        # Scrollable Area
        self.scroll_historial = ctk.CTkScrollableFrame(self.hist_list_container, fg_color="transparent")
        self.scroll_historial.pack(expand=True, fill='both', padx=0, pady=0)
        
        # Variables de selección
        self.selected_hist_c_id = None
        self.selected_hist_tipo = None
        self.selected_hist_evento = None
        self.selected_hist_row_frame = None
        
        self.historial_btn_frame = ctk.CTkFrame(self.tab_historial, fg_color="transparent")
        # El contenedor empieza oculto hasta que se seleccione una fila
        
        self.btn_hist_accion = ctk.CTkButton(self.historial_btn_frame, text="Seleccionar...", state="disabled")
        self.btn_hist_accion.pack(side='left', padx=5)

    def select_historial_row(self, row_frame, c_id, tipo, evento):
        if self.selected_hist_row_frame:
            self.selected_hist_row_frame.configure(fg_color="transparent")
            
        self.selected_hist_row_frame = row_frame
        self.selected_hist_c_id = c_id
        self.selected_hist_tipo = tipo
        self.selected_hist_evento = evento
        
        # Resaltado sutil
        row_frame.configure(fg_color="#2A2A35")
        self.on_historial_select()

    def refresh_historial(self):
        self._render_hist_gen = getattr(self, '_render_hist_gen', 0) + 1
        gen = self._render_hist_gen
        
        self.selected_hist_row_frame = None
        self.selected_hist_c_id = None
        self.selected_hist_tipo = None
        self.selected_hist_evento = None
        self.historial_btn_frame.pack_forget()
            
        items = repository.obtener_historial_recomendaciones(limit=100)
        search_term = self.entry_buscar_historial.get().lower()
        
        # Calcular contadores
        total_mem = 0
        pelis_mem = series_mem = anime_mem = 0
        
        items_with_content = []
        for h in items:
            c = repository.obtener_contenido_por_id(h.contenido_id)
            if not c:
                continue
                
            items_with_content.append((h, c))
            total_mem += 1
            
            is_anime = getattr(c, 'es_anime', False) or getattr(c, 'tipo', '') == 'ANIME' or getattr(c, 'mal_id', None) is not None
            if is_anime:
                anime_mem += 1
            elif c.tipo.upper() == 'MOVIE':
                pelis_mem += 1
            elif c.tipo.upper() == 'TV':
                series_mem += 1
                
        # Actualizar Segmented Button
        if hasattr(self, "filtro_tipo_historial"):
            current_raw = self.filtro_tipo_historial.get()
            self.filtro_tipo_historial.configure(values=[f"Todas ({total_mem})", f"Películas ({pelis_mem})", f"Series ({series_mem})", f"Anime ({anime_mem})"])
            
            if "Películas" in current_raw:
                self.filtro_tipo_historial.set(f"Películas ({pelis_mem})")
            elif "Series" in current_raw:
                self.filtro_tipo_historial.set(f"Series ({series_mem})")
            elif "Anime" in current_raw:
                self.filtro_tipo_historial.set(f"Anime ({anime_mem})")
            else:
                self.filtro_tipo_historial.set(f"Todas ({total_mem})")
                
            current_new = self.filtro_tipo_historial.get()
            for val, btn in self.filtro_tipo_historial._buttons_dict.items():
                if val == current_new:
                    btn.configure(text_color="#FFFFFF")
                else:
                    btn.configure(text_color="#C084FC")
                    
        filtro_tipo_raw = getattr(self, "filtro_tipo_historial", None)
        filtro_tipo_val_raw = filtro_tipo_raw.get() if filtro_tipo_raw else "Todas"
        
        if "Películas" in filtro_tipo_val_raw: filtro_tipo_val = "Películas"
        elif "Series" in filtro_tipo_val_raw: filtro_tipo_val = "Series"
        elif "Anime" in filtro_tipo_val_raw: filtro_tipo_val = "Anime (Todos)"
        else: filtro_tipo_val = "Todos"
        
        evento_map = {
            "PARA_DESPUES": "Para después",
            "YA_LA_VI": "Ya la vi",
            "SIGUIENTE": "Siguiente",
            "RECOMENDADA_HOY": "Recomendada",
            "EMPEZAR_EN_PROGRESO": "En progreso",
            "TERMINADA": "Terminada"
        }
        
        filtered_items = []
        for h, c in items_with_content:
            if not self._cumple_filtro(c, filtro_tipo_val):
                continue
                
            titulo = c.titulo if c else "Desconocido"
            if search_term and search_term not in titulo.lower():
                continue
                
            filtered_items.append((h, c, titulo))
            
        new_container = ctk.CTkFrame(self.scroll_historial, fg_color="transparent")
        self._render_historial_batch(filtered_items, 0, evento_map, new_container, batch_size=15, gen=gen)

    def _render_historial_batch(self, filtered_items, start_idx, evento_map, container, batch_size=15, gen=None):
        if gen is not None and getattr(self, '_render_hist_gen', None) != gen:
            return
            
        end_idx = min(start_idx + batch_size, len(filtered_items))
        for idx in range(start_idx, end_idx):
            h, c, titulo = filtered_items[idx]
            
            fecha_str = str(h.fecha)[:10] if h.fecha else ""
            tipo = self._get_tipo_legible(c)
            
            raw_evento = h.accion if h.accion else ""
            evento = evento_map.get(raw_evento, raw_evento.replace("_", " ").capitalize())
            
            # Crear la fila
            row_frame = ctk.CTkFrame(container, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(side="top", fill='x', anchor="n", padx=5, pady=2)
            
            row_frame.grid_columnconfigure(0, minsize=100)
            row_frame.grid_columnconfigure(1, weight=1)
            row_frame.grid_columnconfigure(2, minsize=120)
            row_frame.grid_columnconfigure(3, minsize=140)
            
            lbl_fecha = ctk.CTkLabel(row_frame, text=fecha_str, font=("Segoe UI", 12, "normal"), text_color="#64748B", anchor="w", cursor="hand2")
            lbl_fecha.grid(row=0, column=0, sticky="w", padx=15, pady=8)
            
            lbl_tit = ctk.CTkLabel(row_frame, text=titulo, font=("Segoe UI", 13, "normal"), text_color="#F1F5F9", anchor="w", cursor="hand2")
            lbl_tit.grid(row=0, column=1, sticky="w", padx=5, pady=8)
            
            lbl_tipo = ctk.CTkLabel(row_frame, text=tipo, font=("Segoe UI", 13, "normal"), text_color="#94A3B8", anchor="w", cursor="hand2")
            lbl_tipo.grid(row=0, column=2, sticky="w", padx=5, pady=8)
            
            badge_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            badge_frame.grid(row=0, column=3, sticky="w", padx=5, pady=8)
            
            badge = ui_styles.crear_badge_estado(badge_frame, evento)
            badge.pack()
            
            # Evento de selección
            def on_click(evt, r=row_frame, cid=h.contenido_id, t=tipo, ev=evento):
                self.select_historial_row(r, cid, t, ev)
                
            row_frame.bind("<Button-1>", on_click)
            lbl_fecha.bind("<Button-1>", on_click)
            lbl_tit.bind("<Button-1>", on_click)
            lbl_tipo.bind("<Button-1>", on_click)
            badge_frame.bind("<Button-1>", on_click)
            badge.bind("<Button-1>", on_click)
            
        if end_idx < len(filtered_items):
            self.root.after(10, lambda: self._render_historial_batch(filtered_items, end_idx, evento_map, container, batch_size, gen))
        else:
            for widget in self.scroll_historial.winfo_children():
                if widget != container:
                    widget.destroy()
            container.pack(side="top", fill="x", anchor="n")
            self.on_historial_select()

    def on_historial_select(self):
        if not self.selected_hist_c_id:
            self.historial_btn_frame.pack_forget()
            return
            
        self.historial_btn_frame.pack(side='bottom', pady=15)
        tipo = self.selected_hist_tipo
        evento = self.selected_hist_evento
        
        cmd = self.on_historial_movie if "Película" in tipo else self.on_historial_serie
        
        if evento == "Para después" or evento == "PARA_DESPUES":
            self.btn_hist_accion.configure(text="▶ Empezar ahora / Ver", state="normal", fg_color="#164E63", hover_color="#083344", text_color="#06B6D4", corner_radius=16, font=("Segoe UI", 13, "bold"), command=cmd)
        elif "Película" in tipo:
            self.btn_hist_accion.configure(text="✔ Ya la vi", state="normal", fg_color="#064E3B", hover_color="#022C22", text_color="#10B981", corner_radius=16, font=("Segoe UI", 13, "bold"), command=self.on_historial_movie)
        else: # Series / Anime
            self.btn_hist_accion.configure(text="▶ Empezar a ver", state="normal", fg_color="#164E63", hover_color="#083344", text_color="#06B6D4", corner_radius=16, font=("Segoe UI", 13, "bold"), command=self.on_historial_serie)

    def on_historial_movie(self):
        if not self.selected_hist_c_id: return
        c_id = self.selected_hist_c_id
        uc = recommendation.agregar_a_biblioteca(c_id)
        recommendation.marcar_vista_pelicula(uc.id)
        self.show_toast("Película marcada como vista.")
        self.refrescar_todo()

    def on_historial_serie(self):
        if not self.selected_hist_c_id: return
        c_id = self.selected_hist_c_id
        uc = recommendation.agregar_a_biblioteca(c_id)
        progreso = repository.obtener_progreso_serie(uc.id)
        if progreso:
            messagebox.showwarning("Aviso", "Ya estabas viendo esta serie.")
        else:
            recommendation.empezar_serie(uc.id)
            self.show_toast("¡Serie empezada! T1 C1")
        self.refrescar_todo()

    # --- PESTAÑA BUSCAR ---
    def setup_tab_buscar(self):
        top_frame = ctk.CTkFrame(self.tab_buscar, fg_color="transparent")
        top_frame.pack(fill='x', padx=10, pady=5)
        
        self.entry_buscar_online = ctk.CTkEntry(top_frame, placeholder_text="Buscar en TMDB / Jikan...", width=300, corner_radius=14)
        self.entry_buscar_online.pack(side='left', padx=5)
        self.entry_buscar_online.bind("<Return>", lambda e: self.on_buscar_online())
        
        btn_buscar = ctk.CTkButton(top_frame, text="Buscar", command=self.on_buscar_online,
                                   corner_radius=14, fg_color="#7C3AED", hover_color="#6D28D9", text_color="#FFFFFF")
        btn_buscar.pack(side='left', padx=5)
        
        self.scroll_buscar = ctk.CTkScrollableFrame(self.tab_buscar, fg_color="transparent")
        self.scroll_buscar.pack(expand=True, fill='both', padx=10, pady=5)
        
        self.resultados_busqueda = []

    def on_buscar_online(self):
        query = self.entry_buscar_online.get().strip()
        if not query: return
        
        for w in self.scroll_buscar.winfo_children():
            w.destroy()
            
        btn = self.entry_buscar_online.master.winfo_children()[1]
        btn.configure(text="Buscando...", state="disabled")
        
        def _process():
            try:
                res = recommendation.buscar_online(query)
                self.ui_queue.put(lambda: self._mostrar_resultados_busqueda(res))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
            finally:
                self.ui_queue.put(lambda: btn.configure(text="Buscar", state="normal"))
                
        threading.Thread(target=_process, daemon=True).start()

    def _mostrar_resultados_busqueda(self, resultados):
        self.resultados_busqueda = resultados
        
        new_container = ctk.CTkFrame(self.scroll_buscar, fg_color="transparent")
            
        for idx, c in enumerate(resultados):
            card = ctk.CTkFrame(new_container, fg_color="#181524", corner_radius=10, border_width=1, border_color="#262335")
            card.pack(side="top", fill='x', anchor="n", padx=10, pady=5)
            
            card.grid_columnconfigure(0, minsize=60) # Poster
            card.grid_columnconfigure(1, weight=1)   # Titulo + Subtitulo
            card.grid_columnconfigure(2, minsize=160) # Accion
            
            poster_lbl = ctk.CTkLabel(card, text="", width=60, height=90, fg_color="#2B1A4A", corner_radius=6)
            poster_lbl.grid(row=0, column=0, padx=10, pady=10)
            
            if c.poster_url:
                def _load_img(url, lbl):
                    try:
                        img = self._download_poster(url, size=(60, 90))
                        if img:
                            self.ui_queue.put(lambda l=lbl, i=img: l.configure(image=i, text="") if l.winfo_exists() else None)
                    except Exception as e:
                        print(f"Error en _load_img de busqueda: {e}")
                threading.Thread(target=_load_img, args=(c.poster_url, poster_lbl), daemon=True).start()
            
            text_container = ctk.CTkFrame(card, fg_color="transparent")
            text_container.grid(row=0, column=1, sticky="w", padx=10, pady=12)
            
            lbl_tit = ctk.CTkLabel(text_container, text=c.titulo, font=("Segoe UI", 14, "bold"), text_color="#F1F5F9", anchor="w", wraplength=350)
            lbl_tit.pack(anchor="w")
            
            tipo_label = "Anime" if c.es_anime else ("Película" if c.tipo == TIPO_PELICULA else "Serie")
            origen_label = "Jikan (Anime)" if c.es_anime else "TMDB"
            sub_text = f"{tipo_label} • {origen_label}"
            
            lbl_sub = ctk.CTkLabel(text_container, text=sub_text, font=("Segoe UI", 11), text_color="#94A3B8", anchor="w")
            lbl_sub.pack(anchor="w", pady=(2, 0))
            
            action_frame = ctk.CTkFrame(card, fg_color="transparent")
            action_frame.grid(row=0, column=2, padx=15, pady=12, sticky="e")
            
            estado = repository.obtener_estado_en_biblioteca(c)
            
            def handle_action(choice, i=idx, is_movie=(c.tipo == TIPO_PELICULA)):
                if choice == "📌 Para después":
                    self.on_buscar_para_despues(i)
                elif choice == "✔ Ya la vi (Terminada)":
                    if is_movie:
                        self.on_buscar_ya_vi(i)
                    else:
                        self.on_buscar_ya_termine(i)
                elif choice == "▶ Empezar ahora (En progreso)":
                    self.on_buscar_empezar(i)
                elif choice.startswith("📌 En Biblioteca"):
                    pass # Solo lectura o se podria extender
                    
            options = ["📌 Para después", "✔ Ya la vi (Terminada)"]
            if c.tipo != TIPO_PELICULA:
                options = ["📌 Para después", "▶ Empezar ahora (En progreso)", "✔ Ya la vi (Terminada)"]
                
            if estado:
                estado_legible = ESTADOS_LEGIBLES.get(estado, estado)
                current_val = f"📌 En Biblioteca ({estado_legible})"
                opt_menu = ctk.CTkOptionMenu(action_frame, values=[current_val] + options, command=handle_action,
                                             corner_radius=14, fg_color="#1E1B2E", button_color="#1E1B2E", button_hover_color="#2A2640",
                                             text_color="#C084FC", dropdown_fg_color="#181524", dropdown_hover_color="#3B185F", dropdown_text_color="#F1F5F9", font=("Segoe UI", 12))
                opt_menu.set(current_val)
                opt_menu.pack(side="right")
            else:
                opt_menu = ctk.CTkOptionMenu(action_frame, values=["+ Añadir a mi lista..."] + options, command=handle_action,
                                             corner_radius=14, fg_color="#1E1B2E", button_color="#1E1B2E", button_hover_color="#2A2640",
                                             text_color="#C084FC", dropdown_fg_color="#181524", dropdown_hover_color="#3B185F", dropdown_text_color="#F1F5F9", font=("Segoe UI", 12))
                opt_menu.set("+ Añadir a mi lista...")
                opt_menu.pack(side="right")
                
        for w in self.scroll_buscar.winfo_children():
            if w != new_container:
                w.destroy()
        new_container.pack(side="top", fill="x", anchor="n")

    def on_buscar_select(self, event):
        selected = self.tree_buscar.selection()
        if not selected:
            return
            
        # Ocultar todo primero
        for w in self.btn_frame_buscar.winfo_children():
            w.pack_forget()
            
        idx = int(self.tree_buscar.item(selected[0])['values'][0])
        c = self.resultados_busqueda[idx]
        
        estado = repository.obtener_estado_en_biblioteca(c)
        if estado:
            estado_legible = ESTADOS_LEGIBLES.get(estado, estado)
            self.lbl_buscar_estado.configure(text=f"📌 Ya está en tu Biblioteca (Estado: {estado_legible})")
            self.lbl_buscar_estado.pack(pady=10)
        else:
            if c.tipo == TIPO_PELICULA:
                self.btn_buscar_para_despues.pack(side='left', padx=5)
                self.btn_buscar_ya_vi.pack(side='left', padx=5)
            else:
                self.btn_buscar_empezar.pack(side='left', padx=5)
                self.btn_buscar_ya_viendo.pack(side='left', padx=5)
                self.btn_buscar_ya_termine.pack(side='left', padx=5)

    def _asegurar_contenido_bd(self, idx):
        c = self.resultados_busqueda[idx]
        c_db = repository.obtener_contenido_por_tmdb_id(c.tmdb_id, c.tipo) if c.tmdb_id else None
        if not c_db and c.mal_id:
            c_db = repository.obtener_contenido_por_mal_id(c.mal_id)
            
        if not c_db:
            new_id = repository.insertar_contenido(c)
            c_db = repository.obtener_contenido_por_id(new_id)
            
        return c_db

    def on_buscar_para_despues(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                repository.actualizar_estado_usuario_contenido(uc.id, ESTADO_PARA_DESPUES)
                repository.upsert_historial_recomendacion(c_db.id, "PARA_DESPUES")
                
                self.ui_queue.put(lambda: self.show_toast(f"'{c_db.titulo}' añadido para después."))
                self.ui_queue.put(self.refrescar_todo)
                self.ui_queue.put(lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
        threading.Thread(target=_process, daemon=True).start()
        
    def on_buscar_ya_vi(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                
                repository.actualizar_estado_usuario_contenido(uc.id, ESTADO_TERMINADA)
                repository.upsert_historial_recomendacion(c_db.id, "YA_LA_VI")
                
                self.ui_queue.put(lambda: self.show_toast(f"'{c_db.titulo}' marcada como vista."))
                self.ui_queue.put(self.refrescar_todo)
                self.ui_queue.put(lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
        threading.Thread(target=_process, daemon=True).start()
        
    def on_buscar_empezar(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                
                progreso = repository.obtener_progreso_serie(uc.id)
                if progreso:
                    self.ui_queue.put(lambda: messagebox.showwarning("Aviso", "Ya estabas viendo esta serie."))
                else:
                    recommendation.empezar_serie(uc.id)
                    self.ui_queue.put(lambda: self.show_toast(f"¡Empezaste '{c_db.titulo}'! (T1 C1)"))
                self.ui_queue.put(self.refrescar_todo)
                self.ui_queue.put(lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
        threading.Thread(target=_process, daemon=True).start()

    def on_buscar_ya_viendo(self, idx=None):
        if idx is None: return
        c = self.resultados_busqueda[idx]
        
        c_db = self._asegurar_contenido_bd(idx)
        uc = recommendation.agregar_a_biblioteca(c_db.id)
        
        def _on_success(p):
            self.show_toast(f"Progreso importado: T{p.temporada_actual} C{p.episodio_actual}")
            self.refrescar_todo()
            self._mostrar_resultados_busqueda(self.resultados_busqueda)
            
        ModalProgresoSerie(self.root, c_db, uc.id, _on_success)
        
    def on_buscar_ya_termine(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                p, _ = recommendation.marcar_serie_terminada(uc.id)
                self.ui_queue.put(lambda: self.show_toast(f"'{c_db.titulo}' terminada en T{p.temporada_actual} C{p.episodio_actual}"))
            except Exception as e:
                self.ui_queue.put(lambda: messagebox.showerror("Error", str(e)))
            finally:
                self.ui_queue.put(self.refrescar_todo)
                self.ui_queue.put(lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
                
        threading.Thread(target=_process, daemon=True).start()

def run():
    init_db()
    root = ctk.CTk()
    app = QueVeoHoyApp(root)
    root.mainloop()

if __name__ == "__main__":
    run()
