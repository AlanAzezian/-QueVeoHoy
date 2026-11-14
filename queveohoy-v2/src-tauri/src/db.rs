use rusqlite::{Connection, Result, params};

pub const DIAS_EXCLUSION_SKIP: i64 = 10;
use std::path::PathBuf;
use crate::models::{Contenido, ProgresoSerie, RecomendacionHoy, ObraBusquedaPayload};
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct DbState {
    pub omitted_in_session: Mutex<HashSet<i64>>,
    pub omitted_en_progreso_session: Mutex<HashSet<i64>>,
    pub ultimo_reset_fecha: Mutex<String>,
}

pub fn get_db_path() -> PathBuf {
    let home_dir = dirs::home_dir()
        .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
        .expect("Could not find home directory");
    
    home_dir.join(".queveohoy.db")
}

pub fn get_cache_posters_dir() -> PathBuf {
    let home_dir = dirs::home_dir()
        .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
        .unwrap_or_else(|| PathBuf::from(""));

    home_dir.join(".queveohoy").join("cache_posters")
}

pub fn es_url_poster_valida(raw: &str) -> bool {
    let trimmed = raw.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with('/')
}

pub fn normalizar_poster_url_salida(raw: Option<String>) -> Option<String> {
    let es_http_o_https = |s: &str| -> bool {
        let bytes = s.as_bytes();
        (bytes.len() >= 7 && bytes[..7].eq_ignore_ascii_case(b"http://"))
            || (bytes.len() >= 8 && bytes[..8].eq_ignore_ascii_case(b"https://"))
    };

    let mut current = raw?.trim().to_string();
    if current.is_empty() {
        return None;
    }

    const TMDB_PREFIXES: [&str; 2] = [
        "https://image.tmdb.org/t/p/w500",
        "http://image.tmdb.org/t/p/w500",
    ];

    loop {
        let mut stripped = None;
        for prefix in TMDB_PREFIXES {
            if let Some(rest) = current.strip_prefix(prefix) {
                let rest_trimmed = rest.trim_start();
                if es_http_o_https(rest_trimmed) {
                    stripped = Some(rest_trimmed.to_string());
                    break;
                }
            }
        }
        match stripped {
            Some(next) => current = next,
            None => break,
        }
    }

    if current.is_empty() {
        return None;
    }
    if es_http_o_https(&current) {
        return Some(current);
    }
    if current.starts_with('/') {
        return Some(format!("https://image.tmdb.org/t/p/w500{}", current));
    }

    let before_slash = current.split('/').next().unwrap_or("");
    if let Some((scheme, _)) = before_slash.split_once(':') {
        let is_explicit_scheme = !scheme.is_empty()
            && scheme.bytes().enumerate().all(|(i, b)| {
                if i == 0 {
                    b.is_ascii_alphabetic()
                } else {
                    b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')
                }
            });
        if is_explicit_scheme || current.contains("://") {
            return None;
        }
    }

    Some(format!("https://image.tmdb.org/t/p/w500/{}", current))
}

pub fn resolve_poster_salida(
    cache_dir: &std::path::Path,
    id: i64,
    raw: Option<String>,
) -> Option<String> {
    let local_file = cache_dir.join(format!("{}.jpg", id));
    if local_file.exists() {
        return Some(local_file.to_string_lossy().to_string());
    }
    normalizar_poster_url_salida(raw)
}

pub fn get_connection() -> Result<Connection> {
    let db_path = get_db_path();
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

pub fn init_db() -> Result<()> {
    let conn = get_connection()?;
    init_db_conn(&conn)
}

pub fn init_db_conn(conn: &Connection) -> Result<()> {

    // Verificar columnas existentes
    let mut stmt = conn.prepare("PRAGMA table_info(contenido)")?;
    let mut rows = stmt.query([])?;
    let mut has_error_col = false;
    let mut has_generos_col = false;
    let mut has_backdrop_col = false;
    let mut has_director_col = false;
    let mut has_creadores_col = false;
    let mut has_reparto_col = false;
    let mut has_total_temporadas_col = false;
    let mut has_plataformas_col = false;
    let mut has_providers_updated_at_col = false;
    let mut has_status_col = false;
    let mut has_last_air_date_col = false;
    let mut has_duracion_col = false;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "last_metadata_fetch_error" { has_error_col = true; }
        if name == "generos" { has_generos_col = true; }
        if name == "backdrop_url" { has_backdrop_col = true; }
        if name == "director" { has_director_col = true; }
        if name == "creadores" { has_creadores_col = true; }
        if name == "reparto" { has_reparto_col = true; }
        if name == "total_temporadas" { has_total_temporadas_col = true; }
        if name == "plataformas" { has_plataformas_col = true; }
        if name == "providers_updated_at" { has_providers_updated_at_col = true; }
        if name == "status" { has_status_col = true; }
        if name == "last_air_date" { has_last_air_date_col = true; }
        if name == "duracion" { has_duracion_col = true; }
    }

    // Migración segura — last_metadata_fetch_error
    if !has_error_col {
        println!("Ejecutando migración: agregando last_metadata_fetch_error a contenido...");
        conn.execute("ALTER TABLE contenido ADD COLUMN last_metadata_fetch_error TEXT", [])?;
    }

    // Migración segura — generos
    if !has_generos_col {
        println!("Ejecutando migración: agregando generos a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN generos TEXT", []);
    }

    // Migración segura — backdrop_url
    if !has_backdrop_col {
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN backdrop_url TEXT", []);
    }

    // Migración segura — director, creadores, reparto, total_temporadas, plataformas, providers_updated_at
    if !has_director_col {
        println!("Ejecutando migración: agregando director a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN director TEXT", []);
    }
    if !has_creadores_col {
        println!("Ejecutando migración: agregando creadores a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN creadores TEXT", []);
    }
    if !has_reparto_col {
        println!("Ejecutando migración: agregando reparto a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN reparto TEXT", []);
    }
    if !has_total_temporadas_col {
        println!("Ejecutando migración: agregando total_temporadas a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN total_temporadas INTEGER", []);
    }
    if !has_plataformas_col {
        println!("Ejecutando migración: agregando plataformas a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN plataformas TEXT", []);
    }
    if !has_providers_updated_at_col {
        println!("Ejecutando migración: agregando providers_updated_at a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN providers_updated_at TEXT", []);
    }
    if !has_status_col {
        println!("Ejecutando migración: agregando status a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN status TEXT", []);
    }
    if !has_last_air_date_col {
        println!("Ejecutando migración: agregando last_air_date a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN last_air_date TEXT", []);
    }
    if !has_duracion_col {
        println!("Ejecutando migración: agregando duracion a contenido...");
        let _ = conn.execute("ALTER TABLE contenido ADD COLUMN duracion INTEGER", []);
    }

    // Nota Etapa 5.9.1B: Se retiró la purga automática de catalogo_offline por falta de géneros.
    // Una obra sin géneros puede participar de recomendaciones cuando no hay filtros de género activos.

    // Asegurar que catalogo_offline se limpie automáticamente
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS trg_limpiar_catalogo 
         AFTER INSERT ON usuario_contenido
         BEGIN
            DELETE FROM catalogo_offline WHERE contenido_id = NEW.contenido_id;
         END;",
        []
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS omitidos_sesion (
            contenido_id INTEGER NOT NULL,
            fecha TEXT NOT NULL,
            PRIMARY KEY (contenido_id, fecha)
        )",
        []
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS trg_limpiar_catalogo_upd 
         AFTER UPDATE ON usuario_contenido
         BEGIN
            DELETE FROM catalogo_offline WHERE contenido_id = NEW.contenido_id;
         END;",
        []
    )?;

    // Normalizar registros previos: 'AJUSTE_PROGRESO' -> 'EMPEZAR_EN_PROGRESO'
    let _ = conn.execute(
        "UPDATE historial_recomendaciones SET accion = 'EMPEZAR_EN_PROGRESO' WHERE accion = 'AJUSTE_PROGRESO'",
        []
    );

    // Etapa 5.9.2D-1: Tablas de persistencia para recuperador local, presupuesto TMDB y reintentos de pósteres
    conn.execute(
        "CREATE TABLE IF NOT EXISTS poster_retry_state (
            contenido_id INTEGER PRIMARY KEY REFERENCES contenido(id) ON DELETE CASCADE,
            url_fallida TEXT NOT NULL,
            intentos INTEGER NOT NULL DEFAULT 0,
            estado TEXT NOT NULL,
            proximo_reintento INTEGER,
            ultimo_error TEXT,
            actualizado_en INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tmdb_request_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            requested_at INTEGER NOT NULL,
            endpoint_kind TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tmdb_request_log_requested_at
         ON tmdb_request_log(requested_at)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tmdb_worker_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            tmdb_bloqueado_hasta INTEGER,
            motivo_bloqueo TEXT,
            ultimo_error TEXT,
            errores_consecutivos INTEGER NOT NULL DEFAULT 0,
            ultima_operacion_at INTEGER
        )",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tmdb_group_cooldown (
            grupo TEXT PRIMARY KEY CHECK (
                grupo IN ('peliculas_no_anime', 'series_no_anime', 'peliculas_anime', 'series_anime')
            ),
            bloqueado_hasta INTEGER NOT NULL DEFAULT 0,
            fallos_sin_aporte INTEGER NOT NULL DEFAULT 0,
            motivo TEXT
        )",
        [],
    )?;

    Ok(())
}

fn normalize_str(s: &str) -> String {
    s.to_lowercase()
     .chars()
     .map(|c| match c {
         'á' | 'à' | 'ä' | 'â' => 'a',
         'é' | 'è' | 'ë' | 'ê' => 'e',
         'í' | 'ì' | 'ï' | 'î' => 'i',
         'ó' | 'ò' | 'ö' | 'ô' => 'o',
         'ú' | 'ù' | 'ü' | 'û' => 'u',
         'ñ' => 'n',
         _ => c,
     })
     .collect()
}

