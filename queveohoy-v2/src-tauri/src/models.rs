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
