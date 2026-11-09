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

    // Purga controlada para forzar el rellenado de géneros en offline
    let _ = conn.execute(
        "DELETE FROM catalogo_offline WHERE contenido_id IN (SELECT id FROM contenido WHERE generos IS NULL OR generos = '')",
        []
    );

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

    Ok(())
}

pub fn get_recomendacion_de_hoy(state: &DbState, generos: Option<Vec<String>>) -> Result<Option<RecomendacionHoy>> {
    let conn = get_connection()?;
    
    // 1. Verificar cambio de día calendario y resetear omit_clause
    let today_str: String = conn.query_row("SELECT date('now', 'localtime')", [], |row| row.get(0))?;
    {
        let mut last_date = state.ultimo_reset_fecha.lock().unwrap();
        if *last_date != today_str {
            let threshold_date: String = conn.query_row("SELECT date('now', 'localtime', ?1)", params![format!("-{} days", DIAS_EXCLUSION_SKIP)], |row| row.get(0)).unwrap_or_else(|_| today_str.clone());
            let _ = conn.execute("DELETE FROM omitidos_sesion WHERE fecha < ?", params![threshold_date]);
            
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

    // 2. Try to find a series "en progreso"
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
         WHERE uc.estado = 'en_progreso' {}",
         omit_progreso_clause
    ))?;

    let mut in_progress_contents = Vec::new();
    let mut rows = stmt.query([])?;
    
    while let Some(row) = rows.next()? {
        in_progress_contents.push(map_recomendacion(&row)?);
    }

    if !in_progress_contents.is_empty() {
        let mut rng = rand::thread_rng();
        let selected = in_progress_contents.choose(&mut rng).cloned();
        println!("[DEBUG Hoy] Recomendación encontrada en_progreso: {:?}", selected);
        return Ok(selected);
    }

    // Helper closure to query available contents
    let get_available = |conn: &Connection, ignore_omit: bool, active_generos: Option<&Vec<String>>| -> Result<Vec<Contenido>> {
        let current_omit_clause = if ignore_omit { "".to_string() } else { omit_clause.clone() };
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
             current_omit_clause
        ))?;
        
        let mut results = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let generos_str: Option<String> = row.get(12).ok().flatten();
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
            
            // Check genres if provided
            if let Some(target_gens) = active_generos {
                if !target_gens.is_empty() {
                    let mut has_match = false;
                    if let Some(g_str) = &generos_str {
                        for tg in target_gens {
                            if g_str.contains(tg) {
                                has_match = true;
                                break;
                            }
                        }
                    }
                    if !has_match {
                        continue;
                    }
                }
            }

            let raw_poster: Option<String> = row.get(7)?;
            let poster_url = raw_poster.map(|path| {
                let home_dir = dirs::home_dir()
                    .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
                    .unwrap_or_else(|| PathBuf::from(""));
                let full_path = home_dir.join(".queveohoy").join("cache_posters").join(format!("{}.jpg", id));
                if full_path.exists() {
                    full_path.to_string_lossy().to_string()
                } else {
                    format!("https://image.tmdb.org/t/p/w500{}", path)
                }
            });

            results.push(Contenido {
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

    // 2. Query catalogo_offline with omit and genres
    let mut available_contents = get_available(&conn, false, generos.as_ref())?;

    // Check sanity: If < 15 virgin contents in DB without genre filter, wake up worker
    let raw_virgin_count = get_available(&conn, false, None)?.len();
    if raw_virgin_count < 15 {
        println!("[DEBUG Hoy] Stock virgen es {} (< 15), despertando worker...", raw_virgin_count);
        crate::discovery::trigger_discovery();
    }

    // Fallback 1: Clear omitted_in_session and retry (with genre filter)
    if available_contents.is_empty() {
        println!("[DEBUG Hoy] Fallback 1: Catálogo vacío con omitidos, limpiando omitted_in_session...");
        state.omitted_in_session.lock().unwrap().clear();
        available_contents = get_available(&conn, true, generos.as_ref())?;
    }

    // Fallback 2: Ignore genre filter
    if available_contents.is_empty() {
        println!("[DEBUG Hoy] Fallback 2: Catálogo aún vacío, ignorando filtro de género...");
        available_contents = get_available(&conn, true, None)?;
    }

    if available_contents.is_empty() {
        println!("[DEBUG Hoy] catalogo_offline está vacío incluso tras los fallbacks. Devolviendo null.");
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
    let poster_url = raw_poster.map(|path| {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path;
        }
        let home_dir = dirs::home_dir()
            .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
            .unwrap_or_else(|| PathBuf::from(""));
        let full_path = home_dir.join(".queveohoy").join("cache_posters").join(format!("{}.jpg", id));
        if full_path.exists() {
            full_path.to_string_lossy().to_string()
        } else if path.starts_with('/') {
            format!("https://image.tmdb.org/t/p/w500{}", path)
        } else {
            format!("https://image.tmdb.org/t/p/w500/{}", path)
        }
    });

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

    let home_dir = dirs::home_dir()
        .or_else(|| std::env::var("USERPROFILE").map(PathBuf::from).ok())
        .unwrap_or_else(|| PathBuf::from(""));
    let cache_dir = home_dir.join(".queveohoy").join("cache_posters");

    let resolve_poster = |id: i64, raw_path: Option<String>| -> Option<String> {
        raw_path.map(|path| {
            if path.starts_with("http://") || path.starts_with("https://") {
                path
            } else {
                let full_path = cache_dir.join(format!("{}.jpg", id));
                if full_path.exists() {
                    full_path.to_string_lossy().to_string()
                } else if path.starts_with('/') {
                    format!("https://image.tmdb.org/t/p/w500{}", path)
                } else {
                    path
                }
            }
        })
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