fn parse_generos_vec(raw: Option<&str>) -> Vec<String> {
    let raw = match raw {
        Some(r) if !r.trim().is_empty() => r.trim(),
        _ => return Vec::new(),
    };
    if let Ok(vec) = serde_json::from_str::<Vec<String>>(raw) {
        return vec;
    }
    raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn match_tipo(tipo: &str, es_anime: bool, tipos: Option<&[String]>) -> bool {
    let tipos = match tipos {
        Some(t) if !t.is_empty() => t,
        _ => return true,
    };
    let has_pelis = tipos.iter().any(|t| t == "peliculas");
    let has_series = tipos.iter().any(|t| t == "series");
    let has_anime = tipos.iter().any(|t| t == "anime");

    if (!has_pelis && !has_series && !has_anime) || (has_pelis && has_series && has_anime) {
        return true;
    }

    let es_peli = tipo == "MOVIE";
    let es_serie = tipo == "TV";

    if has_pelis && !has_series && !has_anime {
        return es_peli && !es_anime;
    }
    if !has_pelis && has_series && !has_anime {
        return es_serie && !es_anime;
    }
    if !has_pelis && !has_series && has_anime {
        return es_anime;
    }
    if has_pelis && has_series && !has_anime {
        return (es_peli || es_serie) && !es_anime;
    }
    if has_pelis && !has_series && has_anime {
        return es_peli && es_anime;
    }
    if !has_pelis && has_series && has_anime {
        return es_serie && es_anime;
    }
    true
}

fn match_generos(raw_generos: Option<&str>, target_generos: Option<&[String]>) -> bool {
    let targets = match target_generos {
        Some(t) if !t.is_empty() => t,
        _ => return true,
    };
    let item_gens: Vec<String> = parse_generos_vec(raw_generos)
        .into_iter()
        .map(|g| normalize_str(&g))
        .collect();

    targets.iter().all(|target| {
        let norm_target = normalize_str(target);
        item_gens.iter().any(|g| g.contains(&norm_target) || norm_target.contains(g.as_str()))
    })
}

fn match_filtros(
    tipo: &str,
    es_anime: bool,
    generos: Option<&str>,
    tipos_filtro: Option<&[String]>,
    generos_filtro: Option<&[String]>,
) -> bool {
    match_tipo(tipo, es_anime, tipos_filtro) && match_generos(generos, generos_filtro)
}

pub fn get_recomendacion_de_hoy(state: &DbState, filtros: Option<crate::models::FiltrosRecomendacion>) -> Result<Option<RecomendacionHoy>> {
    let conn = get_connection()?;

    let (target_tipos, target_generos) = match &filtros {
        Some(f) => (f.tipos.as_deref(), f.generos.as_deref()),
        None => (None, None),
    };
    let has_active_filters = target_tipos.map(|t| !t.is_empty()).unwrap_or(false)
        || target_generos.map(|g| !g.is_empty()).unwrap_or(false);
    
    // 1. Verificar cambio de día calendario y resetear omit_clause
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    {
        let mut last_date = state.ultimo_reset_fecha.lock().unwrap();
        if *last_date != today_str {
            let mut omitted = state.omitted_in_session.lock().unwrap();
            if let Ok((nuevos_omitidos, _)) = get_omitidos_recientes(DIAS_EXCLUSION_SKIP) {
                *omitted = nuevos_omitidos;
            }
            let mut omitted_progreso = state.omitted_en_progreso_session.lock().unwrap();
            omitted_progreso.clear();
            
            *last_date = today_str;
        }
    }

    let omit_progreso_clause = {
        let omitted_en_progreso = state.omitted_en_progreso_session.lock().unwrap();
        let omitted_en_progreso_list = omitted_en_progreso.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        if omitted_en_progreso.is_empty() {
            "".to_string()
        } else {
            format!("AND c.id NOT IN ({})", omitted_en_progreso_list)
        }
    };

    let omit_clause = {
        let omitted = state.omitted_in_session.lock().unwrap();
        let omitted_list = omitted.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        if omitted.is_empty() {
            "".to_string()
        } else {
            format!("AND c.id NOT IN ({})", omitted_list)
        }
    };

    // 2. Try to find a series "en progreso" (respetando filtros activos y exclusiones)
    let mut stmt = conn.prepare(&format!(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date,
                c.duracion
         FROM contenido c
         JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         WHERE uc.estado = 'en_progreso' {} {}",
         omit_progreso_clause,
         omit_clause
    ))?;

    let mut in_progress_contents = Vec::new();
    let mut rows = stmt.query([])?;
    
    while let Some(row) = rows.next()? {
        let rec = map_recomendacion(&row)?;
        if match_filtros(
            &rec.contenido.tipo,
            rec.contenido.es_anime,
            rec.contenido.generos.as_deref(),
            target_tipos,
            target_generos,
        ) {
            in_progress_contents.push(rec);
        }
    }

    if !in_progress_contents.is_empty() {
        let mut rng = rand::thread_rng();
        let selected = in_progress_contents.choose(&mut rng).cloned();
        println!("[DEBUG Hoy] Recomendación encontrada en_progreso (con filtros): {:?}", selected);
        return Ok(selected);
    }

    // Helper closure to query available contents
    let get_available = |conn: &Connection| -> Result<Vec<Contenido>> {
        let mut stmt = conn.prepare(&format!(
            "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                    c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                    (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                    c.generos,
                    c.backdrop_url,
                    c.director,
                    c.creadores,
                    c.reparto,
                    c.total_temporadas,
                    c.plataformas,
                    c.providers_updated_at,
                    c.status,
                    c.last_air_date,
                    c.duracion
             FROM catalogo_offline co
             JOIN contenido c ON co.contenido_id = c.id
             LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
             WHERE uc.estado IS NULL {}",
             omit_clause
        ))?;
        
        let mut results = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let tipo_str: String = row.get(3)?;
            let es_anime: bool = row.get::<_, Option<i64>>(4)?.unwrap_or(0) == 1;
            let generos_str: Option<String> = row.get(12).ok().flatten();

            // Validación estricta de filtros (Tipo AND Géneros)
            if !match_filtros(
                &tipo_str,
                es_anime,
                generos_str.as_deref(),
                target_tipos,
                target_generos,
            ) {
                continue;
            }

            let raw_backdrop: Option<String> = row.get::<_, Option<String>>(13).ok().flatten();
            let backdrop_url = raw_backdrop.map(|bp| {
                if bp.starts_with("http://") || bp.starts_with("https://") {
                    bp
                } else if bp.starts_with('/') {
                    format!("https://image.tmdb.org/t/p/w1280{}", bp)
                } else {
                    format!("https://image.tmdb.org/t/p/w1280/{}", bp)
                }
            });
            let director: Option<String> = row.get::<_, Option<String>>(14).ok().flatten();
            let creadores: Option<String> = row.get::<_, Option<String>>(15).ok().flatten();
            let reparto: Option<String> = row.get::<_, Option<String>>(16).ok().flatten();
            let total_temporadas: Option<i64> = row.get::<_, Option<i64>>(17).ok().flatten();
            let plataformas: Option<String> = row.get::<_, Option<String>>(18).ok().flatten();
            let providers_updated_at: Option<String> = row.get::<_, Option<String>>(19).ok().flatten();
            let status: Option<String> = row.get::<_, Option<String>>(20).ok().flatten();
            let last_air_date: Option<String> = row.get::<_, Option<String>>(21).ok().flatten();
            let duracion: Option<i64> = row.get::<_, Option<i64>>(22).ok().flatten();

            let raw_poster: Option<String> = row.get(7)?;
            let cache_dir = get_cache_posters_dir();
            let poster_url = resolve_poster_salida(&cache_dir, id, raw_poster);

            results.push(Contenido {
                id: Some(id),
                tmdb_id: row.get(1)?,
                mal_id: row.get(2)?,
                tipo: tipo_str,
                es_anime,
                titulo: row.get(5)?,
                titulo_original: row.get(6)?,
                poster_url,
                backdrop_url,
                sinopsis: row.get(8)?,
                fecha_estreno: row.get(9)?,
                creado_en: row.get(10)?,
                fetch_recently_failed: row.get::<_, Option<i64>>(11).ok().flatten().map(|v| v == 1),
                generos: generos_str,
                director,
                creadores,
                reparto,
                total_temporadas,
                plataformas,
                providers_updated_at,
                status,
                last_air_date,
                duracion,
            });
        }
        Ok(results)
    };

    // 3. Query catalogo_offline with omit and filters
    let available_contents = get_available(&conn)?;

    // Comprobar disponibilidad real: si el stock disponible (no omitido y virgen) es < 15, despertar worker
    let raw_virgin_count = {
        let mut stmt = conn.prepare(&format!(
            "SELECT COUNT(*) FROM catalogo_offline co JOIN contenido c ON co.contenido_id = c.id LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id WHERE uc.estado IS NULL {}",
            omit_clause
        ))?;
        stmt.query_row([], |r| r.get::<_, usize>(0)).unwrap_or(0)
    };
    if raw_virgin_count < 15 {
        println!("[DEBUG Hoy] Stock disponible no omitido es {} (< 15), despertando worker...", raw_virgin_count);
        crate::discovery::trigger_discovery();
    }

    // Si no hay contenidos disponibles (por filtros restrictivos o catálogo agotado), devolvemos None de manera controlada sin borrar descartes
    if available_contents.is_empty() {
        if has_active_filters {
            println!("[DEBUG Hoy] No hay contenidos que coincidan con los filtros activos. Devolviendo None.");
        } else {
            println!("[DEBUG Hoy] catalogo_offline no tiene obras disponibles vigentes (agotadas o descartadas). Devolviendo None.");
        }
        return Ok(None);
    }

    // Pick random
    let mut rng = rand::thread_rng();
    let selected = available_contents.choose(&mut rng).cloned();
    
    let res = selected.map(|c| {
        RecomendacionHoy {
            contenido: c,
            estado: None,
            progreso: None,
        }
    });
    println!("[DEBUG Hoy] Recomendación encontrada en catalogo_offline: {:?}", res);
    Ok(res)
}

fn map_recomendacion(row: &rusqlite::Row) -> rusqlite::Result<RecomendacionHoy> {
    let id: i64 = row.get(0)?;
    let raw_poster: Option<String> = row.get(7)?;
    let cache_dir = get_cache_posters_dir();
    let poster_url = resolve_poster_salida(&cache_dir, id, raw_poster);

    let raw_backdrop: Option<String> = row.get::<usize, Option<String>>(20).ok().flatten();
    let backdrop_url = raw_backdrop.map(|bp| {
        if bp.starts_with("http://") || bp.starts_with("https://") {
            bp
        } else if bp.starts_with('/') {
            format!("https://image.tmdb.org/t/p/w1280{}", bp)
        } else {
            format!("https://image.tmdb.org/t/p/w1280/{}", bp)
        }
    });

    let contenido = Contenido {
        id: Some(id),
        tmdb_id: row.get(1)?,
        mal_id: row.get(2)?,
        tipo: row.get(3)?,
        es_anime: row.get::<_, Option<i64>>(4)?.unwrap_or(0) == 1,
        titulo: row.get(5)?,
        titulo_original: row.get(6)?,
        poster_url,
        backdrop_url,
        sinopsis: row.get(8)?,
        fecha_estreno: row.get(9)?,
        creado_en: row.get(10)?,
        fetch_recently_failed: row.get::<usize, Option<i64>>(18).ok().flatten().map(|v| v == 1),
        generos: row.get::<usize, Option<String>>(19).ok().flatten(),
        director: row.get::<usize, Option<String>>(21).ok().flatten(),
        creadores: row.get::<usize, Option<String>>(22).ok().flatten(),
        reparto: row.get::<usize, Option<String>>(23).ok().flatten(),
        total_temporadas: row.get::<usize, Option<i64>>(24).ok().flatten(),
        plataformas: row.get::<usize, Option<String>>(25).ok().flatten(),
        providers_updated_at: row.get::<usize, Option<String>>(26).ok().flatten(),
        status: row.get::<usize, Option<String>>(27).ok().flatten(),
        last_air_date: row.get::<usize, Option<String>>(28).ok().flatten(),
        duracion: row.get::<usize, Option<i64>>(29).ok().flatten(),
    };
    
    let uc_id: Option<i64> = row.get::<_, Option<i64>>(11).ok().flatten();
    let estado: Option<String> = row.get::<_, Option<String>>(12).ok().flatten();
    
    let progreso = if let Some(ucid) = uc_id {
        if let Ok(ps_id) = row.get::<_, i64>(13) {
            Some(ProgresoSerie {
                id: Some(ps_id),
                usuario_contenido_id: ucid,
                temporada_actual: row.get(14)?,
                episodio_actual: row.get(15)?,
                episodio_absoluto_actual: row.get(16)?,
                actualizado_en: None,
                metadata_temporadas: row.get::<_, Option<String>>(17).ok().flatten(),
            })
        } else {
            None
        }
    } else {
        None
    };
    
    Ok(RecomendacionHoy {
        contenido,
        estado,
        progreso,
    })
}

pub fn get_en_progreso() -> Result<Vec<RecomendacionHoy>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date,
                c.duracion
         FROM contenido c
         JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         WHERE uc.estado = 'en_progreso'
         ORDER BY uc.fecha_inicio DESC"
    )?;

    let mut items = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        items.push(map_recomendacion(row)?);
    }
    
    Ok(items)
}

pub fn get_biblioteca() -> Result<Vec<RecomendacionHoy>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date,
                c.duracion
         FROM contenido c
         JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         WHERE uc.estado IN ('pendiente', 'guardado')
         ORDER BY uc.fecha_inicio DESC"
    )?;

    let mut items = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        items.push(map_recomendacion(row)?);
    }
    Ok(items)
}

pub fn get_terminadas() -> Result<Vec<RecomendacionHoy>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date,
                c.duracion
         FROM contenido c
         JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         WHERE uc.estado = 'terminada'
         ORDER BY uc.fecha_finalizacion DESC"
    )?;

    let mut items = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        items.push(map_recomendacion(row)?);
    }
    Ok(items)
}

pub fn get_historial() -> Result<Vec<RecomendacionHoy>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, hr.fecha,
                COALESCE(uc.id, 0) as uc_id, hr.accion,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date
         FROM (
             SELECT contenido_id, accion, MAX(fecha) as fecha
             FROM historial_recomendaciones
             GROUP BY contenido_id
         ) hr
         JOIN contenido c ON hr.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         ORDER BY hr.fecha DESC
         LIMIT 100"
    )?;

    let mut items = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        items.push(map_recomendacion(row)?);
    }
    
    Ok(items)
}

pub fn registrar_accion(contenido_id: i64, accion: &str) -> Result<()> {
    let conn = get_connection()?;
    conn.execute(
        "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
         VALUES (?1, datetime('now', 'localtime'), ?2, 1)
         ON CONFLICT(contenido_id) DO UPDATE SET
            fecha = datetime('now', 'localtime'),
            accion = excluded.accion,
            veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
        params![contenido_id, accion]
    )?;
    Ok(())
}

pub fn update_estado(contenido_id: i64, nuevo_estado: &str) -> Result<()> {
    let mut conn = get_connection()?;
    let tx = conn.transaction()?;
    
    // Check if it exists in usuario_contenido
    let mut stmt = tx.prepare("SELECT id FROM usuario_contenido WHERE contenido_id = ?1")?;
    let exists = stmt.exists(params![contenido_id])?;
    drop(stmt);
    
    if exists {
        if nuevo_estado == "terminada" {
            tx.execute(
                "UPDATE usuario_contenido SET estado = ?1, fecha_finalizacion = datetime('now', 'localtime') WHERE contenido_id = ?2",
                params![nuevo_estado, contenido_id]
            )?;
        } else {
            tx.execute(
                "UPDATE usuario_contenido SET estado = ?1, fecha_inicio = datetime('now', 'localtime') WHERE contenido_id = ?2",
                params![nuevo_estado, contenido_id]
            )?;
        }
    } else {
        if nuevo_estado == "terminada" {
            tx.execute(
                "INSERT INTO usuario_contenido (contenido_id, estado, fecha_finalizacion) VALUES (?1, ?2, datetime('now', 'localtime'))",
                params![contenido_id, nuevo_estado]
            )?;
        } else {
            tx.execute(
                "INSERT INTO usuario_contenido (contenido_id, estado) VALUES (?1, ?2)",
                params![contenido_id, nuevo_estado]
            )?;
        }
    }
    
    // If it's en_progreso and a series, ensure progreso_serie exists
    if nuevo_estado == "en_progreso" {
        // We need the uc_id
        let uc_id: i64 = tx.query_row(
            "SELECT id FROM usuario_contenido WHERE contenido_id = ?1",
            params![contenido_id],
            |row| row.get(0)
        )?;
        
        let tipo: String = tx.query_row(
            "SELECT tipo FROM contenido WHERE id = ?1",
            params![contenido_id],
            |row| row.get(0)
        )?;
        
        if tipo != "MOVIE" {
            let mut stmt = tx.prepare("SELECT id FROM progreso_serie WHERE usuario_contenido_id = ?1")?;
            let has_ps = stmt.exists(params![uc_id])?;
            drop(stmt);
            if !has_ps {
                tx.execute(
                    "INSERT INTO progreso_serie (usuario_contenido_id, temporada_actual, episodio_actual, episodio_absoluto_actual) VALUES (?1, 1, 1, 1)",
                    params![uc_id]
                )?;
            }
        }
    }
    
    let accion = match nuevo_estado {
        "terminada" => "YA_LA_VI",
        "pendiente" => "PARA_DESPUES",
        "en_progreso" => "EMPEZAR_EN_PROGRESO",
        "abandonada" => "ABANDONADA",
        _ => "EMPEZAR_EN_PROGRESO"
    };
    
    tx.execute(
        "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
         VALUES (?1, datetime('now', 'localtime'), ?2, 1)
         ON CONFLICT(contenido_id) DO UPDATE SET
            fecha = datetime('now', 'localtime'),
            accion = excluded.accion,
            veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
        params![contenido_id, accion]
    )?;

    tx.commit()?;
    Ok(())
}

pub fn actualizar_progreso(contenido_id: i64, temporada: i64, episodio: i64) -> Result<()> {
    let mut conn = get_connection()?;
    let tx = conn.transaction()?;
    internal_actualizar_progreso(&tx, contenido_id, temporada, episodio, "EMPEZAR_EN_PROGRESO")?;
    tx.commit()?;
    Ok(())
}

