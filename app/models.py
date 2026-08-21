from dataclasses import dataclass
from typing import Optional

TIPO_SERIE = "TV"
TIPO_PELICULA = "MOVIE"

ESTADO_PARA_DESPUES = "pendiente"
ESTADO_EN_PROGRESO = "en_progreso"
ESTADO_PAUSADA = "pausada"
ESTADO_ABANDONADA = "abandonada"
ESTADO_TERMINADA = "terminada"
ESTADO_VISTA = "vista"

ESTADOS_LEGIBLES = {
    ESTADO_PARA_DESPUES: "Para después",
    ESTADO_EN_PROGRESO: "En progreso",
    ESTADO_PAUSADA: "Pausada",
    ESTADO_ABANDONADA: "Abandonada",
    ESTADO_TERMINADA: "Terminada",
    ESTADO_VISTA: "Vista"
}

@dataclass
class Contenido:
    id: Optional[int]
    tmdb_id: Optional[int]
    mal_id: Optional[int]
    tipo: str
    es_anime: bool
    titulo: str
    titulo_original: Optional[str]
    poster_url: Optional[str]
    sinopsis: Optional[str]
    fecha_estreno: Optional[str]
    creado_en: Optional[str]

@dataclass
class UsuarioContenido:
    id: Optional[int]
    contenido_id: int
    estado: str
    fecha_agregado: Optional[str]
    fecha_inicio: Optional[str]
    fecha_finalizacion: Optional[str]

@dataclass
class ProgresoSerie:
    id: Optional[int]
    usuario_contenido_id: int
    temporada_actual: int
    episodio_actual: int
    episodio_absoluto_actual: Optional[int]
    actualizado_en: Optional[str]

@dataclass
class EpisodioVisto:
    id: Optional[int]
    usuario_contenido_id: int
    tmdb_episode_id: Optional[int]
    temporada: int
    episodio: int
    episodio_absoluto: Optional[int]
    fecha_visto: Optional[str]
    origen: str

@dataclass
class HistorialRecomendacion:
    id: Optional[int]
    contenido_id: int
    fecha: Optional[str]
    accion: str
    veces_mostrada: int
