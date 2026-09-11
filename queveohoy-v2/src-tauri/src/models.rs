use serde::{Deserialize, Serialize};

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
    pub sinopsis: Option<String>,
    pub fecha_estreno: Option<String>,
    pub creado_en: Option<String>,
    pub fetch_recently_failed: Option<bool>,
    pub generos: Option<String>, // JSON string: ["Acción", "Drama"]
}

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
}