fn internal_actualizar_progreso(conn: &Connection, contenido_id: i64, temporada: i64, episodio: i64, fallback_accion: &str) -> Result<bool> {
    // First, ensure the item is in 'en_progreso' state
    let uc_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM usuario_contenido WHERE contenido_id = ?1)",
        params![contenido_id],
        |row| row.get(0)
    )?;

    let uc_id: i64 = if uc_exists {
        conn.execute(
            "UPDATE usuario_contenido SET estado = 'en_progreso' WHERE contenido_id = ?1",
            params![contenido_id]
        )?;
        conn.query_row(
            "SELECT id FROM usuario_contenido WHERE contenido_id = ?1",
            params![contenido_id],
            |row| row.get(0)
        )?
    } else {
        conn.execute(
            "INSERT INTO usuario_contenido (contenido_id, estado, fecha_inicio) VALUES (?1, 'en_progreso', datetime('now', 'localtime'))",
            params![contenido_id]
        )?;
        conn.last_insert_rowid()
    };

    // Now upsert into progreso_serie
    let ps_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM progreso_serie WHERE usuario_contenido_id = ?1)",
        params![uc_id],
        |row| row.get(0)
    )?;

    if ps_exists {
        conn.execute(
            "UPDATE progreso_serie SET temporada_actual = ?1, episodio_actual = ?2 WHERE usuario_contenido_id = ?3",
            params![temporada, episodio, uc_id]
        )?;
    } else {
        conn.execute(
            "INSERT INTO progreso_serie (usuario_contenido_id, temporada_actual, episodio_actual, episodio_absoluto_actual) VALUES (?1, ?2, ?3, ?3)",
            params![uc_id, temporada, episodio]
        )?;
    }

    // Check auto-completion
    let is_last_season_and_episode: bool = conn.query_row(
        "SELECT 
            (SELECT MAX(season_number) FROM tv_temporadas WHERE contenido_id = ?1) = ?2
         AND
            (SELECT COUNT(*) FROM tv_episodios te 
             JOIN tv_temporadas tt ON te.temporada_id = tt.id 
             WHERE tt.contenido_id = ?1 AND tt.season_number = ?2) = ?3",
        params![contenido_id, temporada, episodio],
        |row| row.get::<_, Option<bool>>(0).map(|b| b.unwrap_or(false))
    ).unwrap_or(false);

    if is_last_season_and_episode {
        conn.execute(
            "UPDATE usuario_contenido SET estado = 'terminada', fecha_finalizacion = datetime('now', 'localtime') WHERE id = ?1",
            params![uc_id]
        )?;
        
        conn.execute(
            "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
             VALUES (?1, datetime('now', 'localtime'), 'YA_LA_VI', 1)
             ON CONFLICT(contenido_id) DO UPDATE SET
                fecha = datetime('now', 'localtime'),
                accion = 'YA_LA_VI',
                veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
            params![contenido_id]
        )?;
    } else {
        conn.execute(
            "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
             VALUES (?1, datetime('now', 'localtime'), ?2, 1)
             ON CONFLICT(contenido_id) DO UPDATE SET
                fecha = datetime('now', 'localtime'),
                accion = excluded.accion,
                veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
            params![contenido_id, fallback_accion]
        )?;
    }
    
    Ok(is_last_season_and_episode)
}

pub fn avanzar_episodio(contenido_id: i64) -> Result<crate::models::AvanceResultado> {
    let mut conn = get_connection()?;
    
    // 1. Get metadata from 'contenido' for fetch fallback if needed
    let (tmdb_id, mal_id, es_anime, tipo): (Option<i64>, Option<i64>, bool, String) = conn.query_row(
        "SELECT tmdb_id, mal_id, es_anime, tipo FROM contenido WHERE id = ?1",
        params![contenido_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? == 1, row.get(3)?))
    )?;

    let mut proceed_strict = false;

    // Only apply complex logic for TV/Anime
    if tipo == "TV" || es_anime {
        // 2. Verify if we have metadata BEFORE opening the transaction
        let has_metadata: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tv_temporadas WHERE contenido_id = ?1",
            params![contenido_id],
            |row| row.get(0)
        ).unwrap_or(0);

        if has_metadata == 0 {
            // Fallback: try to fetch
            match tauri::async_runtime::block_on(crate::tmdb::fetch_and_cache_temporadas(contenido_id, tmdb_id, mal_id, es_anime)) {
                Ok(_) => {
                    let check_metadata: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM tv_temporadas WHERE contenido_id = ?1",
                        params![contenido_id],
                        |row| row.get(0)
                    ).unwrap_or(0);
                    
                    if check_metadata > 0 {
                        proceed_strict = true;
                    }
                },
                Err(e) => {
                    eprintln!("Fallo al realizar fetch_and_cache_temporadas en avanzar_episodio (Error: {}), se usara modo optimista.", e);
                }
            }
        } else {
            proceed_strict = true;
        }
    }

    // 3. NOW open the transaction (so its read snapshot includes the newly fetched metadata)
    let tx = conn.transaction()?;

    // 4. Fetch current season and episode
    let (mut temp_actual, mut ep_actual): (i64, i64) = tx.query_row(
        "SELECT ps.temporada_actual, ps.episodio_actual 
         FROM progreso_serie ps 
         JOIN usuario_contenido uc ON ps.usuario_contenido_id = uc.id 
         WHERE uc.contenido_id = ?1",
        params![contenido_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).unwrap_or((1, 0)); // If not found, start at S1 E0, so +1 gives S1 E1

    if tipo != "TV" && !es_anime {
        let is_terminada = internal_actualizar_progreso(&tx, contenido_id, temp_actual, ep_actual + 1, "EMPEZAR_EN_PROGRESO")?;
        tx.commit()?;
        return Ok(crate::models::AvanceResultado { temporada: temp_actual, episodio: ep_actual + 1, terminada: is_terminada, metadata_temporadas: None });
    }

    let logged_action = "EMPEZAR_EN_PROGRESO";
    let is_terminada;

    if proceed_strict {
        // strict logic
        let count_episodes: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tv_episodios te 
             JOIN tv_temporadas tt ON te.temporada_id = tt.id 
             WHERE tt.contenido_id = ?1 AND tt.season_number = ?2",
            params![contenido_id, temp_actual],
            |row| row.get(0)
        ).unwrap_or(0);

        if ep_actual + 1 <= count_episodes {
            ep_actual += 1;
        } else {
            // Find next season
            let next_season_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM tv_temporadas WHERE contenido_id = ?1 AND season_number = ?2)",
                params![contenido_id, temp_actual + 1],
                |row| row.get(0)
            ).unwrap_or(false);

            if next_season_exists {
                temp_actual += 1;
                ep_actual = 1;
            } else {
                ep_actual += 1; // It will trigger auto-completion inside internal_actualizar_progreso
            }
        }
        
        is_terminada = internal_actualizar_progreso(&tx, contenido_id, temp_actual, ep_actual, logged_action)?;
    } else {
        // Optimistic
        ep_actual += 1;
        is_terminada = internal_actualizar_progreso(&tx, contenido_id, temp_actual, ep_actual, logged_action)?;
    }

    let metadata_temporadas: Option<String> = tx.query_row(
        "SELECT json_group_object(
            CAST(tt.season_number AS TEXT),
            (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
         )
         FROM tv_temporadas tt
         WHERE tt.contenido_id = ?1",
        params![contenido_id],
        |row| row.get(0)
    ).ok();
    tx.commit()?;

    Ok(crate::models::AvanceResultado {
        temporada: temp_actual,
        episodio: ep_actual,
        terminada: is_terminada,
        metadata_temporadas
    })
}

pub fn omitir_todos_en_progreso(state: &DbState) -> Result<()> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare("SELECT c.id FROM contenido c JOIN usuario_contenido uc ON c.id = uc.contenido_id WHERE uc.estado = 'en_progreso'")?;
    let mut rows = stmt.query([])?;
    let mut omitted = state.omitted_en_progreso_session.lock().unwrap();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    Ok(())
}

pub fn abandonar_multiples(ids: Vec<i64>) -> Result<()> {
    if ids.is_empty() { return Ok(()); }
    let mut conn = get_connection()?;
    let tx = conn.transaction()?;
    
    // Actualizar estados por lote
    for id in &ids {
        tx.execute(
            "UPDATE usuario_contenido SET estado = 'abandonada' WHERE contenido_id = ?1",
            params![id]
        )?;
        
        tx.execute(
            "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
             VALUES (?1, datetime('now', 'localtime'), 'ABANDONADA', 1)
             ON CONFLICT(contenido_id) DO UPDATE SET
                fecha = datetime('now', 'localtime'),
                accion = excluded.accion,
                veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
            params![id]
        )?;
    }
    
    tx.commit()?;
    Ok(())
}

pub fn get_omitidos_recientes(dias: i64) -> Result<(std::collections::HashSet<i64>, String)> {
    let conn = get_connection()?;
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    let threshold_date: String = conn.query_row("SELECT date('now', 'localtime', ?1)", params![format!("-{} days", dias)], |row| row.get(0))?;
    
    // Purgar exclusiones vencidas (más antiguas que el período establecido de 10 días)
    let _ = conn.execute("DELETE FROM omitidos_sesion WHERE fecha < ?1", params![threshold_date]);

    let mut stmt = conn.prepare("SELECT DISTINCT contenido_id FROM omitidos_sesion WHERE fecha >= ?1")?;
    let mut rows = stmt.query(params![threshold_date])?;
    
    let mut omitted = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        omitted.insert(id);
    }
    
    Ok((omitted, today_str))
}

pub fn add_omitido_sesion(contenido_id: i64) -> Result<()> {
    let conn = get_connection()?;
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    conn.execute(
        "INSERT OR IGNORE INTO omitidos_sesion (contenido_id, fecha) VALUES (?1, ?2)",
        params![contenido_id, today_str]
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_estado_contenido(id: i64) -> Result<String> {
    let conn = get_connection()?;
    let estado: String = conn.query_row(
        "SELECT estado FROM usuario_contenido WHERE contenido_id = ?1",
        rusqlite::params![id],
        |row| row.get(0)
    ).unwrap_or_else(|_| "".to_string());
    Ok(estado)
}

pub fn get_metadata_fetch_params(id: i64) -> Result<(Option<i64>, Option<i64>, bool)> {
    let conn = get_connection()?;
    conn.query_row(
        "SELECT tmdb_id, mal_id, es_anime FROM contenido WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? == 1))
    )
}

pub fn get_metadata_json(contenido_id: i64) -> Result<Option<String>> {
    let conn = get_connection()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tv_temporadas WHERE contenido_id = ?1",
        rusqlite::params![contenido_id],
        |row| row.get(0)
    ).unwrap_or(0);
    
    if count == 0 {
        return Ok(None);
    }
    
    let json_str: Option<String> = conn.query_row(
        "SELECT json_group_object(
             CAST(season_number AS TEXT),
             (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
         )
         FROM tv_temporadas tt WHERE tt.contenido_id = ?1",
        rusqlite::params![contenido_id],
        |row| row.get(0)
    ).unwrap_or(None);
    Ok(json_str)
}

pub fn get_generos_disponibles_hoy() -> Result<Vec<crate::models::GeneroConteo>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.generos 
         FROM catalogo_offline co
         JOIN contenido c ON co.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE uc.estado IS NULL AND c.generos IS NOT NULL AND c.generos != ''"
    )?;

    let mut rows = stmt.query([])?;
    let mut genre_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    while let Some(row) = rows.next()? {
        let generos_str: String = row.get(0)?;
        
        // Tratar de parsear como JSON array
        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&generos_str) {
            for g in parsed {
                *genre_counts.entry(g).or_insert(0) += 1;
            }
        } else {
            // Fallback a separador por comas
            for g in generos_str.split(',') {
                let g = g.trim();
                if !g.is_empty() {
                    *genre_counts.entry(g.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut result: Vec<crate::models::GeneroConteo> = genre_counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(nombre, count)| crate::models::GeneroConteo { nombre, count })
        .collect();

    // Ordenar de mayor a menor cantidad
    result.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(result)
}

pub fn get_stock_util_por_grupo() -> Result<crate::models::StockGrupoInfo> {
    let conn = get_connection()?;
    get_stock_util_por_grupo_conn(&conn)
}

pub fn get_stock_util_por_grupo_conn(conn: &Connection) -> Result<crate::models::StockGrupoInfo> {
    let threshold_param = format!("-{} days", DIAS_EXCLUSION_SKIP);
    let mut stmt = conn.prepare(
        "SELECT
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0) AS peliculas_no_anime,
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0) AS series_no_anime,
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0) AS peliculas_anime,
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0) AS series_anime,
            COUNT(c.id) AS total_util
        FROM catalogo_offline co
        JOIN contenido c ON co.contenido_id = c.id
        LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
        WHERE uc.estado IS NULL
          AND c.id NOT IN (
              SELECT contenido_id FROM omitidos_sesion 
              WHERE fecha >= date('now', 'localtime', ?1)
          )"
    )?;
    
    let info = stmt.query_row(params![threshold_param], |row| {
        Ok(crate::models::StockGrupoInfo {
            peliculas_no_anime: row.get(0)?,
            series_no_anime: row.get(1)?,
            peliculas_anime: row.get(2)?,
            series_anime: row.get(3)?,
            total_util: row.get(4)?,
        })
    })?;

    Ok(info)
}

pub fn prune_catalogo_offline() -> Result<usize> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.contenido_id FROM catalogo_offline c
         WHERE c.fecha_agregado < datetime('now', '-15 days')
         AND c.contenido_id NOT IN (SELECT contenido_id FROM usuario_contenido)"
    )?;
    let ids_to_prune: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);

    let rows_affected = conn.execute(
        "DELETE FROM catalogo_offline
         WHERE contenido_id IN (
             SELECT c.contenido_id FROM catalogo_offline c
             WHERE c.fecha_agregado < datetime('now', '-15 days')
             AND c.contenido_id NOT IN (SELECT contenido_id FROM usuario_contenido)
         )",
        []
    )?;

    if !ids_to_prune.is_empty() {
        let cache_dir = get_cache_posters_dir();
        for id in ids_to_prune {
            let referenced: bool = conn
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM catalogo_offline WHERE contenido_id = ?1
                        UNION ALL
                        SELECT 1 FROM usuario_contenido WHERE contenido_id = ?1
                    )",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !referenced {
                let poster_path = cache_dir.join(format!("{}.jpg", id));
                if poster_path.exists() {
                    if let Err(e) = std::fs::remove_file(&poster_path) {
                        eprintln!("[GC] Advertencia: no se pudo eliminar póster huérfano {:?}: {}", poster_path, e);
                    }
                }
            }
        }
    }

    Ok(rows_affected)
}

pub fn get_recomendacion_by_contenido_id(contenido_id: i64) -> Result<RecomendacionHoy> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-12 hours')) as fetch_recently_failed,
                c.generos,
                c.backdrop_url,
                c.director,
                c.creadores,
                c.reparto,
                c.total_temporadas,
                c.plataformas,
                c.providers_updated_at,
                c.status,
                c.last_air_date,
                c.duracion
         FROM contenido c
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         LEFT JOIN progreso_serie ps ON uc.id = ps.usuario_contenido_id
         LEFT JOIN (
            SELECT tt.contenido_id,
                   json_group_object(
                       CAST(tt.season_number AS TEXT),
                       (SELECT COUNT(*) FROM tv_episodios te WHERE te.temporada_id = tt.id)
                   ) as temporadas_json
            FROM tv_temporadas tt
            GROUP BY tt.contenido_id
         ) tm ON c.id = tm.contenido_id
         WHERE c.id = ?1"
    )?;
    stmt.query_row(params![contenido_id], map_recomendacion)
}

