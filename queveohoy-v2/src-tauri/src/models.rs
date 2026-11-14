use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MiembroReparto {
    pub nombre: String,
    pub personaje: String,
    pub orden: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProveedorStreaming {
    pub id: i64,
    pub nombre: String,
    pub logo_path: Option<String>,
    pub tipo: String, // "flatrate" | "rent" | "buy"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlataformasStreaming {
    pub flatrate: Vec<ProveedorStreaming>,
    pub rent: Vec<ProveedorStreaming>,
    pub buy: Vec<ProveedorStreaming>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contenido {
    pub id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub tipo: String, // "TV" or "MOVIE"
    pub es_anime: bool,
    pub titulo: String,
    pub titulo_original: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub sinopsis: Option<String>,
    pub fecha_estreno: Option<String>,
    pub creado_en: Option<String>,
    pub fetch_recently_failed: Option<bool>,
    pub generos: Option<String>, // JSON string: ["Acción", "Drama"]
    pub director: Option<String>,
    pub creadores: Option<String>, // JSON string: ["Nombre 1", "Nombre 2"]
    pub reparto: Option<String>, // JSON string: [{"nombre": "...", "personaje": "...", "orden": 0}]
    pub total_temporadas: Option<i64>,
    pub plataformas: Option<String>, // JSON string: {"flatrate": [...], "rent": [...], "buy": [...]}
    pub providers_updated_at: Option<String>,
    pub status: Option<String>,
    pub last_air_date: Option<String>, // ISO datetime
    pub duracion: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsuarioContenido {
    pub id: Option<i64>,
    pub contenido_id: i64,
    pub estado: String,
    pub fecha_agregado: Option<String>,
    pub fecha_inicio: Option<String>,
    pub fecha_finalizacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgresoSerie {
    pub id: Option<i64>,
    pub usuario_contenido_id: i64,
    pub temporada_actual: i64,
    pub episodio_actual: i64,
    pub episodio_absoluto_actual: Option<i64>,
    pub actualizado_en: Option<String>,
    pub metadata_temporadas: Option<String>,
}

// Combined structure to send to frontend for the "Hoy" tab
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecomendacionHoy {
    pub contenido: Contenido,
    pub estado: Option<String>,
    pub progreso: Option<ProgresoSerie>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FiltrosRecomendacion {
    pub tipos: Option<Vec<String>>,
    pub generos: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AvanceResultado {
    pub temporada: i64,
    pub episodio: i64,
    pub terminada: bool,
    pub metadata_temporadas: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneroConteo {
    pub nombre: String,
    pub count: usize,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrupoContenido {
    PeliculasNoAnime,
    SeriesNoAnime,
    PeliculasAnime,
    SeriesAnime,
}

impl GrupoContenido {
    pub fn as_str(&self) -> &'static str {
        match self {
            GrupoContenido::PeliculasNoAnime => "peliculas_no_anime",
            GrupoContenido::SeriesNoAnime => "series_no_anime",
            GrupoContenido::PeliculasAnime => "peliculas_anime",
            GrupoContenido::SeriesAnime => "series_anime",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "peliculas_no_anime" => Some(GrupoContenido::PeliculasNoAnime),
            "series_no_anime" => Some(GrupoContenido::SeriesNoAnime),
            "peliculas_anime" => Some(GrupoContenido::PeliculasAnime),
            "series_anime" => Some(GrupoContenido::SeriesAnime),
            _ => None,
        }
    }

    pub fn db_filter(&self) -> (&'static str, i64) {
        match self {
            GrupoContenido::PeliculasNoAnime => ("MOVIE", 0),
            GrupoContenido::SeriesNoAnime => ("TV", 0),
            GrupoContenido::PeliculasAnime => ("MOVIE", 1),
            GrupoContenido::SeriesAnime => ("TV", 1),
        }
    }

    pub fn all() -> [Self; 4] {
        [
            GrupoContenido::PeliculasNoAnime,
            GrupoContenido::SeriesNoAnime,
            GrupoContenido::PeliculasAnime,
            GrupoContenido::SeriesAnime,
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ConteoPorGrupo {
    pub peliculas_no_anime: i64,
    pub series_no_anime: i64,
    pub peliculas_anime: i64,
    pub series_anime: i64,
    pub total: i64,
}

impl ConteoPorGrupo {
    pub fn get(&self, grupo: GrupoContenido) -> i64 {
        match grupo {
            GrupoContenido::PeliculasNoAnime => self.peliculas_no_anime,
            GrupoContenido::SeriesNoAnime => self.series_no_anime,
            GrupoContenido::PeliculasAnime => self.peliculas_anime,
            GrupoContenido::SeriesAnime => self.series_anime,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TmdbEndpointKind {
    Discover,
    TvDetail,
}

impl TmdbEndpointKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TmdbEndpointKind::Discover => "discover",
            TmdbEndpointKind::TvDetail => "tv_detail",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ReservaTmdbResultado {
    Reservada {
        reserva_id: i64,
        peticiones_en_ventana: i64,
        restante_en_ventana: i64,
    },
    PresupuestoAgotado {
        peticiones_en_ventana: i64,
        limite_aplicado: i64,
    },
    BloqueoActivo {
        bloqueado_hasta: i64,
        motivo: Option<String>,
    },
    IntervaloOperacionNoCumplido {
        ultimo_inicio_at: i64,
        proximo_disponible_at: i64,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct TmdbWorkerState {
    pub tmdb_bloqueado_hasta: Option<i64>,
    pub motivo_bloqueo: Option<String>,
    pub ultimo_error: Option<String>,
    pub errores_consecutivos: i64,
    pub ultima_operacion_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TmdbGroupCooldown {
    pub grupo: GrupoContenido,
    pub bloqueado_hasta: i64,
    pub fallos_sin_aporte: i64,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PosterRetryState {
    pub contenido_id: i64,
    pub url_fallida: String,
    pub intentos: i64,
    pub estado: String,
    pub proximo_reintento: Option<i64>,
    pub ultimo_error: Option<String>,
    pub actualizado_en: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct StockGrupoInfo {
    pub peliculas_no_anime: i64,
    pub series_no_anime: i64,
    pub peliculas_anime: i64,
    pub series_anime: i64,
    pub total_util: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResultadoBusqueda {
    pub tmdb_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub tipo: String, // "MOVIE" | "TV"
    pub es_anime: bool,
    pub titulo: String,
    pub titulo_original: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub sinopsis: Option<String>,
    pub fecha_estreno: Option<String>,
    pub generos: Option<String>, // JSON array string
    pub ya_en_biblioteca: bool,
    pub estado_actual: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObraBusquedaPayload {
    pub tmdb_id: Option<i64>,
    pub mal_id: Option<i64>,
    pub tipo: String,
    pub es_anime: bool,
    pub titulo: String,
    pub titulo_original: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub sinopsis: Option<String>,
    pub fecha_estreno: Option<String>,
    pub generos: Option<String>,
}
