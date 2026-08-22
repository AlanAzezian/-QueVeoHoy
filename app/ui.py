import tkinter as tk
from tkinter import ttk, messagebox
import customtkinter as ctk
import urllib.request
import io
import threading
import os

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
                p = recommendation.importar_progreso_serie(
                    self.uc_id,
                    temp,
                    ep,
                    marcar_anteriores=True
                )
                self.after(0, lambda: self.on_success(p))
                self.after(0, self.destroy)
            except Exception as e:
                self.after(0, lambda: messagebox.showerror("Error", str(e)))
                self.after(0, lambda: self.btn_confirmar.configure(text="Confirmar", state="normal"))
                
        threading.Thread(target=_process, daemon=True).start()

class QueVeoHoyApp:
    def __init__(self, root):
        self.root = root
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
        
        # Main Tabview
        self.tabview = ctk.CTkTabview(self.root, fg_color=COLOR_BG_CARD, segmented_button_fg_color=COLOR_BG,
                                      segmented_button_selected_color=COLOR_PRIMARY,
                                      segmented_button_selected_hover_color=COLOR_HOVER,
                                      segmented_button_unselected_hover_color=COLOR_DARK,
                                      corner_radius=16)
        self.tabview.pack(expand=True, fill='both', padx=20, pady=20)
        
        self.tab_hoy = self.tabview.add("Hoy")
        self.tab_en_progreso = self.tabview.add("En progreso")
        self.tab_pendientes = self.tabview.add("Biblioteca")
        self.tab_historial = self.tabview.add("Historial")
        self.tab_buscar = self.tabview.add("Buscar")
        
        self.tabview.configure(command=self.on_tab_changed)
        
        self.poster_cache = {}
        self.recomendacion_actual = None
        self._next_rec_cache = None
        self._is_prefetching = False

        self.setup_tab_hoy()
        self.setup_tab_en_progreso()
        self.setup_tab_pendientes()
        self.setup_tab_historial()
        self.setup_tab_buscar()
        
        self.apply_treeview_style()
        
        if not HAS_PILLOW:
            messagebox.showwarning("Falta Pillow", "La librería Pillow no está instalada. No se mostrarán los pósters.")

        self.root.after(100, self.refrescar_todo)
        
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

    def on_tab_changed(self):
        self.refrescar_todo()

    def refrescar_todo(self):
        self.refresh_hoy()
        self.refresh_en_progreso()
        self.refresh_pendientes()
        self.refresh_historial()

    # --- PESTAÑA HOY ---
    def setup_tab_hoy(self):
        # Frame central tipo tarjeta
        self.card_frame = ctk.CTkFrame(self.tab_hoy, fg_color=COLOR_BG_CARD, corner_radius=20)
        self.card_frame.pack(expand=True, fill='both', padx=40, pady=(0, 5))
        
        # Botones anclados abajo (se empaquetan primero para garantizar su visibilidad en el fondo)
        btn_frame = ctk.CTkFrame(self.card_frame, fg_color="transparent")
        btn_frame.pack(side='bottom', pady=(5, 15))
        
        self.btn_visto = ctk.CTkButton(btn_frame, text="✔ Ya la vi", command=self.on_marcar_visto, 
                                       corner_radius=18, fg_color="#064E3B", hover_color="#042F2E", 
                                       text_color="#10B981", border_color="#059669", border_width=2, font=("Segoe UI", 12, "bold"))
        
        self.btn_para_despues = ctk.CTkButton(btn_frame, text="🕒 Dejar para después", command=self.on_para_despues, 
                                              corner_radius=18, fg_color="#78350F", hover_color="#451A03", 
                                              text_color="#F59E0B", border_color="#D97706", border_width=2, font=("Segoe UI", 12, "bold"))
                                              
        self.btn_siguiente = ctk.CTkButton(btn_frame, text="⏩ Siguiente", command=self.on_siguiente, 
                                           corner_radius=18, fg_color="#3B0764", hover_color="#2E054E", 
                                           text_color="#C084FC", border_color="#A855F7", border_width=2, font=("Segoe UI", 12, "bold"))
        
        self.btn_visto.pack(side='left', padx=5)
        self.btn_para_despues.pack(side='left', padx=5)
        self.btn_siguiente.pack(side='left', padx=5)
        
        # Fila extra de botones
        btn_frame_extra = ctk.CTkFrame(self.card_frame, fg_color="transparent", height=0)
        btn_frame_extra.pack(side='bottom', pady=(0, 5))
        
        self.btn_pausar = ctk.CTkButton(btn_frame_extra, text="⏸ Pausar", command=self.on_pausar, 
                                        corner_radius=18, fg_color="#1E1B4B", hover_color="#17153B", 
                                        text_color="#818CF8", border_color="#6366F1", border_width=2, font=("Segoe UI", 12, "bold"))
        self.btn_abandonar = ctk.CTkButton(btn_frame_extra, text="✕ Abandonar", command=self.on_abandonar, 
                                           corner_radius=18, fg_color="#7F1D1D", hover_color="#450A0A", 
                                           text_color="#EF4444", border_color="#DC2626", border_width=2, font=("Segoe UI", 12, "bold"))
        self.btn_ya_viendo = ctk.CTkButton(btn_frame_extra, text="▶ Ya la estoy viendo", command=self.on_ya_viendo, 
                                           corner_radius=18, fg_color="#164E63", hover_color="#083344", 
                                           text_color="#06B6D4", border_color="#0891B2", border_width=2, font=("Segoe UI", 12, "bold"))
        self.btn_ya_termine = ctk.CTkButton(btn_frame_extra, text="✔ Ya la terminé", command=self.on_ya_termine, 
                                            corner_radius=18, fg_color="#064E3B", hover_color="#042F2E", 
                                            text_color="#10B981", border_color="#059669", border_width=2, font=("Segoe UI", 12, "bold"))

        # Póster Frame (Aura / Profundidad)
        self.shadow_frame = ctk.CTkFrame(self.card_frame, fg_color="transparent", 
                                         corner_radius=16, border_width=3, border_color="#A855F7")
        self.shadow_frame.pack(side='top', pady=(10, 2))
        
        # Póster
        self.lbl_poster = ctk.CTkLabel(self.shadow_frame, text="")
        self.lbl_poster.pack(padx=10, pady=10)
        
        # Info
        self.lbl_titulo = ctk.CTkLabel(self.card_frame, text="Cargando...", font=("Helvetica", 22, "bold"), text_color=COLOR_TEXT, wraplength=480, justify="center")
        self.lbl_titulo.pack(side='top', padx=25, pady=(2, 2))
        
        self.detalle_frame = ctk.CTkFrame(self.card_frame, fg_color="transparent")
        self.detalle_frame.pack(side='top', pady=(0, 6))
        
        self.lbl_badge_tipo = ctk.CTkLabel(self.detalle_frame, text="", fg_color="#3B0764", text_color="#C084FC", corner_radius=8, font=("Segoe UI", 13, "bold"))
        self.lbl_badge_tipo.pack(side="left", padx=(0, 8))
        
        self.lbl_anio = ctk.CTkLabel(self.detalle_frame, text="", text_color="#94A3B8", font=("Segoe UI", 13))
        self.lbl_anio.pack(side="left")
        
        # Sinopsis (Cambiado a CTkTextbox para evitar recortes y permitir flujo nativo)
        self.lbl_sinopsis = ctk.CTkTextbox(self.card_frame, font=("Helvetica", 14), text_color="#E0E0E5", 
                                           fg_color="transparent", border_width=0, wrap="word", activate_scrollbars=False, height=240)
        self.lbl_sinopsis.pack(side='top', padx=30, pady=(5, 15), fill='both', expand=True)

    def _set_sinopsis(self, texto):
        self.lbl_sinopsis.configure(state="normal")
        self.lbl_sinopsis.delete("1.0", "end")
        self.lbl_sinopsis.insert("1.0", texto)
        self.lbl_sinopsis.tag_config("center", justify="center")
        self.lbl_sinopsis.tag_add("center", "1.0", "end")
        self.lbl_sinopsis.configure(state="disabled")

    def _download_poster(self, url, size=(180, 270)):
        if not HAS_PILLOW or not url:
            return None
            
        cache_key = (url, size)
        if cache_key in self.poster_cache:
            return self.poster_cache[cache_key]
            
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
            raw_data = urllib.request.urlopen(req, timeout=5).read()
            im = Image.open(io.BytesIO(raw_data))
            
            # Cambiamos la escala original para garantizar que todo quepa en la ventana sin colapsar
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
                self._next_rec_cache = (rec, img)
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
        
        self.btn_visto.configure(state="disabled")
        self.btn_para_despues.configure(state="disabled")
        self.btn_siguiente.configure(state="disabled")
        
        if not self.recomendacion_actual:
            if self._next_rec_cache and not forzar_aleatoria:
                rec, img = self._next_rec_cache
                self._next_rec_cache = None
                

                self._apply_rec(rec, img=img, current_id=current_id)
                return
                
            self.lbl_titulo.configure(text="Cargando...")
            self.lbl_badge_tipo.configure(text="")
            self.lbl_anio.configure(text="")
            self._set_sinopsis("Buscando la mejor recomendación...")
            self.lbl_poster.configure(image=None, text="Cargando...")
            self.btn_pausar.pack_forget()
            self.btn_abandonar.pack_forget()
            self.btn_ya_viendo.pack_forget()
            self.btn_ya_termine.pack_forget()
            
            def _fetch():
                try:
                    rec = recommendation.recomendacion_de_hoy(forzar_aleatoria=forzar_aleatoria)
                    if current_id != self._rec_request_id: return
                    
                    img = self._download_poster(rec.poster_url) if rec else None
                    if current_id != self._rec_request_id: return
                    
                    self.root.after(0, lambda r=rec, i=img: self._apply_rec(r, img=i, current_id=current_id))
                    self.root.after(0, self._prefetch_next)
                except Exception as e:
                    err_msg = str(e)
                    import traceback
                    traceback.print_exc()
                    if current_id != self._rec_request_id: return
                    self.root.after(0, lambda msg=err_msg: self._apply_rec(None, error=msg, current_id=current_id))
            threading.Thread(target=_fetch, daemon=True).start()
        else:
            def _fetch_poster():
                img = self._download_poster(self.recomendacion_actual.poster_url)
                if current_id != self._rec_request_id: return
                self.root.after(0, lambda: self._apply_rec(self.recomendacion_actual, img=img, current_id=current_id))
                self.root.after(0, self._prefetch_next)
            threading.Thread(target=_fetch_poster, daemon=True).start()
            
    def _apply_rec(self, rec, error=None, img=None, current_id=None):
        if current_id is not None and current_id != getattr(self, '_rec_request_id', -1):
            return
            
        if error:
            self.lbl_titulo.configure(text="Error")
            self._set_sinopsis(error)
            self.lbl_poster.configure(image=None, text="Error")
            self.btn_visto.configure(state="normal")
            self.btn_para_despues.configure(state="normal")
            self.btn_siguiente.configure(state="normal")
            return
            
        self.recomendacion_actual = rec
        
        # Persistencia centralizada de la recomendación activa actual
        try:
            repository.upsert_historial_recomendacion(rec.id, "RECOMENDADA_HOY")
        except Exception:
            pass
        
        self.btn_pausar.pack_forget()
        self.btn_abandonar.pack_forget()
        self.btn_ya_viendo.pack_forget()
        self.btn_ya_termine.pack_forget()
        self.btn_ya_termine.configure(text="✔ Ya la terminé", state="normal")
        self.btn_ya_viendo.configure(text="▶ Ya la estoy viendo", state="normal")
        
        # Restaurar btn_para_despues si estaba oculto, para mantener orden, se usa before
        if not self.btn_para_despues.winfo_ismapped():
            self.btn_para_despues.pack(side='left', padx=5, before=self.btn_siguiente)
        
        if not rec:
            self.lbl_titulo.configure(text="¡No hay nada nuevo para ver!")
            self.lbl_badge_tipo.configure(text="")
            self.lbl_anio.configure(text="")
            self._set_sinopsis("No se encontraron recomendaciones. Intentá más tarde.")
            self.lbl_poster.configure(image=None, text="No content")
            
            self.btn_visto.configure(state="disabled")
            self.btn_para_despues.configure(state="disabled")
            self.btn_siguiente.configure(state="disabled")
            return

        self.btn_visto.configure(state="normal")
        self.btn_para_despues.configure(state="normal")
        self.btn_siguiente.configure(state="normal")

        c = rec
        
        print(f"DEBUG CONTENIDO: {c.titulo} | Tipo: {c.tipo} | Estreno: {c.fecha_estreno} | Sinopsis: {c.sinopsis[:30] if c.sinopsis else 'VACIA'}")
        
        self.lbl_titulo.configure(text=c.titulo)
        
        anio = ""
        if c.fecha_estreno and len(c.fecha_estreno) >= 4:
            anio = c.fecha_estreno[:4]
            
        if c.tipo == TIPO_PELICULA:
            tipo_texto = "PELÍCULA"
        elif c.es_anime:
            tipo_texto = "ANIME • SERIE" if c.tipo == TIPO_SERIE else "ANIME • PELÍCULA"
        else:
            tipo_texto = "SERIE"
            
        self.lbl_badge_tipo.configure(text=f" {tipo_texto} ")
            
        uc = repository.obtener_usuario_contenido_por_contenido_id(c.id)
        if c.tipo == TIPO_SERIE and uc and uc.estado in (ESTADO_EN_PROGRESO, ESTADO_PAUSADA):
            progreso = repository.obtener_progreso_serie(uc.id)
            if progreso:
                anio += f" • T{progreso.temporada_actual} C{progreso.episodio_actual}"
                
        self.lbl_anio.configure(text=f"• {anio}" if anio else "")
        if c.tipo == TIPO_SERIE:
            if uc and uc.estado == ESTADO_EN_PROGRESO:
                self.btn_visto.configure(text="✔ Capítulo Visto")
                self.btn_para_despues.pack_forget()
                self.btn_pausar.pack(side='left', padx=5)
                self.btn_abandonar.pack(side='left', padx=5)
            else:
                self.btn_visto.configure(text="▶ Empezar Serie")
                if not uc or uc.estado == ESTADO_PARA_DESPUES:
                    self.btn_ya_viendo.pack(side='left', padx=5)
                    self.btn_ya_termine.pack(side='left', padx=5)
        else:
            self.btn_visto.configure(text="✔ Ya la vi")
            
        self._set_sinopsis(c.sinopsis if c.sinopsis else "Sin sinopsis disponible.")
        
        if img:
            self.lbl_poster.configure(image=img, text="")
        else:
            self.lbl_poster.configure(image=None, text="Sin Imagen" if not c.poster_url else "Cargando...")
            
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
                                self.root.after(0, lambda: self.show_toast(f"¡Has finalizado {c.titulo}!"))
                                self.recomendacion_actual = None
                            else:
                                self.root.after(0, lambda p=p: self.show_toast(f"¡Visto! Ahora estás en T{p.temporada_actual} C{p.episodio_actual}"))
                        else:
                            recommendation.empezar_serie(uc.id)
                            self.root.after(0, lambda: self.show_toast("¡Serie empezada! T1 C1"))
                    else:
                        recommendation.marcar_vista_pelicula(uc.id)
                        self.recomendacion_actual = None
                        self.root.after(0, lambda: self.show_toast("¡Película marcada como vista!"))
                except Exception as e:
                    self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
                
                self.root.after(0, self.refrescar_todo)

            threading.Thread(target=_process, daemon=True).start()

    def on_para_despues(self):
        if self.recomendacion_actual:
            c = self.recomendacion_actual
            self.recomendacion_actual = None
            
            def _process():
                uc = recommendation.agregar_a_biblioteca(c.id)
                recommendation.guardar_para_despues(uc.id)
                self.root.after(0, self.refrescar_todo)
                self.root.after(0, self._prefetch_next)
                
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
                self.root.after(0, lambda: self.refresh_hoy(forzar_aleatoria=was_series_in_progress))
                self.root.after(0, self._prefetch_next)
                
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
                self.root.after(0, lambda: self.show_toast("Serie pausada."))
                self.root.after(0, lambda: self.refresh_hoy(forzar_aleatoria=was_series_in_progress))
                self.root.after(0, self.refrescar_todo)
                
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
            
            def _on_success(p):
                self.show_toast(f"Importada. Vas en T{p.temporada_actual} C{p.episodio_actual}")
                self.recomendacion_actual = None
                self.refrescar_todo()
                
            ModalProgresoSerie(self.root, c, uc.id if hasattr(uc, 'id') else uc.id, _on_success)

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
                        
                    self.root.after(0, _on_success)
                except Exception as e:
                    print(f"Error al terminar serie: {e}")
                    self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
                    self.root.after(0, self.refrescar_todo)
                
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
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="SERIE / TÍTULO", font=("Segoe UI", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_titulo.grid(row=0, column=0, sticky="w", padx=(20, 10), pady=8)
        
        lbl_h_avance = ctk.CTkLabel(header_frame, text="AVANCE", font=("Segoe UI", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_avance.grid(row=0, column=1, sticky="w", pady=8)
        
        lbl_h_estado = ctk.CTkLabel(header_frame, text="ESTADO", font=("Segoe UI", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_estado.grid(row=0, column=2, sticky="w", pady=8)
        
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
        for widget in self.scroll_progreso.winfo_children():
            widget.destroy()
            
        self.selected_prog_row_frame = None
        self.selected_prog_uc_id = None
        
        items = recommendation.obtener_series_activas()
        for idx, item in enumerate(items):
            avance = f"T{item.temporada_actual} C{item.episodio_actual}"
            estado_legible = "Pausada" if item.estado == "pausada" else "En progreso"
            tipo_legible = "Anime (Serie)" if item.es_anime else "Serie"
            
            # Crear la fila
            row_frame = ctk.CTkFrame(self.scroll_progreso, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(fill='x', padx=5, pady=0)
            
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
                    if img and lbl.winfo_exists():
                        lbl.configure(image=img, text="")
                threading.Thread(target=_load_img, args=(c.poster_url, lbl_poster), daemon=True).start()
            
            # Contenedor para título y badge
            text_container = ctk.CTkFrame(tit_frame, fg_color="transparent", cursor="hand2")
            text_container.pack(side="left", anchor="center")
            
            lbl_tit = ctk.CTkLabel(text_container, text=item.titulo, font=("Segoe UI", 14, "bold"), text_color=COLOR_TEXT, anchor="w", cursor="hand2")
            lbl_tit.pack(anchor="w")
            
            if item.es_anime:
                anime_badge = ctk.CTkFrame(text_container, fg_color="#3B0764", corner_radius=4, cursor="hand2")
                anime_badge.pack(anchor="w", pady=(4, 0))
                ctk.CTkLabel(anime_badge, text="ANIME", font=("Segoe UI", 9, "bold"), text_color="#C084FC", cursor="hand2").pack(padx=6, pady=2)
            
            # --- COLUMNA 1: AVANCE ---
            avance_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            avance_frame.grid(row=0, column=1, sticky="w", pady=10)
            
            # Formatear el texto de avance en cyan claro y ubicarlo alineado
            lbl_avance = ctk.CTkLabel(avance_frame, text=avance, font=("Segoe UI", 12, "bold"), text_color="#06B6D4", anchor="w", cursor="hand2")
            lbl_avance.pack(side="top", anchor="w")
            
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
            prog_bar.pack(side="top", pady=(4, 0), anchor="w")
            prog_bar.set(porcentaje)
            
            # --- COLUMNA 2: ESTADO ---
            badge_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            badge_frame.grid(row=0, column=2, sticky="w", pady=10)
            
            badge = ui_styles.crear_badge_estado(badge_frame, estado_legible)
            badge.pack(anchor="w")
            
            # Separador sutil
            if idx < len(items) - 1:
                sep = ctk.CTkFrame(self.scroll_progreso, height=1, fg_color="#232330")
                sep.pack(fill='x', padx=15)
            
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
                    self.tabview.set("Hoy")
            
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
                self.root.after(0, lambda: self.show_toast(f"Progreso corregido: T{p.temporada_actual} C{p.episodio_actual}"))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
            self.root.after(0, self.refrescar_todo)
            
        threading.Thread(target=_process, daemon=True).start()

    # --- PESTAÑA PARA DESPUÉS ---
    def setup_tab_pendientes(self):
        # Fila Superior (Panel de Métricas):
        stats_frame = ctk.CTkFrame(self.tab_pendientes, fg_color="#1A1A22", corner_radius=10)
        stats_frame.pack(fill='x', padx=10, pady=(5, 5))
        
        lbl_stats_title = ctk.CTkLabel(stats_frame, text="📊 Contenido visto:", text_color="#E0E0E5", font=("Helvetica", 14, "bold"))
        lbl_stats_title.pack(pady=(10, 0))
        
        self.lbl_stats = ctk.CTkLabel(stats_frame, text="🏆 Total: 0 | 🎬 Pelis: 0 | 📺 Series: 0 | ⛩️ Anime Series: 0 | ⛩️ Anime Pelis: 0", text_color="#A78BFA", font=("Helvetica", 13, "bold"))
        self.lbl_stats.pack(pady=(5, 10))

        # Fila Inferior (Filtros y Buscador):
        controls_frame = ctk.CTkFrame(self.tab_pendientes, fg_color="transparent")
        controls_frame.pack(fill='x', padx=10, pady=5)
        
        self.filtro_tipo_pendientes = ctk.CTkOptionMenu(controls_frame, values=["Tipo", "Películas", "Series", "Anime (Todos)", "Anime (Series)", "Anime (Películas)"], command=lambda _: self.refresh_pendientes(),
                                                   fg_color="#6D28D9", button_color="#5B21B6", button_hover_color="#7C3AED",
                                                   dropdown_fg_color="#1E1E24", dropdown_hover_color="#6D28D9",
                                                   dropdown_text_color="#FFFFFF", text_color="#FFFFFF")
        self.filtro_tipo_pendientes.set("Tipo")
        self.filtro_tipo_pendientes.pack(side='left', padx=5)

        self.filtro_estado_pendientes = ctk.CTkOptionMenu(controls_frame, values=["Estado", "Para después", "Terminados / Vistos", "Abandonadas"], command=lambda _: self.refresh_pendientes(),
                                                   fg_color="#6D28D9", button_color="#5B21B6", button_hover_color="#7C3AED",
                                                   dropdown_fg_color="#1E1E24", dropdown_hover_color="#6D28D9",
                                                   dropdown_text_color="#FFFFFF", text_color="#FFFFFF")
        self.filtro_estado_pendientes.set("Estado")
        self.filtro_estado_pendientes.pack(side='left', padx=5)
        
        self.entry_buscar_pendientes = ctk.CTkEntry(controls_frame, placeholder_text="Buscar en biblioteca...")
        self.entry_buscar_pendientes.pack(side='right', fill='x', expand=True, padx=5)
        self.entry_buscar_pendientes.bind("<KeyRelease>", lambda event: self.refresh_pendientes())
        
        # --- NUEVA LISTA CUSTOM CON SCROLL ---
        self.pend_list_container = ctk.CTkFrame(self.tab_pendientes, fg_color=COLOR_BG_CARD)
        self.pend_list_container.pack(expand=True, fill='both', padx=10, pady=(10, 0))
        
        # Header Row
        header_frame = ctk.CTkFrame(self.pend_list_container, fg_color=COLOR_BG, corner_radius=8)
        header_frame.pack(fill='x', padx=5, pady=5)
        
        header_frame.grid_columnconfigure(0, weight=1)
        header_frame.grid_columnconfigure(1, minsize=100)
        header_frame.grid_columnconfigure(2, minsize=120)
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="TÍTULO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_titulo.grid(row=0, column=0, sticky="w", padx=15, pady=5)
        
        lbl_h_tipo = ctk.CTkLabel(header_frame, text="TIPO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_tipo.grid(row=0, column=1, sticky="w", padx=5, pady=5)
        
        lbl_h_estado = ctk.CTkLabel(header_frame, text="ESTADO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_estado.grid(row=0, column=2, padx=5, pady=5)
        
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
        
        btn_reanudar = ctk.CTkButton(btn_frame, text="Reanudar Seleccionado", command=self.on_reanudar,
                                     corner_radius=30, fg_color=COLOR_PRIMARY, hover_color=COLOR_HOVER)
        btn_eliminar_sel = ctk.CTkButton(btn_frame, text="Eliminar Seleccionados", command=self.on_eliminar_seleccionados,
                                         corner_radius=30, fg_color="transparent", border_width=1, border_color="#555")
        btn_eliminar_todos = ctk.CTkButton(btn_frame, text="Eliminar Todos", command=self.on_eliminar_todos,
                                           corner_radius=30, fg_color="transparent", border_width=1, border_color="#e04f4f", text_color="#e04f4f")
                                           
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
        for widget in self.scroll_pendientes.winfo_children():
            widget.destroy()
            
        self.selected_pend_row_frame = None
        self.selected_pend_uc_id = None
        self.selected_pend_c_id = None
        self.selected_pend_tipo = None
            
        try:
            m = repository.obtener_metricas_biblioteca()
            
            items = recommendation.obtener_biblioteca_inactiva()
            
            # Fallback en memoria si la BD da 0
            if m['total'] == 0 and len(items) > 0:
                pelis_mem = series_mem = anime_series_mem = anime_pelis_mem = 0
                for item in items:
                    c = item.contenido
                    e = item.usuario_contenido
                    if e.estado.lower() in ('terminada', 'terminado', 'vista', 'visto'):
                        is_anime = c.es_anime or c.mal_id is not None
                        if is_anime and c.tipo.upper() == 'TV':
                            anime_series_mem += 1
                        elif is_anime and c.tipo.upper() == 'MOVIE':
                            anime_pelis_mem += 1
                        elif c.tipo.upper() == 'MOVIE':
                            pelis_mem += 1
                        elif c.tipo.upper() == 'TV':
                            series_mem += 1
                
                if (pelis_mem + series_mem + anime_series_mem + anime_pelis_mem) > 0:
                    m = {
                        "pelis": pelis_mem,
                        "series": series_mem,
                        "anime_series": anime_series_mem,
                        "anime_pelis": anime_pelis_mem,
                        "total": pelis_mem + series_mem + anime_series_mem + anime_pelis_mem
                    }

            print(f"[DEBUG STATS] Métricas calculadas: {m}")
            texto = f"🏆 Total: {m['total']} | 🎬 Pelis: {m['pelis']} | 📺 Series: {m['series']} | ⛩️ Anime Series: {m.get('anime_series', 0)} | ⛩️ Anime Pelis: {m.get('anime_pelis', 0)}"
            self.lbl_stats.configure(text=texto)
        except Exception as e:
            print(f"[ERROR STATS] Error calculando métricas: {e}")
            items = recommendation.obtener_biblioteca_inactiva()
        
        filtro_tipo = getattr(self, "filtro_tipo_pendientes", None)
        filtro_tipo_val = filtro_tipo.get() if filtro_tipo else "Tipo"
        
        filtro_estado = getattr(self, "filtro_estado_pendientes", None)
        filtro_estado_val = filtro_estado.get() if filtro_estado else "Estado"
        
        entry_busq = getattr(self, "entry_buscar_pendientes", None)
        texto_busq = entry_busq.get().strip().lower() if entry_busq else ""
        
        for item in items:
            c = item.contenido
            e = item.usuario_contenido
            
            if texto_busq and texto_busq not in c.titulo.lower():
                continue
                
            # 1. Aplicar filtro de tipo
            if not self._cumple_filtro(c, filtro_tipo_val):
                continue
                
            # 2. Aplicar filtro de estado
            is_terminado = e.estado in (ESTADO_TERMINADA, ESTADO_VISTA)
            is_para_despues = e.estado == ESTADO_PARA_DESPUES
            is_abandonada = e.estado == ESTADO_ABANDONADA
            
            if filtro_estado_val == "Para después" and not is_para_despues:
                continue
            elif filtro_estado_val == "Terminados / Vistos" and not is_terminado:
                continue
            elif filtro_estado_val == "Abandonadas" and not is_abandonada:
                continue
            elif filtro_estado_val in ("Todos", "Estado"):
                pass # Incluye todo
                
            tipo_legible = self._get_tipo_legible(c)
            estado_legible = ESTADOS_LEGIBLES.get(e.estado, e.estado)
            
            # Crear la fila
            row_frame = ctk.CTkFrame(self.scroll_pendientes, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(fill='x', padx=5, pady=2)
            
            row_frame.grid_columnconfigure(0, weight=1)
            row_frame.grid_columnconfigure(1, minsize=100)
            row_frame.grid_columnconfigure(2, minsize=120)
            
            lbl_tit = ctk.CTkLabel(row_frame, text=c.titulo, font=("Helvetica", 12), text_color=COLOR_TEXT, anchor="w", cursor="hand2")
            lbl_tit.grid(row=0, column=0, sticky="w", padx=15, pady=8)
            
            lbl_tipo = ctk.CTkLabel(row_frame, text=tipo_legible, font=("Helvetica", 12), text_color=COLOR_TEXT_SEC, anchor="w", cursor="hand2")
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
                self.tabview.set("Hoy")
                
            except recommendation.LimiteSeriesEnProgresoAlcanzado:
                messagebox.showerror("Límite Alcanzado", "No podés tener más de 4 series en progreso.")
            except Exception as e:
                messagebox.showerror("Error", str(e))
        else:
            self.show_toast("Película seleccionada para ver hoy")
            self.recomendacion_actual = c
            self._next_rec_cache = None
            self.refrescar_todo()
            self.tabview.set("Hoy")

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
        
        lbl_filtro = ctk.CTkLabel(top_frame, text="Filtrar:", text_color="#E0E0E5")
        lbl_filtro.pack(side='left', padx=(0, 5))
        
        self.filtro_historial = ctk.CTkOptionMenu(top_frame, values=["Todos", "Películas", "Series", "Anime (Todos)", "Anime (Series)", "Anime (Películas)"], command=lambda _: self.refresh_historial(),
                                                  fg_color="#6D28D9", button_color="#5B21B6", button_hover_color="#7C3AED",
                                                  dropdown_fg_color="#1E1E24", dropdown_hover_color="#6D28D9",
                                                  dropdown_text_color="#FFFFFF", text_color="#FFFFFF")
        self.filtro_historial.set("Todos")
        self.filtro_historial.pack(side='left', padx=5)
        
        self.entry_buscar_historial = ctk.CTkEntry(top_frame, placeholder_text="Buscar en historial...", width=300)
        self.entry_buscar_historial.pack(side='left', padx=15)
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
        
        lbl_h_fecha = ctk.CTkLabel(header_frame, text="FECHA", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_fecha.grid(row=0, column=0, sticky="w", padx=15, pady=5)
        
        lbl_h_titulo = ctk.CTkLabel(header_frame, text="TÍTULO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_titulo.grid(row=0, column=1, sticky="w", padx=5, pady=5)
        
        lbl_h_tipo = ctk.CTkLabel(header_frame, text="TIPO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_tipo.grid(row=0, column=2, sticky="w", padx=5, pady=5)
        
        lbl_h_evento = ctk.CTkLabel(header_frame, text="EVENTO", font=("Helvetica", 11, "bold"), text_color=COLOR_TEXT_SEC)
        lbl_h_evento.grid(row=0, column=3, padx=5, pady=5)
        
        # Scrollable Area
        self.scroll_historial = ctk.CTkScrollableFrame(self.hist_list_container, fg_color="transparent")
        self.scroll_historial.pack(expand=True, fill='both', padx=0, pady=0)
        
        # Variables de selección
        self.selected_hist_c_id = None
        self.selected_hist_tipo = None
        self.selected_hist_row_frame = None
        
        self.historial_btn_frame = ctk.CTkFrame(self.tab_historial, fg_color="transparent")
        self.historial_btn_frame.pack(side='bottom', pady=15)
        
        self.btn_hist_accion = ctk.CTkButton(self.historial_btn_frame, text="Seleccionar...", state="disabled",
                                      corner_radius=30, fg_color="#6D28D9", hover_color="#7C3AED",
                                      text_color="#FFFFFF", text_color_disabled="#FFFFFF")
        self.btn_hist_accion.pack(side='left', padx=5)

    def select_historial_row(self, row_frame, c_id, tipo):
        if self.selected_hist_row_frame:
            self.selected_hist_row_frame.configure(fg_color="transparent")
            
        self.selected_hist_row_frame = row_frame
        self.selected_hist_c_id = c_id
        self.selected_hist_tipo = tipo
        
        # Resaltado sutil
        row_frame.configure(fg_color="#2A2A35")
        self.on_historial_select()

    def refresh_historial(self):
        for widget in self.scroll_historial.winfo_children():
            widget.destroy()
            
        self.selected_hist_row_frame = None
        self.selected_hist_c_id = None
        self.selected_hist_tipo = None
            
        items = repository.obtener_historial_recomendaciones(limit=100)
        search_term = self.entry_buscar_historial.get().lower()
        filtro = getattr(self, "filtro_historial", None)
        filtro_val = filtro.get() if filtro else "Todos"
        
        evento_map = {
            "PARA_DESPUES": "Para después",
            "YA_LA_VI": "Ya la vi",
            "SIGUIENTE": "Siguiente",
            "RECOMENDADA_HOY": "Recomendada",
            "EMPEZAR_EN_PROGRESO": "En progreso",
            "TERMINADA": "Terminada"
        }
        
        for h in items:
            fecha_str = str(h.fecha)[:10] if h.fecha else ""
            c = repository.obtener_contenido_por_id(h.contenido_id)
            
            if c and not self._cumple_filtro(c, filtro_val):
                continue
                
            titulo = c.titulo if c else "Desconocido"
            tipo = self._get_tipo_legible(c)
            
            raw_evento = h.accion if h.accion else ""
            evento = evento_map.get(raw_evento, raw_evento.replace("_", " ").capitalize())
            
            if search_term and search_term not in titulo.lower():
                continue
                
            # Crear la fila
            row_frame = ctk.CTkFrame(self.scroll_historial, fg_color="transparent", corner_radius=8, cursor="hand2")
            row_frame.pack(fill='x', padx=5, pady=2)
            
            row_frame.grid_columnconfigure(0, minsize=100)
            row_frame.grid_columnconfigure(1, weight=1)
            row_frame.grid_columnconfigure(2, minsize=120)
            row_frame.grid_columnconfigure(3, minsize=140)
            
            lbl_fecha = ctk.CTkLabel(row_frame, text=fecha_str, font=("Helvetica", 11), text_color=COLOR_TEXT_SEC, anchor="w", cursor="hand2")
            lbl_fecha.grid(row=0, column=0, sticky="w", padx=15, pady=8)
            
            lbl_tit = ctk.CTkLabel(row_frame, text=titulo, font=("Helvetica", 12, "bold"), text_color=COLOR_TEXT, anchor="w", cursor="hand2")
            lbl_tit.grid(row=0, column=1, sticky="w", padx=5, pady=8)
            
            lbl_tipo = ctk.CTkLabel(row_frame, text=tipo, font=("Helvetica", 12), text_color=COLOR_TEXT_SEC, anchor="w", cursor="hand2")
            lbl_tipo.grid(row=0, column=2, sticky="w", padx=5, pady=8)
            
            badge_frame = ctk.CTkFrame(row_frame, fg_color="transparent", cursor="hand2")
            badge_frame.grid(row=0, column=3, padx=5, pady=8)
            
            badge = ui_styles.crear_badge_estado(badge_frame, evento)
            badge.pack()
            
            # Evento de selección
            def on_click(evt, r=row_frame, cid=h.contenido_id, t=tipo):
                self.select_historial_row(r, cid, t)
                
            row_frame.bind("<Button-1>", on_click)
            lbl_fecha.bind("<Button-1>", on_click)
            lbl_tit.bind("<Button-1>", on_click)
            lbl_tipo.bind("<Button-1>", on_click)
            badge_frame.bind("<Button-1>", on_click)
            badge.bind("<Button-1>", on_click)
            
        self.on_historial_select()

    def on_historial_select(self):
        if not self.selected_hist_c_id:
            self.btn_hist_accion.configure(text="Seleccionar...", state="disabled", command=None)
            return
            
        tipo = self.selected_hist_tipo
        
        if "Película" in tipo:
            self.btn_hist_accion.configure(text="Ya la vi", state="normal", command=self.on_historial_movie)
        elif "Serie" in tipo:
            self.btn_hist_accion.configure(text="Empezar a ver", state="normal", command=self.on_historial_serie)

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
        
        self.entry_buscar_online = ctk.CTkEntry(top_frame, placeholder_text="Buscar en TMDB / Jikan...", width=300)
        self.entry_buscar_online.pack(side='left', padx=5)
        self.entry_buscar_online.bind("<Return>", lambda e: self.on_buscar_online())
        
        btn_buscar = ctk.CTkButton(top_frame, text="Buscar", command=self.on_buscar_online,
                                   corner_radius=30, fg_color=COLOR_PRIMARY, hover_color=COLOR_HOVER)
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
                self.root.after(0, lambda: self._mostrar_resultados_busqueda(res))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
            finally:
                self.root.after(0, lambda: btn.configure(text="Buscar", state="normal"))
                
        threading.Thread(target=_process, daemon=True).start()

    def _mostrar_resultados_busqueda(self, resultados):
        self.resultados_busqueda = resultados
        
        for w in self.scroll_buscar.winfo_children():
            w.destroy()
            
        for idx, c in enumerate(resultados):
            card = ctk.CTkFrame(self.scroll_buscar, fg_color=COLOR_BG_CARD, corner_radius=12)
            card.pack(fill='x', padx=10, pady=5)
            
            card.grid_columnconfigure(0, minsize=60) # Poster
            card.grid_columnconfigure(1, weight=1)   # Titulo
            card.grid_columnconfigure(2, minsize=100) # Badge
            card.grid_columnconfigure(3, minsize=160) # Accion
            
            poster_lbl = ctk.CTkLabel(card, text="", width=60, height=90, fg_color="#2B1A4A", corner_radius=8)
            poster_lbl.grid(row=0, column=0, padx=10, pady=10)
            
            if c.poster_url:
                def _load_img(url, lbl):
                    img = self._download_poster(url, size=(60, 90))
                    if img:
                        self.root.after(0, lambda: lbl.configure(image=img, text=""))
                threading.Thread(target=_load_img, args=(c.poster_url, poster_lbl), daemon=True).start()
            
            lbl_tit = ctk.CTkLabel(card, text=c.titulo, font=FONT_CARD, text_color=COLOR_TEXT, anchor="w", wraplength=200)
            lbl_tit.grid(row=0, column=1, sticky="w", padx=10, pady=12)
            
            badge_frame = ctk.CTkFrame(card, fg_color="transparent")
            badge_frame.grid(row=0, column=2, padx=10, pady=12)
            
            tipo_label = "Anime" if c.es_anime else ("Película" if c.tipo == TIPO_PELICULA else "Serie")
            badge = ui_styles.crear_badge_estado(badge_frame, tipo_label)
            badge.pack()
            
            action_frame = ctk.CTkFrame(card, fg_color="transparent")
            action_frame.grid(row=0, column=3, padx=15, pady=12, sticky="e")
            
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
                                             fg_color="#374151", button_color="#4B5563", button_hover_color="#6B7280", font=FONT_SUB)
                opt_menu.set(current_val)
                opt_menu.pack(side="right")
            else:
                opt_menu = ctk.CTkOptionMenu(action_frame, values=["+ Añadir a mi lista..."] + options, command=handle_action,
                                             fg_color="#10B981", button_color="#059669", button_hover_color="#047857", font=FONT_SUB)
                opt_menu.set("+ Añadir a mi lista...")
                opt_menu.pack(side="right")

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
                
                self.root.after(0, lambda: self.show_toast(f"'{c_db.titulo}' añadido para después."))
                self.root.after(0, self.refrescar_todo)
                self.root.after(0, lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
        threading.Thread(target=_process, daemon=True).start()
        
    def on_buscar_ya_vi(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                
                repository.actualizar_estado_usuario_contenido(uc.id, ESTADO_TERMINADA)
                repository.upsert_historial_recomendacion(c_db.id, "YA_LA_VI")
                
                self.root.after(0, lambda: self.show_toast(f"'{c_db.titulo}' marcada como vista."))
                self.root.after(0, self.refrescar_todo)
                self.root.after(0, lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
        threading.Thread(target=_process, daemon=True).start()
        
    def on_buscar_empezar(self, idx=None):
        if idx is None: return
        
        def _process():
            try:
                c_db = self._asegurar_contenido_bd(idx)
                uc = recommendation.agregar_a_biblioteca(c_db.id)
                
                progreso = repository.obtener_progreso_serie(uc.id)
                if progreso:
                    self.root.after(0, lambda: messagebox.showwarning("Aviso", "Ya estabas viendo esta serie."))
                else:
                    recommendation.empezar_serie(uc.id)
                    self.root.after(0, lambda: self.show_toast(f"¡Empezaste '{c_db.titulo}'! (T1 C1)"))
                self.root.after(0, self.refrescar_todo)
                self.root.after(0, lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
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
                self.root.after(0, lambda: self.show_toast(f"'{c_db.titulo}' terminada en T{p.temporada_actual} C{p.episodio_actual}"))
            except Exception as e:
                self.root.after(0, lambda: messagebox.showerror("Error", str(e)))
            finally:
                self.root.after(0, self.refrescar_todo)
                self.root.after(0, lambda: self._mostrar_resultados_busqueda(self.resultados_busqueda))
                
        threading.Thread(target=_process, daemon=True).start()

def run():
    init_db()
    root = ctk.CTk()
    app = QueVeoHoyApp(root)
    root.mainloop()

if __name__ == "__main__":
    run()