pub async fn guardar_obra_busqueda(obra: ObraBusquedaPayload, estado: String) -> std::result::Result<RecomendacionHoy, String> {
    let contenido_id = {
        let mut conn = get_connection().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let existing_id: Option<i64> = tx.query_row(
            "SELECT id FROM contenido WHERE (?1 IS NOT NULL AND tmdb_id = ?1) OR (?2 IS NOT NULL AND mal_id = ?2) LIMIT 1",
            params![obra.tmdb_id, obra.mal_id],
            |row| row.get(0)
        ).ok();

        let id = match existing_id {
            Some(cid) => {
                if let Some(gens) = &obra.generos {
                    let _ = tx.execute(
                        "UPDATE contenido SET generos = ?1 WHERE id = ?2 AND (generos IS NULL OR generos = '')",
                        params![gens, cid]
                    );
                }
                if let Some(b_url) = &obra.backdrop_url {
                    let _ = tx.execute(
                        "UPDATE contenido SET backdrop_url = ?1 WHERE id = ?2 AND (backdrop_url IS NULL OR backdrop_url = '')",
                        params![b_url, cid]
                    );
                }
                cid
            }
            None => {
                tx.execute(
                    "INSERT INTO contenido (tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos, backdrop_url)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        obra.tmdb_id,
                        obra.mal_id,
                        obra.tipo,
                        if obra.es_anime { 1 } else { 0 },
                        obra.titulo,
                        obra.titulo_original,
                        obra.sinopsis,
                        obra.fecha_estreno,
                        obra.poster_url,
                        obra.generos,
                        obra.backdrop_url
                    ]
                ).map_err(|e| format!("Error al insertar contenido: {}", e))?;
                tx.last_insert_rowid()
            }
        };

        let uc_exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM usuario_contenido WHERE contenido_id = ?1)",
            params![id],
            |row| row.get(0)
        ).unwrap_or(false);

        if uc_exists {
            if estado == "terminada" {
                tx.execute(
                    "UPDATE usuario_contenido SET estado = ?1, fecha_finalizacion = datetime('now', 'localtime') WHERE contenido_id = ?2",
                    params![estado, id]
                ).map_err(|e| format!("Error actualizando usuario_contenido: {}", e))?;
            } else {
                tx.execute(
                    "UPDATE usuario_contenido SET estado = ?1, fecha_inicio = datetime('now', 'localtime') WHERE contenido_id = ?2",
                    params![estado, id]
                ).map_err(|e| format!("Error actualizando usuario_contenido: {}", e))?;
            }
        } else {
            if estado == "terminada" {
                tx.execute(
                    "INSERT INTO usuario_contenido (contenido_id, estado, fecha_finalizacion) VALUES (?1, ?2, datetime('now', 'localtime'))",
                    params![id, estado]
                ).map_err(|e| format!("Error insertando usuario_contenido: {}", e))?;
            } else {
                tx.execute(
                    "INSERT INTO usuario_contenido (contenido_id, estado, fecha_inicio) VALUES (?1, ?2, datetime('now', 'localtime'))",
                    params![id, estado]
                ).map_err(|e| format!("Error insertando usuario_contenido: {}", e))?;
            }
        }

        if estado == "en_progreso" && (obra.tipo == "TV" || obra.es_anime) {
            let uc_id: i64 = tx.query_row(
                "SELECT id FROM usuario_contenido WHERE contenido_id = ?1",
                params![id],
                |row| row.get(0)
            ).map_err(|e| format!("Error obteniendo uc_id: {}", e))?;

            let mut stmt = tx.prepare("SELECT id FROM progreso_serie WHERE usuario_contenido_id = ?1")
                .map_err(|e| format!("Error preparando select progreso_serie: {}", e))?;
            let has_ps = stmt.exists(params![uc_id])
                .map_err(|e| format!("Error comprobando existencia en progreso_serie: {}", e))?;
            drop(stmt);

            if !has_ps {
                tx.execute(
                    "INSERT INTO progreso_serie (usuario_contenido_id, temporada_actual, episodio_actual, episodio_absoluto_actual) VALUES (?1, 1, 1, 1)",
                    params![uc_id]
                ).map_err(|e| format!("Error insertando progreso_serie: {}", e))?;
            }
        }

        let accion = match estado.as_str() {
            "terminada" => "YA_LA_VI",
            "pendiente" => "PARA_DESPUES",
            "en_progreso" => "EMPEZAR_EN_PROGRESO",
            "abandonada" => "ABANDONADA",
            _ => "EMPEZAR_EN_PROGRESO"
        };

        tx.execute(
            "INSERT INTO historial_recomendaciones (contenido_id, fecha, accion, veces_mostrada)
             VALUES (?1, datetime('now', 'localtime'), ?2, 1)
             ON CONFLICT(contenido_id) DO UPDATE SET
                fecha = datetime('now', 'localtime'),
                accion = excluded.accion,
                veces_mostrada = historial_recomendaciones.veces_mostrada + 1",
            params![id, accion]
        ).map_err(|e| format!("Error registrando en historial_recomendaciones: {}", e))?;

        tx.commit().map_err(|e| format!("Error commit transacción: {}", e))?;
        id
    };

    if let Some(poster) = &obra.poster_url {
        let remote_url = if poster.starts_with("http://") || poster.starts_with("https://") {
            poster.clone()
        } else if poster.starts_with('/') {
            format!("https://image.tmdb.org/t/p/w500{}", poster)
        } else {
            String::new()
        };

        if !remote_url.is_empty() {
            let cid = contenido_id;
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent("QueVeoHoy/2.0 (https://github.com/alan/queveohoy)")
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new());
                crate::discovery::cache_poster_image(&client, cid, &remote_url).await;
            });
        }
    }

    if estado == "en_progreso" && (obra.tipo == "TV" || obra.es_anime) {
        let tmdb_id = obra.tmdb_id;
        let mal_id = obra.mal_id;
        let es_anime = obra.es_anime;
        let cid = contenido_id;
        let _ = crate::tmdb::fetch_and_cache_temporadas(cid, tmdb_id, mal_id, es_anime).await;
    }

    if obra.tmdb_id.is_some() {
        let _ = crate::tmdb::fetch_and_apply_enrichment(contenido_id).await;
    }

    get_recomendacion_by_contenido_id(contenido_id)
        .map_err(|e| format!("Error al recuperar recomendación guardada: {}", e))
}

pub fn update_contenido_enrichment(
    contenido_id: i64,
    director: Option<&str>,
    creadores: Option<&str>,
    reparto: Option<&str>,
    total_temporadas: Option<i64>,
    plataformas: Option<&str>,
    providers_updated_at: Option<&str>,
    status: Option<&str>,
    last_air_date: Option<&str>,
    duracion: Option<i64>,
    backdrop_url: Option<&str>,
) -> Result<()> {
    let conn = get_connection()?;
    conn.execute(
        "UPDATE contenido SET 
            director = COALESCE(?1, director),
            creadores = COALESCE(?2, creadores),
            reparto = COALESCE(?3, reparto),
            total_temporadas = COALESCE(?4, total_temporadas),
            plataformas = COALESCE(?5, plataformas),
            providers_updated_at = COALESCE(?6, providers_updated_at),
            status = COALESCE(?7, status),
            last_air_date = COALESCE(?8, last_air_date),
            duracion = COALESCE(?9, duracion),
            backdrop_url = COALESCE(?10, backdrop_url),
            last_metadata_fetch_error = NULL
         WHERE id = ?11",
        params![
            director,
            creadores,
            reparto,
            total_temporadas,
            plataformas,
            providers_updated_at,
            status,
            last_air_date,
            duracion,
            backdrop_url,
            contenido_id
        ],
    )?;
    Ok(())
}

pub fn get_contenido_enrichment_params(id: i64) -> Result<(Option<i64>, Option<i64>, String, bool)> {
    let conn = get_connection()?;
    conn.query_row(
        "SELECT tmdb_id, mal_id, tipo, es_anime FROM contenido WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get::<_, i64>(3)? == 1))
    )
}



pub fn omitir_en_progreso(state: &DbState, contenido_id: i64) -> Result<()> {
    let mut omitted = state.omitted_en_progreso_session.lock().unwrap();
    omitted.insert(contenido_id);
    Ok(())
}

pub fn get_catalogo() -> Result<Vec<crate::models::ResultadoBusqueda>> {
    let conn = get_connection()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.backdrop_url, c.sinopsis, 
                c.fecha_estreno, c.generos, uc.estado
         FROM catalogo_offline co
         JOIN contenido c ON co.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE c.poster_url IS NOT NULL AND TRIM(c.poster_url) != ''
         ORDER BY co.fecha_agregado DESC"
    )?;

    let cache_dir = get_cache_posters_dir();

    let resolve_poster = |id: i64, raw_path: Option<String>| -> Option<String> {
        resolve_poster_salida(&cache_dir, id, raw_path)
    };

    let resolve_backdrop = |raw_path: Option<String>| -> Option<String> {
        raw_path.map(|path| {
            if path.starts_with("http://") || path.starts_with("https://") {
                path
            } else if path.starts_with('/') {
                format!("https://image.tmdb.org/t/p/w1280{}", path)
            } else {
                path
            }
        })
    };

    let mut items = Vec::new();
    let mut rows = stmt.query([])?;
    let mut seen_ids = std::collections::HashSet::new();

    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let tmdb_id: Option<i64> = row.get(1)?;
        let mal_id: Option<i64> = row.get(2)?;
        let key = if let Some(t) = tmdb_id {
            format!("tmdb_{}", t)
        } else if let Some(m) = mal_id {
            format!("mal_{}", m)
        } else {
            let t: String = row.get(5)?;
            format!("title_{}", t)
        };
        seen_ids.insert(key);

        let raw_poster: Option<String> = row.get(7)?;
        let poster_url = resolve_poster(id, raw_poster);

        let raw_backdrop: Option<String> = row.get(8)?;
        let backdrop_url = resolve_backdrop(raw_backdrop);

        let estado: Option<String> = row.get(12)?;
        let ya_en_biblioteca = estado.is_some();
        let es_anime_raw: i64 = row.get(4)?;
        items.push(crate::models::ResultadoBusqueda {
            tmdb_id,
            mal_id,
            tipo: row.get(3)?,
            es_anime: es_anime_raw == 1,
            titulo: row.get(5)?,
            titulo_original: row.get(6)?,
            poster_url,
            backdrop_url,
            sinopsis: row.get(9)?,
            fecha_estreno: row.get(10)?,
            generos: row.get(11)?,
            ya_en_biblioteca,
            estado_actual: estado,
        });
    }

    if items.len() < 80 {
        let mut stmt2 = conn.prepare(
            "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                    c.titulo_original, c.poster_url, c.backdrop_url, c.sinopsis, 
                    c.fecha_estreno, c.generos, uc.estado
             FROM contenido c
             LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
             WHERE c.poster_url IS NOT NULL AND TRIM(c.poster_url) != ''
             ORDER BY c.id DESC
             LIMIT 150"
        )?;
        let mut rows2 = stmt2.query([])?;
        while let Some(row) = rows2.next()? {
            let id: i64 = row.get(0)?;
            let tmdb_id: Option<i64> = row.get(1)?;
            let mal_id: Option<i64> = row.get(2)?;
            let key = if let Some(t) = tmdb_id {
                format!("tmdb_{}", t)
            } else if let Some(m) = mal_id {
                format!("mal_{}", m)
            } else {
                let t: String = row.get(5)?;
                format!("title_{}", t)
            };
            if seen_ids.contains(&key) {
                continue;
            }
            seen_ids.insert(key);

            let raw_poster: Option<String> = row.get(7)?;
            let poster_url = resolve_poster(id, raw_poster);

            let raw_backdrop: Option<String> = row.get(8)?;
            let backdrop_url = resolve_backdrop(raw_backdrop);

            let estado: Option<String> = row.get(12)?;
            let ya_en_biblioteca = estado.is_some();
            let es_anime_raw: i64 = row.get(4)?;
            items.push(crate::models::ResultadoBusqueda {
                tmdb_id,
                mal_id,
                tipo: row.get(3)?,
                es_anime: es_anime_raw == 1,
                titulo: row.get(5)?,
                titulo_original: row.get(6)?,
                poster_url,
                backdrop_url,
                sinopsis: row.get(9)?,
                fecha_estreno: row.get(10)?,
                generos: row.get(11)?,
                ya_en_biblioteca,
                estado_actual: estado,
            });
        }
    }

    Ok(items)
}

/// Etapa 5.9.2B: Recuperar obras que ya existen en la tabla `contenido`
/// pero que todavía no están incorporadas en `catalogo_offline`.
///
/// Criterios obligatorios:
/// - Existe en `contenido`.
/// - No está en `catalogo_offline`.
/// - No pertenece a `usuario_contenido`.
/// - Corresponde al grupo solicitado.
///
/// Priorización de candidatas:
/// 1. Con póster, sinopsis y géneros.
/// 2. Con póster y sinopsis, sin géneros.
/// 3. Con póster y otros metadatos incompletos.
/// 4. Con sinopsis, sin póster.
/// 5. Información muy limitada.
///
/// Criterio secundario: `c.creado_en DESC` (ingreso más reciente a la base).
/// Desempate determinista: `c.id ASC`.
///
/// Transaccional e idempotente (INSERT OR IGNORE con BEGIN IMMEDIATE y reintentos en caso de SQLITE_BUSY).
pub fn recuperar_obras_locales_por_grupo(grupo: crate::models::GrupoContenido, limite: usize) -> Result<usize> {
    let mut conn = get_connection()?;
    recuperar_obras_locales_por_grupo_conn(&mut conn, grupo, limite)
}

pub fn recuperar_obras_locales_por_grupo_conn(
    conn: &mut Connection,
    grupo: crate::models::GrupoContenido,
    limite: usize,
) -> Result<usize> {
    if limite == 0 {
        return Ok(0);
    }

    let (tipo_db, es_anime_db) = match grupo {
        crate::models::GrupoContenido::PeliculasNoAnime => ("MOVIE", 0),
        crate::models::GrupoContenido::SeriesNoAnime => ("TV", 0),
        crate::models::GrupoContenido::PeliculasAnime => ("MOVIE", 1),
        crate::models::GrupoContenido::SeriesAnime => ("TV", 1),
    };

    const MAX_RETRIES: usize = 5;
    let mut retries = 0;

    loop {
        let result = (|| -> Result<usize> {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

            // 1. Seleccionar IDs candidatos con la priorización estricta por calidad de metadatos
            let candidate_ids: Vec<i64> = {
                let mut stmt = tx.prepare(
                    "SELECT c.id
                     FROM contenido c
                     WHERE c.tipo = ?1 AND c.es_anime = ?2
                       AND c.id NOT IN (SELECT contenido_id FROM catalogo_offline)
                       AND c.id NOT IN (SELECT contenido_id FROM usuario_contenido)
                     ORDER BY
                         CASE
                             WHEN (c.poster_url IS NOT NULL AND TRIM(c.poster_url) != '')
                                  AND (c.sinopsis IS NOT NULL AND TRIM(c.sinopsis) != '')
                                  AND (c.generos IS NOT NULL AND TRIM(c.generos) != '' AND c.generos != '[]') THEN 1
                             WHEN (c.poster_url IS NOT NULL AND TRIM(c.poster_url) != '')
                                  AND (c.sinopsis IS NOT NULL AND TRIM(c.sinopsis) != '') THEN 2
                             WHEN (c.poster_url IS NOT NULL AND TRIM(c.poster_url) != '') THEN 3
                             WHEN (c.sinopsis IS NOT NULL AND TRIM(c.sinopsis) != '') THEN 4
                             ELSE 5
                         END ASC,
                         c.creado_en DESC,
                         c.id ASC
                     LIMIT ?3"
                )?;

                let rows = stmt.query_map(params![tipo_db, es_anime_db, limite as i64], |row| row.get(0))?;
                let mut ids = Vec::new();
                for r in rows {
                    ids.push(r?);
                }
                ids
            };

            if candidate_ids.is_empty() {
                tx.commit()?;
                return Ok(0);
            }

            // 2. Insertar cada candidato comprobando ausencia en usuario_contenido
            let mut insertados = 0;
            {
                let mut insert_stmt = tx.prepare(
                    "INSERT OR IGNORE INTO catalogo_offline (contenido_id, fecha_agregado)
                     SELECT ?1, datetime('now', 'localtime')
                     WHERE NOT EXISTS (
                         SELECT 1 FROM usuario_contenido WHERE contenido_id = ?1
                     )"
                )?;

                for id in candidate_ids {
                    let rows_affected = insert_stmt.execute(params![id])?;
                    if rows_affected > 0 {
                        insertados += rows_affected;
                    }
                }
            }

            tx.commit()?;
            Ok(insertados)
        })();

        match result {
            Ok(count) => return Ok(count),
            Err(e) => {
                if let rusqlite::Error::SqliteFailure(ref f, _) = e {
                    if f.extended_code == rusqlite::ffi::SQLITE_BUSY
                        || f.extended_code == rusqlite::ffi::SQLITE_BUSY_SNAPSHOT
                        || f.code == rusqlite::ErrorCode::DatabaseBusy
                    {
                        retries += 1;
                        if retries < MAX_RETRIES {
                            std::thread::sleep(std::time::Duration::from_millis(50 * retries as u64));
                            continue;
                        }
                    }
                }
                return Err(e);
            }
        }
    }
}

// ============================================================================
// Etapa 5.9.2D-1: Persistencia, métricas por grupo, presupuesto TMDB,
// bloqueos/cooldowns y reintentos de pósteres
// ============================================================================

pub const OBJETIVO_INVENTARIO_POR_GRUPO: i64 = 375;
pub const TMDB_VENTANA_SEGUNDOS: i64 = 300;
pub const TMDB_MAX_PETICIONES_VENTANA: i64 = 18;
pub const TMDB_MARGEN_INICIO_OPERACION: i64 = 16;
pub const TMDB_INTERVALO_OPERACION_SEGUNDOS: i64 = 60;
pub const TMDB_LOG_RETENCION_SEGUNDOS: i64 = 900;

pub const POSTER_MAX_INTENTOS: i64 = 5;
pub const ESTADO_POSTER_PENDIENTE: &str = "pendiente";
pub const ESTADO_POSTER_AGOTADO: &str = "agotado";
pub const ESTADO_POSTER_RESUELTO: &str = "resuelto";

pub fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn with_busy_retry<T, F>(mut op: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    const MAX_RETRIES: usize = 5;
    let mut retries = 0;

    loop {
        match op() {
            Ok(val) => return Ok(val),
            Err(e) => {
                if let rusqlite::Error::SqliteFailure(ref f, _) = e {
                    if f.extended_code == rusqlite::ffi::SQLITE_BUSY
                        || f.extended_code == rusqlite::ffi::SQLITE_BUSY_SNAPSHOT
                        || f.code == rusqlite::ErrorCode::DatabaseBusy
                    {
                        retries += 1;
                        if retries < MAX_RETRIES {
                            std::thread::sleep(std::time::Duration::from_millis(50 * retries as u64));
                            continue;
                        }
                    }
                }
                return Err(e);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Métricas por grupo (Inventario y Candidatas locales)
// ----------------------------------------------------------------------------

pub fn get_inventario_por_grupo() -> Result<crate::models::ConteoPorGrupo> {
    let conn = get_connection()?;
    get_inventario_por_grupo_conn(&conn)
}

pub fn get_inventario_por_grupo_conn(conn: &Connection) -> Result<crate::models::ConteoPorGrupo> {
    let mut stmt = conn.prepare(
        "SELECT
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0),
            COUNT(c.id)
         FROM catalogo_offline co
         JOIN contenido c ON co.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE uc.contenido_id IS NULL",
    )?;

    stmt.query_row([], |row| {
        Ok(crate::models::ConteoPorGrupo {
            peliculas_no_anime: row.get(0)?,
            series_no_anime: row.get(1)?,
            peliculas_anime: row.get(2)?,
            series_anime: row.get(3)?,
            total: row.get(4)?,
        })
    })
}

pub fn get_inventario_grupo(grupo: crate::models::GrupoContenido) -> Result<i64> {
    let conn = get_connection()?;
    get_inventario_grupo_conn(&conn, grupo)
}

pub fn get_inventario_grupo_conn(
    conn: &Connection,
    grupo: crate::models::GrupoContenido,
) -> Result<i64> {
    let (tipo_db, es_anime_db) = grupo.db_filter();
    conn.query_row(
        "SELECT COUNT(c.id)
         FROM catalogo_offline co
         JOIN contenido c ON co.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE c.tipo = ?1
           AND c.es_anime = ?2
           AND uc.contenido_id IS NULL",
        params![tipo_db, es_anime_db],
        |row| row.get(0),
    )
}

pub fn get_candidatas_locales_por_grupo() -> Result<crate::models::ConteoPorGrupo> {
    let conn = get_connection()?;
    get_candidatas_locales_por_grupo_conn(&conn)
}

pub fn get_candidatas_locales_por_grupo_conn(
    conn: &Connection,
) -> Result<crate::models::ConteoPorGrupo> {
    let mut stmt = conn.prepare(
        "SELECT
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'MOVIE' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN c.tipo = 'TV' AND c.es_anime = 1 THEN 1 ELSE 0 END), 0),
            COUNT(c.id)
         FROM contenido c
         LEFT JOIN catalogo_offline co ON c.id = co.contenido_id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE co.contenido_id IS NULL
           AND uc.contenido_id IS NULL",
    )?;

    stmt.query_row([], |row| {
        Ok(crate::models::ConteoPorGrupo {
            peliculas_no_anime: row.get(0)?,
            series_no_anime: row.get(1)?,
            peliculas_anime: row.get(2)?,
            series_anime: row.get(3)?,
            total: row.get(4)?,
        })
    })
}

pub fn get_candidatas_locales_grupo(grupo: crate::models::GrupoContenido) -> Result<i64> {
    let conn = get_connection()?;
    get_candidatas_locales_grupo_conn(&conn, grupo)
}

pub fn get_candidatas_locales_grupo_conn(
    conn: &Connection,
    grupo: crate::models::GrupoContenido,
) -> Result<i64> {
    let (tipo_db, es_anime_db) = grupo.db_filter();
    conn.query_row(
        "SELECT COUNT(c.id)
         FROM contenido c
         LEFT JOIN catalogo_offline co ON c.id = co.contenido_id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE c.tipo = ?1
           AND c.es_anime = ?2
           AND co.contenido_id IS NULL
           AND uc.contenido_id IS NULL",
        params![tipo_db, es_anime_db],
        |row| row.get(0),
    )
}

// ----------------------------------------------------------------------------
// Reserva atómica del presupuesto TMDB
// ----------------------------------------------------------------------------

pub fn limpiar_tmdb_request_log_conn(conn: &Connection, now: i64) -> Result<usize> {
    let cutoff = now - TMDB_LOG_RETENCION_SEGUNDOS;
    conn.execute(
        "DELETE FROM tmdb_request_log WHERE requested_at < ?1",
        params![cutoff],
    )
}

pub fn reservar_peticion_tmdb(
    endpoint_kind: &str,
    es_inicio_operacion: bool,
) -> Result<crate::models::ReservaTmdbResultado> {
    let mut conn = get_connection()?;
    reservar_peticion_tmdb_conn(&mut conn, endpoint_kind, es_inicio_operacion)
}

pub fn reservar_peticion_tmdb_conn(
    conn: &mut Connection,
    endpoint_kind: &str,
    es_inicio_operacion: bool,
) -> Result<crate::models::ReservaTmdbResultado> {
    reservar_peticion_tmdb_at(conn, endpoint_kind, es_inicio_operacion, current_unix_timestamp())
}

pub fn reservar_peticion_tmdb_at(
    conn: &mut Connection,
    endpoint_kind: &str,
    es_inicio_operacion: bool,
    now: i64,
) -> Result<crate::models::ReservaTmdbResultado> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
            [],
        )?;

        let retention_cutoff = now - TMDB_LOG_RETENCION_SEGUNDOS;
        tx.execute(
            "DELETE FROM tmdb_request_log WHERE requested_at < ?1",
            params![retention_cutoff],
        )?;

        let (bloqueado_hasta, motivo_bloqueo, ultima_operacion_at): (
            Option<i64>,
            Option<String>,
            Option<i64>,
        ) = tx.query_row(
            "SELECT tmdb_bloqueado_hasta, motivo_bloqueo, ultima_operacion_at
             FROM tmdb_worker_state
             WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        if let Some(hasta) = bloqueado_hasta {
            if hasta > now {
                tx.commit()?;
                return Ok(crate::models::ReservaTmdbResultado::BloqueoActivo {
                    bloqueado_hasta: hasta,
                    motivo: motivo_bloqueo,
                });
            }
        }

        if es_inicio_operacion {
            if let Some(ultimo) = ultima_operacion_at {
                let proximo = ultimo + TMDB_INTERVALO_OPERACION_SEGUNDOS;
                if now < proximo {
                    tx.commit()?;
                    return Ok(
                        crate::models::ReservaTmdbResultado::IntervaloOperacionNoCumplido {
                            ultimo_inicio_at: ultimo,
                            proximo_disponible_at: proximo,
                        },
                    );
                }
            }
        }

        let window_start = now - TMDB_VENTANA_SEGUNDOS;
        let count_en_ventana: i64 = tx.query_row(
            "SELECT COUNT(*) FROM tmdb_request_log WHERE requested_at > ?1",
            params![window_start],
            |row| row.get(0),
        )?;

        if es_inicio_operacion {
            if count_en_ventana > TMDB_MARGEN_INICIO_OPERACION {
                tx.commit()?;
                return Ok(crate::models::ReservaTmdbResultado::PresupuestoAgotado {
                    peticiones_en_ventana: count_en_ventana,
                    limite_aplicado: TMDB_MARGEN_INICIO_OPERACION,
                });
            }
        } else if count_en_ventana >= TMDB_MAX_PETICIONES_VENTANA {
            tx.commit()?;
            return Ok(crate::models::ReservaTmdbResultado::PresupuestoAgotado {
                peticiones_en_ventana: count_en_ventana,
                limite_aplicado: TMDB_MAX_PETICIONES_VENTANA,
            });
        }

        tx.execute(
            "INSERT INTO tmdb_request_log (requested_at, endpoint_kind) VALUES (?1, ?2)",
            params![now, endpoint_kind],
        )?;
        let reserva_id = tx.last_insert_rowid();

        if es_inicio_operacion {
            tx.execute(
                "UPDATE tmdb_worker_state SET ultima_operacion_at = ?1 WHERE id = 1",
                params![now],
            )?;
        }

        tx.commit()?;

        let nuevas_en_ventana = count_en_ventana + 1;
        Ok(crate::models::ReservaTmdbResultado::Reservada {
            reserva_id,
            peticiones_en_ventana: nuevas_en_ventana,
            restante_en_ventana: TMDB_MAX_PETICIONES_VENTANA - nuevas_en_ventana,
        })
    })
}

// ----------------------------------------------------------------------------
// Persistencia de bloqueos globales, errores y cooldowns por grupo
// ----------------------------------------------------------------------------

pub fn get_tmdb_worker_state() -> Result<crate::models::TmdbWorkerState> {
    let conn = get_connection()?;
    get_tmdb_worker_state_conn(&conn)
}

pub fn get_tmdb_worker_state_conn(conn: &Connection) -> Result<crate::models::TmdbWorkerState> {
    conn.execute(
        "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
        [],
    )?;

    conn.query_row(
        "SELECT tmdb_bloqueado_hasta, motivo_bloqueo, ultimo_error, errores_consecutivos, ultima_operacion_at
         FROM tmdb_worker_state
         WHERE id = 1",
        [],
        |row| {
            Ok(crate::models::TmdbWorkerState {
                tmdb_bloqueado_hasta: row.get(0)?,
                motivo_bloqueo: row.get(1)?,
                ultimo_error: row.get(2)?,
                errores_consecutivos: row.get(3)?,
                ultima_operacion_at: row.get(4)?,
            })
        },
    )
}

pub fn registrar_bloqueo_tmdb(bloqueado_hasta: i64, motivo: &str) -> Result<()> {
    let mut conn = get_connection()?;
    registrar_bloqueo_tmdb_conn(&mut conn, bloqueado_hasta, motivo)
}

pub fn registrar_bloqueo_tmdb_conn(
    conn: &mut Connection,
    bloqueado_hasta: i64,
    motivo: &str,
) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
            [],
        )?;
        tx.execute(
            "UPDATE tmdb_worker_state
             SET tmdb_bloqueado_hasta = ?1,
                 motivo_bloqueo = ?2
             WHERE id = 1",
            params![bloqueado_hasta, motivo],
        )?;
        tx.commit()?;
        Ok(())
    })
}

pub fn registrar_error_tmdb(error: &str) -> Result<i64> {
    let mut conn = get_connection()?;
    registrar_error_tmdb_conn(&mut conn, error)
}

pub fn registrar_error_tmdb_conn(conn: &mut Connection, error: &str) -> Result<i64> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
            [],
        )?;
        tx.execute(
            "UPDATE tmdb_worker_state
             SET ultimo_error = ?1,
                 errores_consecutivos = errores_consecutivos + 1
             WHERE id = 1",
            params![error],
        )?;
        let consecutivos: i64 = tx.query_row(
            "SELECT errores_consecutivos FROM tmdb_worker_state WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(consecutivos)
    })
}

pub fn limpiar_bloqueo_tmdb() -> Result<()> {
    let mut conn = get_connection()?;
    limpiar_bloqueo_tmdb_conn(&mut conn)
}

pub fn limpiar_bloqueo_tmdb_conn(conn: &mut Connection) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE tmdb_worker_state
             SET tmdb_bloqueado_hasta = NULL,
                 motivo_bloqueo = NULL
             WHERE id = 1",
            [],
        )?;
        tx.commit()?;
        Ok(())
    })
}

pub fn restablecer_errores_tmdb() -> Result<()> {
    let mut conn = get_connection()?;
    restablecer_errores_tmdb_conn(&mut conn)
}

pub fn restablecer_errores_tmdb_conn(conn: &mut Connection) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE tmdb_worker_state
             SET ultimo_error = NULL,
                 errores_consecutivos = 0
             WHERE id = 1",
            [],
        )?;
        tx.commit()?;
        Ok(())
    })
}

pub fn restablecer_estado_tmdb() -> Result<()> {
    let mut conn = get_connection()?;
    restablecer_estado_tmdb_conn(&mut conn)
}

pub fn restablecer_estado_tmdb_conn(conn: &mut Connection) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO tmdb_worker_state (id, errores_consecutivos) VALUES (1, 0)",
            [],
        )?;
        tx.execute(
            "UPDATE tmdb_worker_state
             SET tmdb_bloqueado_hasta = NULL,
                 motivo_bloqueo = NULL,
                 ultimo_error = NULL,
                 errores_consecutivos = 0
             WHERE id = 1",
            [],
        )?;
        tx.commit()?;
        Ok(())
    })
}

pub fn registrar_cooldown_grupo(
    grupo: crate::models::GrupoContenido,
    bloqueado_hasta: i64,
    fallos_sin_aporte: i64,
    motivo: Option<&str>,
) -> Result<()> {
    let mut conn = get_connection()?;
    registrar_cooldown_grupo_conn(&mut conn, grupo, bloqueado_hasta, fallos_sin_aporte, motivo)
}

pub fn registrar_cooldown_grupo_conn(
    conn: &mut Connection,
    grupo: crate::models::GrupoContenido,
    bloqueado_hasta: i64,
    fallos_sin_aporte: i64,
    motivo: Option<&str>,
) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO tmdb_group_cooldown (grupo, bloqueado_hasta, fallos_sin_aporte, motivo)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(grupo) DO UPDATE SET
                 bloqueado_hasta = excluded.bloqueado_hasta,
                 fallos_sin_aporte = excluded.fallos_sin_aporte,
                 motivo = excluded.motivo",
            params![grupo.as_str(), bloqueado_hasta, fallos_sin_aporte, motivo],
        )?;
        tx.commit()?;
        Ok(())
    })
}

pub fn get_cooldown_grupo(
    grupo: crate::models::GrupoContenido,
) -> Result<Option<crate::models::TmdbGroupCooldown>> {
    let conn = get_connection()?;
    get_cooldown_grupo_conn(&conn, grupo)
}

pub fn get_cooldown_grupo_conn(
    conn: &Connection,
    grupo: crate::models::GrupoContenido,
) -> Result<Option<crate::models::TmdbGroupCooldown>> {
    let mut stmt = conn.prepare(
        "SELECT grupo, bloqueado_hasta, fallos_sin_aporte, motivo
         FROM tmdb_group_cooldown
         WHERE grupo = ?1",
    )?;
    let mut rows = stmt.query(params![grupo.as_str()])?;
    if let Some(row) = rows.next()? {
        let grupo_str: String = row.get(0)?;
        let parsed = crate::models::GrupoContenido::from_db_str(&grupo_str).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(
                0,
                "grupo".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;
        Ok(Some(crate::models::TmdbGroupCooldown {
            grupo: parsed,
            bloqueado_hasta: row.get(1)?,
            fallos_sin_aporte: row.get(2)?,
            motivo: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_todos_cooldowns_grupo() -> Result<Vec<crate::models::TmdbGroupCooldown>> {
    let conn = get_connection()?;
    get_todos_cooldowns_grupo_conn(&conn)
}

pub fn get_todos_cooldowns_grupo_conn(
    conn: &Connection,
) -> Result<Vec<crate::models::TmdbGroupCooldown>> {
    let mut stmt = conn.prepare(
        "SELECT grupo, bloqueado_hasta, fallos_sin_aporte, motivo
         FROM tmdb_group_cooldown
         ORDER BY grupo ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let grupo_str: String = row.get(0)?;
        let parsed = crate::models::GrupoContenido::from_db_str(&grupo_str).ok_or_else(|| {
            rusqlite::Error::InvalidColumnType(
                0,
                "grupo".to_string(),
                rusqlite::types::Type::Text,
            )
        })?;
        Ok(crate::models::TmdbGroupCooldown {
            grupo: parsed,
            bloqueado_hasta: row.get(1)?,
            fallos_sin_aporte: row.get(2)?,
            motivo: row.get(3)?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn restablecer_cooldown_grupo(grupo: crate::models::GrupoContenido) -> Result<()> {
    let mut conn = get_connection()?;
    restablecer_cooldown_grupo_conn(&mut conn, grupo)
}

pub fn restablecer_cooldown_grupo_conn(
    conn: &mut Connection,
    grupo: crate::models::GrupoContenido,
) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "DELETE FROM tmdb_group_cooldown WHERE grupo = ?1",
            params![grupo.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    })
}

// ----------------------------------------------------------------------------
// Persistencia de reintentos de pósteres (poster_retry_state)
// ----------------------------------------------------------------------------

fn calcular_backoff_poster_segundos(intentos: i64) -> i64 {
    match intentos {
        1 => 300,
        2 => 900,
        3 => 3600,
        4 => 21600,
        _ => 86400,
    }
}

pub fn get_poster_retry_state(contenido_id: i64) -> Result<Option<crate::models::PosterRetryState>> {
    let conn = get_connection()?;
    get_poster_retry_state_conn(&conn, contenido_id)
}

pub fn get_poster_retry_state_conn(
    conn: &Connection,
    contenido_id: i64,
) -> Result<Option<crate::models::PosterRetryState>> {
    let mut stmt = conn.prepare(
        "SELECT contenido_id, url_fallida, intentos, estado, proximo_reintento, ultimo_error, actualizado_en
         FROM poster_retry_state
         WHERE contenido_id = ?1",
    )?;
    let mut rows = stmt.query(params![contenido_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(crate::models::PosterRetryState {
            contenido_id: row.get(0)?,
            url_fallida: row.get(1)?,
            intentos: row.get(2)?,
            estado: row.get(3)?,
            proximo_reintento: row.get(4)?,
            ultimo_error: row.get(5)?,
            actualizado_en: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn registrar_fallo_poster(
    contenido_id: i64,
    url_fallida: &str,
    error: &str,
) -> Result<crate::models::PosterRetryState> {
    let mut conn = get_connection()?;
    registrar_fallo_poster_conn(&mut conn, contenido_id, url_fallida, error)
}

pub fn registrar_fallo_poster_conn(
    conn: &mut Connection,
    contenido_id: i64,
    url_fallida: &str,
    error: &str,
) -> Result<crate::models::PosterRetryState> {
    registrar_fallo_poster_at(
        conn,
        contenido_id,
        url_fallida,
        error,
        current_unix_timestamp(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoDescargaPoster {
    Exito {
        contenido_id: i64,
    },
    Fallo {
        contenido_id: i64,
        url: String,
        error: String,
    },
}

fn aplicar_fallo_poster_tx(
    tx: &rusqlite::Transaction<'_>,
    contenido_id: i64,
    url_fallida: &str,
    error: &str,
    now: i64,
) -> Result<crate::models::PosterRetryState> {
    let existente: Option<(String, i64, String)> = {
        let mut stmt = tx.prepare(
            "SELECT url_fallida, intentos, estado
             FROM poster_retry_state
             WHERE contenido_id = ?1",
        )?;
        let mut rows = stmt.query(params![contenido_id])?;
        if let Some(row) = rows.next()? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?))
        } else {
            None
        }
    };

    let nuevos_intentos = match existente {
        Some((prev_url, prev_intentos, prev_estado))
            if prev_url == url_fallida && prev_estado != ESTADO_POSTER_RESUELTO =>
        {
            prev_intentos + 1
        }
        _ => 1,
    };

    let (estado, proximo_reintento) = if nuevos_intentos >= POSTER_MAX_INTENTOS {
        (ESTADO_POSTER_AGOTADO.to_string(), None)
    } else {
        let backoff = calcular_backoff_poster_segundos(nuevos_intentos);
        (ESTADO_POSTER_PENDIENTE.to_string(), Some(now + backoff))
    };

    tx.execute(
        "INSERT INTO poster_retry_state (
            contenido_id, url_fallida, intentos, estado, proximo_reintento, ultimo_error, actualizado_en
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(contenido_id) DO UPDATE SET
            url_fallida = excluded.url_fallida,
            intentos = excluded.intentos,
            estado = excluded.estado,
            proximo_reintento = excluded.proximo_reintento,
            ultimo_error = excluded.ultimo_error,
            actualizado_en = excluded.actualizado_en",
        params![
            contenido_id,
            url_fallida,
            nuevos_intentos,
            estado,
            proximo_reintento,
            error,
            now
        ],
    )?;

    Ok(crate::models::PosterRetryState {
        contenido_id,
        url_fallida: url_fallida.to_string(),
        intentos: nuevos_intentos,
        estado,
        proximo_reintento,
        ultimo_error: Some(error.to_string()),
        actualizado_en: now,
    })
}

fn aplicar_exito_poster_tx(
    tx: &rusqlite::Transaction<'_>,
    contenido_id: i64,
    now: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO poster_retry_state (
            contenido_id, url_fallida, intentos, estado, proximo_reintento, ultimo_error, actualizado_en
         ) VALUES (
            ?1,
            COALESCE((SELECT TRIM(poster_url) FROM contenido WHERE id = ?1), ''),
            0,
            ?2,
            NULL,
            NULL,
            ?3
         )
         ON CONFLICT(contenido_id) DO UPDATE SET
            intentos = 0,
            estado = excluded.estado,
            proximo_reintento = NULL,
            ultimo_error = NULL,
            actualizado_en = excluded.actualizado_en",
        params![contenido_id, ESTADO_POSTER_RESUELTO, now],
    )?;
    Ok(())
}

pub fn registrar_fallo_poster_at(
    conn: &mut Connection,
    contenido_id: i64,
    url_fallida: &str,
    error: &str,
    now: i64,
) -> Result<crate::models::PosterRetryState> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let state = aplicar_fallo_poster_tx(&tx, contenido_id, url_fallida, error, now)?;
        tx.commit()?;
        Ok(state)
    })
}

pub fn registrar_exito_poster(contenido_id: i64) -> Result<()> {
    let mut conn = get_connection()?;
    registrar_exito_poster_conn(&mut conn, contenido_id)
}

pub fn registrar_exito_poster_conn(conn: &mut Connection, contenido_id: i64) -> Result<()> {
    registrar_exito_poster_at(conn, contenido_id, current_unix_timestamp())
}

pub fn registrar_exito_poster_at(
    conn: &mut Connection,
    contenido_id: i64,
    now: i64,
) -> Result<()> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        aplicar_exito_poster_tx(&tx, contenido_id, now)?;
        tx.commit()?;
        Ok(())
    })
}

pub fn persistir_resultados_posters(resultados: &[ResultadoDescargaPoster]) -> Result<()> {
    if resultados.is_empty() {
        return Ok(());
    }
    let mut conn = get_connection()?;
    persistir_resultados_posters_conn(&mut conn, resultados)
}

pub fn persistir_resultados_posters_conn(
    conn: &mut Connection,
    resultados: &[ResultadoDescargaPoster],
) -> Result<()> {
    persistir_resultados_posters_at(conn, resultados, current_unix_timestamp())
}

pub fn persistir_resultados_posters_at(
    conn: &mut Connection,
    resultados: &[ResultadoDescargaPoster],
    now: i64,
) -> Result<()> {
    if resultados.is_empty() {
        return Ok(());
    }
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for resultado in resultados {
            match resultado {
                ResultadoDescargaPoster::Exito { contenido_id } => {
                    aplicar_exito_poster_tx(&tx, *contenido_id, now)?;
                }
                ResultadoDescargaPoster::Fallo {
                    contenido_id,
                    url,
                    error,
                } => {
                    aplicar_fallo_poster_tx(&tx, *contenido_id, url, error, now)?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    })
}

pub fn get_posters_pendientes_mantenimiento(
    limite: usize,
) -> Result<Vec<crate::models::PosterRetryState>> {
    let conn = get_connection()?;
    get_posters_pendientes_mantenimiento_conn(&conn, limite)
}

pub fn get_posters_pendientes_mantenimiento_conn(
    conn: &Connection,
    limite: usize,
) -> Result<Vec<crate::models::PosterRetryState>> {
    get_posters_pendientes_mantenimiento_at(conn, current_unix_timestamp(), limite)
}

pub fn get_posters_pendientes_mantenimiento_at(
    conn: &Connection,
    now: i64,
    limite: usize,
) -> Result<Vec<crate::models::PosterRetryState>> {
    if limite == 0 {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare(
        "SELECT c.id,
                COALESCE(NULLIF(TRIM(prs.url_fallida), ''), TRIM(c.poster_url)) AS effective_url,
                COALESCE(prs.intentos, 0) AS intentos,
                COALESCE(prs.estado, ?1) AS estado,
                prs.proximo_reintento,
                prs.ultimo_error,
                COALESCE(prs.actualizado_en, 0) AS actualizado_en
         FROM catalogo_offline co
         INNER JOIN contenido c ON c.id = co.contenido_id
         LEFT JOIN usuario_contenido uc ON uc.contenido_id = c.id
         LEFT JOIN poster_retry_state prs ON prs.contenido_id = c.id
         WHERE uc.contenido_id IS NULL
           AND (
               prs.contenido_id IS NULL
               OR (prs.estado = ?1 AND (prs.proximo_reintento IS NULL OR prs.proximo_reintento <= ?2))
           )
           AND (
               COALESCE(NULLIF(TRIM(prs.url_fallida), ''), TRIM(c.poster_url)) LIKE 'http://%'
               OR COALESCE(NULLIF(TRIM(prs.url_fallida), ''), TRIM(c.poster_url)) LIKE 'https://%'
               OR COALESCE(NULLIF(TRIM(prs.url_fallida), ''), TRIM(c.poster_url)) LIKE '/%'
           )
         ORDER BY COALESCE(prs.proximo_reintento, 0) ASC, COALESCE(prs.actualizado_en, 0) ASC, c.id ASC
         LIMIT ?3",
    )?;

    let rows = stmt.query_map(
        params![ESTADO_POSTER_PENDIENTE, now, limite as i64],
        |row| {
            Ok(crate::models::PosterRetryState {
                contenido_id: row.get(0)?,
                url_fallida: row.get(1)?,
                intentos: row.get(2)?,
                estado: row.get(3)?,
                proximo_reintento: row.get(4)?,
                ultimo_error: row.get(5)?,
                actualizado_en: row.get(6)?,
            })
        },
    )?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn reiniciar_poster_retry_si_cambia_url(
    contenido_id: i64,
    nueva_url: &str,
) -> Result<bool> {
    let mut conn = get_connection()?;
    reiniciar_poster_retry_si_cambia_url_conn(&mut conn, contenido_id, nueva_url)
}

pub fn reiniciar_poster_retry_si_cambia_url_conn(
    conn: &mut Connection,
    contenido_id: i64,
    nueva_url: &str,
) -> Result<bool> {
    with_busy_retry(|| {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let rows_deleted = tx.execute(
            "DELETE FROM poster_retry_state
             WHERE contenido_id = ?1 AND url_fallida != ?2",
            params![contenido_id, nueva_url],
        )?;
        tx.commit()?;
        Ok(rows_deleted > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GrupoContenido, ReservaTmdbResultado};

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("abrir SQLite en memoria");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("activar foreign_keys");

        conn.execute_batch(
            "CREATE TABLE contenido (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tmdb_id INTEGER,
                mal_id INTEGER,
                tipo TEXT NOT NULL,
                es_anime INTEGER NOT NULL DEFAULT 0,
                titulo TEXT NOT NULL,
                titulo_original TEXT,
                poster_url TEXT,
                sinopsis TEXT,
                fecha_estreno TEXT,
                creado_en TEXT DEFAULT (datetime('now')),
                fetch_recently_failed INTEGER DEFAULT 0
            );
            CREATE TABLE catalogo_offline (
                contenido_id INTEGER PRIMARY KEY REFERENCES contenido(id),
                fecha_agregado TEXT
            );
            CREATE TABLE usuario_contenido (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contenido_id INTEGER NOT NULL UNIQUE REFERENCES contenido(id),
                estado TEXT NOT NULL,
                fecha_agregado TEXT,
                fecha_inicio TEXT,
                fecha_finalizacion TEXT
            );
            CREATE TABLE historial_recomendaciones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contenido_id INTEGER NOT NULL,
                accion TEXT NOT NULL,
                fecha TEXT
            );",
        )
        .expect("crear esquema base de prueba");

        init_db_conn(&conn).expect("ejecutar init_db_conn");
        conn
    }

    fn insert_contenido(
        conn: &Connection,
        tipo: &str,
        es_anime: i64,
        titulo: &str,
        poster_url: Option<&str>,
        sinopsis: Option<&str>,
        generos: Option<&str>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO contenido (tipo, es_anime, titulo, poster_url, sinopsis, generos)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![tipo, es_anime, titulo, poster_url, sinopsis, generos],
        )
        .expect("insertar contenido");
        conn.last_insert_rowid()
    }

    fn add_to_catalogo(conn: &Connection, contenido_id: i64) {
        conn.execute(
            "INSERT INTO catalogo_offline (contenido_id, fecha_agregado) VALUES (?1, datetime('now'))",
            params![contenido_id],
        )
        .expect("insertar en catalogo_offline");
    }

    fn add_to_usuario(conn: &Connection, contenido_id: i64, estado: &str) {
        conn.execute(
            "INSERT INTO usuario_contenido (contenido_id, estado, fecha_agregado) VALUES (?1, ?2, datetime('now'))",
            params![contenido_id, estado],
        )
        .expect("insertar en usuario_contenido");
    }

    #[test]
    fn test_01_creacion_idempotente_cuatro_tablas() {
        let conn = setup_test_db();
        init_db_conn(&conn).expect("segunda ejecucion de init_db_conn debe ser idempotente");

        for tabla in [
            "poster_retry_state",
            "tmdb_request_log",
            "tmdb_worker_state",
            "tmdb_group_cooldown",
        ] {
            let existe: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![tabla],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(existe, 1, "Debe existir la tabla {}", tabla);
        }

        let idx_existe: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_tmdb_request_log_requested_at'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(idx_existe, 1);
    }

    #[test]
    fn test_02_inicializacion_fila_unica_tmdb_worker_state() {
        let conn = setup_test_db();
        init_db_conn(&conn).expect("re-ejecucion idempotente");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tmdb_worker_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let state = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(state.errores_consecutivos, 0);
        assert_eq!(state.tmdb_bloqueado_hasta, None);
        assert_eq!(state.ultima_operacion_at, None);

        let err_insert = conn.execute(
            "INSERT INTO tmdb_worker_state (id, errores_consecutivos) VALUES (2, 0)",
            [],
        );
        assert!(err_insert.is_err(), "CHECK (id = 1) debe impedir filas con id != 1");
    }

    #[test]
    fn test_03_conteo_inventario_cuatro_grupos() {
        let conn = setup_test_db();

        let p_no_anime = insert_contenido(&conn, "MOVIE", 0, "Peli 1", None, None, None);
        let s_no_anime_1 = insert_contenido(&conn, "TV", 0, "Serie 1", None, None, None);
        let s_no_anime_2 = insert_contenido(&conn, "TV", 0, "Serie 2", None, None, None);
        let p_anime = insert_contenido(&conn, "MOVIE", 1, "Peli Anime 1", None, None, None);
        let s_anime = insert_contenido(&conn, "TV", 1, "Serie Anime 1", None, None, None);

        for id in [p_no_anime, s_no_anime_1, s_no_anime_2, p_anime, s_anime] {
            add_to_catalogo(&conn, id);
        }

        let inv = get_inventario_por_grupo_conn(&conn).unwrap();
        assert_eq!(inv.peliculas_no_anime, 1);
        assert_eq!(inv.series_no_anime, 2);
        assert_eq!(inv.peliculas_anime, 1);
        assert_eq!(inv.series_anime, 1);
        assert_eq!(inv.total, 5);
    }

    #[test]
    fn test_04_exclusion_biblioteca_del_inventario() {
        let conn = setup_test_db();

        let id_en_catalogo = insert_contenido(&conn, "MOVIE", 0, "Disponible", None, None, None);
        let id_en_biblioteca = insert_contenido(&conn, "MOVIE", 0, "En Biblioteca", None, None, None);

        add_to_catalogo(&conn, id_en_catalogo);
        add_to_catalogo(&conn, id_en_biblioteca);
        // Al insertar en usuario_contenido el trigger limpia catalogo_offline, pero incluso si existiera en ambas, la query excluye usuario_contenido.
        add_to_usuario(&conn, id_en_biblioteca, "PENDIENTE");

        let inv = get_inventario_grupo_conn(&conn, GrupoContenido::PeliculasNoAnime).unwrap();
        assert_eq!(inv, 1);
    }

    #[test]
    fn test_05_descartes_temporales_no_reducen_inventario() {
        let conn = setup_test_db();

        let id = insert_contenido(&conn, "TV", 1, "Anime Omitido", None, None, None);
        add_to_catalogo(&conn, id);
        conn.execute(
            "INSERT INTO omitidos_sesion (contenido_id, fecha) VALUES (?1, date('now', 'localtime'))",
            params![id],
        )
        .unwrap();

        let inv = get_inventario_grupo_conn(&conn, GrupoContenido::SeriesAnime).unwrap();
        assert_eq!(inv, 1);
    }

    #[test]
    fn test_06_candidatas_locales_sin_metadatos_completos_son_elegibles() {
        let conn = setup_test_db();

        insert_contenido(&conn, "MOVIE", 0, "Sin nada", None, None, None);
        insert_contenido(&conn, "TV", 0, "Solo titulo", Some(""), Some(""), Some("[]"));
        insert_contenido(&conn, "MOVIE", 1, "Solo sinopsis", None, Some("Desc"), None);
        insert_contenido(
            &conn,
            "TV",
            1,
            "Completa",
            Some("https://img/p.jpg"),
            Some("Desc"),
            Some("[\"Accion\"]"),
        );

        let cand = get_candidatas_locales_por_grupo_conn(&conn).unwrap();
        assert_eq!(cand.peliculas_no_anime, 1);
        assert_eq!(cand.series_no_anime, 1);
        assert_eq!(cand.peliculas_anime, 1);
        assert_eq!(cand.series_anime, 1);
        assert_eq!(cand.total, 4);
    }

    #[test]
    fn test_07_exclusion_obras_presentes_en_catalogo() {
        let conn = setup_test_db();

        let id1 = insert_contenido(&conn, "MOVIE", 0, "En catalogo", None, None, None);
        let _id2 = insert_contenido(&conn, "MOVIE", 0, "Solo local", None, None, None);
        add_to_catalogo(&conn, id1);

        let cand = get_candidatas_locales_grupo_conn(&conn, GrupoContenido::PeliculasNoAnime).unwrap();
        assert_eq!(cand, 1);
    }

    #[test]
    fn test_08_exclusion_obras_presentes_en_biblioteca() {
        let conn = setup_test_db();

        let id1 = insert_contenido(&conn, "TV", 0, "En biblioteca", None, None, None);
        let _id2 = insert_contenido(&conn, "TV", 0, "Candidata", None, None, None);
        add_to_usuario(&conn, id1, "VIENDO");

        let cand = get_candidatas_locales_grupo_conn(&conn, GrupoContenido::SeriesNoAnime).unwrap();
        assert_eq!(cand, 1);
    }

    #[test]
    fn test_09_reserva_inicial_con_presupuesto_disponible() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        let res = reservar_peticion_tmdb_at(&mut conn, "discover_movie", true, now).unwrap();
        assert_eq!(
            res,
            ReservaTmdbResultado::Reservada {
                reserva_id: 1,
                peticiones_en_ventana: 1,
                restante_en_ventana: 17,
            }
        );

        let state = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(state.ultima_operacion_at, Some(now));
    }

    #[test]
    fn test_10_rechazo_al_alcanzar_limite_global() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        // 1 inicio de operación + 17 internas = 18 peticiones (límite global)
        let first = reservar_peticion_tmdb_at(&mut conn, "discover", true, now).unwrap();
        assert!(matches!(first, ReservaTmdbResultado::Reservada { .. }));

        for i in 0..17 {
            let r = reservar_peticion_tmdb_at(&mut conn, "details", false, now + i).unwrap();
            assert!(matches!(r, ReservaTmdbResultado::Reservada { .. }));
        }

        // La petición 19 debe ser rechazada por presupuesto agotado
        let rejected = reservar_peticion_tmdb_at(&mut conn, "details", false, now + 18).unwrap();
        assert_eq!(
            rejected,
            ReservaTmdbResultado::PresupuestoAgotado {
                peticiones_en_ventana: 18,
                limite_aplicado: 18,
            }
        );

        // Si pasan 60s pero aún hay 17 en ventana (> 16), un nuevo inicio de operación también se rechaza
        let rejected_start =
            reservar_peticion_tmdb_at(&mut conn, "discover", true, now + 60).unwrap();
        assert_eq!(
            rejected_start,
            ReservaTmdbResultado::PresupuestoAgotado {
                peticiones_en_ventana: 18,
                limite_aplicado: 16,
            }
        );
    }

    #[test]
    fn test_11_rechazo_operacion_nueva_antes_de_60_segundos() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        let r1 = reservar_peticion_tmdb_at(&mut conn, "discover", true, now).unwrap();
        assert!(matches!(r1, ReservaTmdbResultado::Reservada { .. }));

        let r2 = reservar_peticion_tmdb_at(&mut conn, "discover", true, now + 59).unwrap();
        assert_eq!(
            r2,
            ReservaTmdbResultado::IntervaloOperacionNoCumplido {
                ultimo_inicio_at: now,
                proximo_disponible_at: now + 60,
            }
        );

        let r3 = reservar_peticion_tmdb_at(&mut conn, "discover", true, now + 60).unwrap();
        assert!(matches!(r3, ReservaTmdbResultado::Reservada { .. }));
    }

    #[test]
    fn test_12_peticiones_internas_permitidas_sin_esperar_60_segundos() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        let r1 = reservar_peticion_tmdb_at(&mut conn, "discover", true, now).unwrap();
        assert!(matches!(r1, ReservaTmdbResultado::Reservada { .. }));

        let r2 = reservar_peticion_tmdb_at(&mut conn, "details", false, now + 1).unwrap();
        assert_eq!(
            r2,
            ReservaTmdbResultado::Reservada {
                reserva_id: 2,
                peticiones_en_ventana: 2,
                restante_en_ventana: 16,
            }
        );

        // ultima_operacion_at sigue siendo el del inicio de operación
        let state = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(state.ultima_operacion_at, Some(now));
    }

    #[test]
    fn test_13_rechazo_reservas_ante_bloqueo_global_activo() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        registrar_bloqueo_tmdb_conn(&mut conn, now + 120, "HTTP 429").unwrap();

        let r_start = reservar_peticion_tmdb_at(&mut conn, "discover", true, now).unwrap();
        assert_eq!(
            r_start,
            ReservaTmdbResultado::BloqueoActivo {
                bloqueado_hasta: now + 120,
                motivo: Some("HTTP 429".to_string()),
            }
        );

        let r_internal = reservar_peticion_tmdb_at(&mut conn, "details", false, now + 10).unwrap();
        assert_eq!(
            r_internal,
            ReservaTmdbResultado::BloqueoActivo {
                bloqueado_hasta: now + 120,
                motivo: Some("HTTP 429".to_string()),
            }
        );

        // Expirado el bloqueo, vuelve a permitir reserva
        let r_after = reservar_peticion_tmdb_at(&mut conn, "discover", true, now + 120).unwrap();
        assert!(matches!(r_after, ReservaTmdbResultado::Reservada { .. }));
    }

    #[test]
    fn test_14_verificacion_atomica_bloqueo_y_presupuesto() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        // Llenar 17 peticiones en ventana (es_inicio_operacion requiere count <= 16)
        for i in 0..17 {
            conn.execute(
                "INSERT INTO tmdb_request_log (requested_at, endpoint_kind) VALUES (?1, 'seed')",
                params![now - 10 + i],
            )
            .unwrap();
        }

        // Fallo por presupuesto no debe actualizar ultima_operacion_at ni insertar log
        let r_fail = reservar_peticion_tmdb_at(&mut conn, "discover", true, now + 20).unwrap();
        assert!(matches!(
            r_fail,
            ReservaTmdbResultado::PresupuestoAgotado {
                peticiones_en_ventana: 17,
                limite_aplicado: 16
            }
        ));

        let state = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(state.ultima_operacion_at, None);

        let log_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tmdb_request_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(log_count, 17);
    }

    #[test]
    fn test_15_limpieza_marcas_mayores_a_900_segundos() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        // Antigua (> 900s) -> debe eliminarse
        conn.execute(
            "INSERT INTO tmdb_request_log (requested_at, endpoint_kind) VALUES (?1, 'old')",
            params![now - 901],
        )
        .unwrap();
        // Dentro de retención (<= 900s) pero fuera de ventana (300s) -> debe conservarse
        conn.execute(
            "INSERT INTO tmdb_request_log (requested_at, endpoint_kind) VALUES (?1, 'mid')",
            params![now - 500],
        )
        .unwrap();
        // Dentro de ventana (300s) -> debe conservarse y contarse
        conn.execute(
            "INSERT INTO tmdb_request_log (requested_at, endpoint_kind) VALUES (?1, 'recent')",
            params![now - 100],
        )
        .unwrap();

        let res = reservar_peticion_tmdb_at(&mut conn, "discover", true, now).unwrap();
        assert_eq!(
            res,
            ReservaTmdbResultado::Reservada {
                reserva_id: 4,
                peticiones_en_ventana: 2,
                restante_en_ventana: 16,
            }
        );

        let old_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tmdb_request_log WHERE endpoint_kind = 'old'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mid_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tmdb_request_log WHERE endpoint_kind = 'mid'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_count, 0);
        assert_eq!(mid_count, 1);
    }

    #[test]
    fn test_16_persistencia_y_consulta_cooldowns_y_errores() {
        let mut conn = setup_test_db();

        let errs1 = registrar_error_tmdb_conn(&mut conn, "timeout").unwrap();
        let errs2 = registrar_error_tmdb_conn(&mut conn, "502").unwrap();
        assert_eq!(errs1, 1);
        assert_eq!(errs2, 2);

        let st = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(st.ultimo_error.as_deref(), Some("502"));
        assert_eq!(st.errores_consecutivos, 2);

        restablecer_errores_tmdb_conn(&mut conn).unwrap();
        let st_reset = get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(st_reset.errores_consecutivos, 0);
        assert_eq!(st_reset.ultimo_error, None);

        registrar_cooldown_grupo_conn(
            &mut conn,
            GrupoContenido::PeliculasAnime,
            1_700_005_000,
            3,
            Some("Sin resultados nuevos"),
        )
        .unwrap();

        let cd = get_cooldown_grupo_conn(&conn, GrupoContenido::PeliculasAnime)
            .unwrap()
            .expect("cooldown debe existir");
        assert_eq!(cd.bloqueado_hasta, 1_700_005_000);
        assert_eq!(cd.fallos_sin_aporte, 3);
        assert_eq!(cd.motivo.as_deref(), Some("Sin resultados nuevos"));

        restablecer_cooldown_grupo_conn(&mut conn, GrupoContenido::PeliculasAnime).unwrap();
        let cd_after = get_cooldown_grupo_conn(&conn, GrupoContenido::PeliculasAnime).unwrap();
        assert!(cd_after.is_none());
    }

    #[test]
    fn test_17_persistencia_y_consulta_reintentos_posteres() {
        let mut conn = setup_test_db();
        let id = insert_contenido(&conn, "MOVIE", 0, "Poster Test", None, None, None);
        add_to_catalogo(&conn, id);
        let now = 1_700_000_000;

        let f1 = registrar_fallo_poster_at(&mut conn, id, "https://img/a.jpg", "404", now).unwrap();
        assert_eq!(f1.intentos, 1);
        assert_eq!(f1.estado, ESTADO_POSTER_PENDIENTE);
        assert_eq!(f1.proximo_reintento, Some(now + 300));

        // Todavía no vence a now+100
        let pend_antes = get_posters_pendientes_mantenimiento_at(&conn, now + 100, 10).unwrap();
        assert!(pend_antes.is_empty());

        // Vence a now+300
        let pend_despues = get_posters_pendientes_mantenimiento_at(&conn, now + 300, 10).unwrap();
        assert_eq!(pend_despues.len(), 1);
        assert_eq!(pend_despues[0].contenido_id, id);

        // Cambio de URL reinicia el estado
        let reiniciado =
            reiniciar_poster_retry_si_cambia_url_conn(&mut conn, id, "https://img/b.jpg").unwrap();
        assert!(reiniciado);
        assert!(get_poster_retry_state_conn(&conn, id).unwrap().is_none());

        // Agotar intentos con nueva URL
        for i in 1..=POSTER_MAX_INTENTOS {
            let res =
                registrar_fallo_poster_at(&mut conn, id, "https://img/b.jpg", "500", now + i)
                    .unwrap();
            if i < POSTER_MAX_INTENTOS {
                assert_eq!(res.estado, ESTADO_POSTER_PENDIENTE);
            } else {
                assert_eq!(res.estado, ESTADO_POSTER_AGOTADO);
                assert_eq!(res.proximo_reintento, None);
            }
        }

        // Éxito marca como resuelto
        registrar_exito_poster_at(&mut conn, id, now + 999).unwrap();
        let final_state = get_poster_retry_state_conn(&conn, id).unwrap().unwrap();
        assert_eq!(final_state.estado, ESTADO_POSTER_RESUELTO);
        assert_eq!(final_state.intentos, 0);
        assert_eq!(final_state.proximo_reintento, None);
    }

    #[test]
    fn test_d4_normalizar_poster_url_salida_y_archivo_local() {
        assert_eq!(normalizar_poster_url_salida(None), None);
        assert_eq!(normalizar_poster_url_salida(Some("".to_string())), None);
        assert_eq!(normalizar_poster_url_salida(Some("   \t\n ".to_string())), None);
        assert_eq!(
            normalizar_poster_url_salida(Some("  /abc.jpg  ".to_string())),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("abc.jpg".to_string())),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("https://cdn.example.com/p.jpg".to_string())),
            Some("https://cdn.example.com/p.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("http://cdn.example.com/p.jpg".to_string())),
            Some("http://cdn.example.com/p.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some(
                "https://image.tmdb.org/t/p/w500https://cdn.myanimelist.net/images/1.jpg"
                    .to_string()
            )),
            Some("https://cdn.myanimelist.net/images/1.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some(
                "http://image.tmdb.org/t/p/w500http://cdn.example.com/1.jpg".to_string()
            )),
            Some("http://cdn.example.com/1.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_url_salida(Some(
                "https://image.tmdb.org/t/p/w500https://image.tmdb.org/t/p/w500/xyz.jpg"
                    .to_string()
            )),
            Some("https://image.tmdb.org/t/p/w500/xyz.jpg".to_string())
        );

        let dir = std::env::temp_dir().join(format!(
            "queveohoy_d4_norm_test_{}_{}",
            std::process::id(),
            current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let id = 4242_i64;
        let local_path = dir.join(format!("{}.jpg", id));
        std::fs::write(&local_path, b"fakejpg").unwrap();

        let resolved = resolve_poster_salida(&dir, id, Some("/remoto.jpg".to_string()));
        assert_eq!(resolved, Some(local_path.to_string_lossy().to_string()));

        let resolved_missing = resolve_poster_salida(&dir, 9999, Some("/remoto.jpg".to_string()));
        assert_eq!(
            resolved_missing,
            Some("https://image.tmdb.org/t/p/w500/remoto.jpg".to_string())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_d4_normalizar_poster_url_rechaza_esquemas_no_admitidos() {
        assert_eq!(
            normalizar_poster_url_salida(Some("ftp://x.jpg".to_string())),
            None
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("file:///C:/posters/x.jpg".to_string())),
            None
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("data:image/png;base64,AAAA".to_string())),
            None
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("javascript:alert(1)".to_string())),
            None
        );
        assert_eq!(
            normalizar_poster_url_salida(Some("s3://bucket/x.jpg".to_string())),
            None
        );
        // Rutas relativas válidas siguen normalizándose con prefijo TMDB
        assert_eq!(
            normalizar_poster_url_salida(Some("sub/x.jpg".to_string())),
            Some("https://image.tmdb.org/t/p/w500/sub/x.jpg".to_string())
        );
    }

    #[test]
    fn test_d4_normalizar_poster_url_acepta_esquema_http_https_case_insensitive() {
        for url in [
            "http://example.com/poster.jpg",
            "https://example.com/poster.jpg",
            "HTTP://example.com/poster.jpg",
            "HTTPS://example.com/poster.jpg",
            "HtTpS://example.com/poster.jpg",
        ] {
            assert_eq!(
                normalizar_poster_url_salida(Some(url.to_string())),
                Some(url.to_string())
            );
        }
    }

    #[test]
    fn test_d4_get_posters_pendientes_mantenimiento_filtra_candidatas() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;

        // 1. Válida en catalogo_offline sin fila en poster_retry_state (sin sinopsis ni fecha)
        let id_ok_rel = insert_contenido(&conn, "MOVIE", 0, "Rel Path", Some("/a.jpg"), None, None);
        add_to_catalogo(&conn, id_ok_rel);

        // 2. Válida https en catalogo_offline y presente en omitidos_sesion (debe incluirse)
        let id_ok_http = insert_contenido(
            &conn,
            "SERIES",
            1,
            "Omitida",
            Some("https://cdn/b.jpg"),
            None,
            None,
        );
        add_to_catalogo(&conn, id_ok_http);
        conn.execute(
            "INSERT INTO omitidos_sesion (contenido_id, fecha) VALUES (?1, '2026-11-14')",
            params![id_ok_http],
        )
        .unwrap();

        // 3. En usuario_contenido -> excluida
        let id_usuario = insert_contenido(
            &conn,
            "MOVIE",
            0,
            "En Usuario",
            Some("/user.jpg"),
            None,
            None,
        );
        add_to_catalogo(&conn, id_usuario);
        conn.execute(
            "INSERT INTO usuario_contenido (contenido_id, estado) VALUES (?1, 'PENDIENTE')",
            params![id_usuario],
        )
        .unwrap();

        // 4. Solo en contenido, NO en catalogo_offline -> excluida
        let _id_solo_contenido = insert_contenido(
            &conn,
            "MOVIE",
            0,
            "Solo Contenido",
            Some("/solo.jpg"),
            None,
            None,
        );

        // 5. URL nula, vacía o inválida -> excluidas
        let id_null = insert_contenido(&conn, "MOVIE", 0, "Null", None, None, None);
        add_to_catalogo(&conn, id_null);
        let id_empty = insert_contenido(&conn, "MOVIE", 0, "Empty", Some("   "), None, None);
        add_to_catalogo(&conn, id_empty);
        let id_invalid = insert_contenido(&conn, "MOVIE", 0, "Invalid", Some("abc.jpg"), None, None);
        add_to_catalogo(&conn, id_invalid);

        // 6. Agotada -> excluida
        let id_agotada = insert_contenido(
            &conn,
            "MOVIE",
            0,
            "Agotada",
            Some("/agotada.jpg"),
            None,
            None,
        );
        add_to_catalogo(&conn, id_agotada);
        for i in 1..=POSTER_MAX_INTENTOS {
            registrar_fallo_poster_at(&mut conn, id_agotada, "/agotada.jpg", "404", now + i)
                .unwrap();
        }

        // 7. Resuelta -> excluida
        let id_resuelta = insert_contenido(
            &conn,
            "MOVIE",
            0,
            "Resuelta",
            Some("/resuelta.jpg"),
            None,
            None,
        );
        add_to_catalogo(&conn, id_resuelta);
        registrar_exito_poster_at(&mut conn, id_resuelta, now).unwrap();

        let pendientes = get_posters_pendientes_mantenimiento_at(&conn, now + 100, 20).unwrap();
        let ids: Vec<i64> = pendientes.iter().map(|p| p.contenido_id).collect();
        assert_eq!(ids, vec![id_ok_rel, id_ok_http]);
    }

    #[test]
    fn test_d4_backoff_agotado_no_elimina_ni_excluye_elegibilidad() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;
        let id = insert_contenido(
            &conn,
            "MOVIE",
            0,
            "Obra Sin Poster",
            Some("https://bad.example/404.jpg"),
            None,
            None,
        );
        add_to_catalogo(&conn, id);

        let expected_backoffs = [300_i64, 900, 3600, 21600];
        for (idx, backoff) in expected_backoffs.iter().enumerate() {
            let st = registrar_fallo_poster_at(
                &mut conn,
                id,
                "https://bad.example/404.jpg",
                "404",
                now,
            )
            .unwrap();
            assert_eq!(st.intentos, (idx + 1) as i64);
            assert_eq!(st.estado, ESTADO_POSTER_PENDIENTE);
            assert_eq!(st.proximo_reintento, Some(now + backoff));
        }

        let quinto = registrar_fallo_poster_at(
            &mut conn,
            id,
            "https://bad.example/404.jpg",
            "404",
            now,
        )
        .unwrap();
        assert_eq!(quinto.intentos, 5);
        assert_eq!(quinto.estado, ESTADO_POSTER_AGOTADO);
        assert_eq!(quinto.proximo_reintento, None);

        // Sigue en contenido y catalogo_offline, y no vuelve a la cola de reintentos
        let en_catalogo: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM catalogo_offline WHERE contenido_id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(en_catalogo, 1);

        let pend = get_posters_pendientes_mantenimiento_at(&conn, now + 999_999, 10).unwrap();
        assert!(pend.is_empty());

        // La lectura de URL almacenada no muta la base de datos
        let stored_url: Option<String> = conn
            .query_row(
                "SELECT poster_url FROM contenido WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored_url.as_deref(), Some("https://bad.example/404.jpg"));
    }

    #[test]
    fn test_d4_persistir_resultados_posters_minibatch() {
        let mut conn = setup_test_db();
        let now = 1_700_000_000;
        let id_ok = insert_contenido(&conn, "MOVIE", 0, "OK", Some("/ok.jpg"), None, None);
        let id_fail = insert_contenido(&conn, "MOVIE", 0, "Fail", Some("/fail.jpg"), None, None);
        add_to_catalogo(&conn, id_ok);
        add_to_catalogo(&conn, id_fail);

        let batch = vec![
            ResultadoDescargaPoster::Exito {
                contenido_id: id_ok,
            },
            ResultadoDescargaPoster::Fallo {
                contenido_id: id_fail,
                url: "/fail.jpg".to_string(),
                error: "HTTP 500".to_string(),
            },
        ];
        persistir_resultados_posters_at(&mut conn, &batch, now).unwrap();

        let st_ok = get_poster_retry_state_conn(&conn, id_ok).unwrap().unwrap();
        assert_eq!(st_ok.estado, ESTADO_POSTER_RESUELTO);

        let st_fail = get_poster_retry_state_conn(&conn, id_fail).unwrap().unwrap();
        assert_eq!(st_fail.estado, ESTADO_POSTER_PENDIENTE);
        assert_eq!(st_fail.intentos, 1);
        assert_eq!(st_fail.proximo_reintento, Some(now + 300));
    }
}
