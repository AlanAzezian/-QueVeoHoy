use rusqlite::{Connection, Result, params};

pub const DIAS_EXCLUSION_SKIP: i64 = 10;
use std::path::PathBuf;
use crate::models::{Contenido, UsuarioContenido, ProgresoSerie, RecomendacionHoy};
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
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == "last_metadata_fetch_error" { has_error_col = true; }
        if name == "generos" { has_generos_col = true; }
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

pub fn get_recomendacion_de_hoy(state: &DbState) -> Result<Option<RecomendacionHoy>> {
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

    let omitted_en_progreso = state.omitted_en_progreso_session.lock().unwrap();
    let omitted_en_progreso_list = omitted_en_progreso.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    let omit_progreso_clause = if omitted_en_progreso.is_empty() {
        "".to_string()
    } else {
        format!("AND c.id NOT IN ({})", omitted_en_progreso_list)
    };

    let omitted = state.omitted_in_session.lock().unwrap();
    let omitted_list = omitted.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    let omit_clause = if omitted.is_empty() {
        "".to_string()
    } else {
        format!("AND c.id NOT IN ({})", omitted_list)
    };

    // 2. Try to find a series "en progreso"
    let mut stmt = conn.prepare(&format!(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                uc.id as uc_id, uc.estado,
                ps.id as ps_id, ps.temporada_actual, ps.episodio_actual, ps.episodio_absoluto_actual,
                tm.temporadas_json,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-24 hours')) as fetch_recently_failed,
                c.generos
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

    // 2. Fallback to catalogo_offline
    let mut stmt = conn.prepare(&format!(
        "SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, 
                c.titulo_original, c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en,
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-24 hours')) as fetch_recently_failed,
                c.generos
         FROM catalogo_offline co
         JOIN contenido c ON co.contenido_id = c.id
         LEFT JOIN usuario_contenido uc ON c.id = uc.contenido_id
         WHERE uc.estado IS NULL {}",
         omit_clause
    ))?;

    let mut available_contents = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
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

        available_contents.push(Contenido {
            id: Some(id),
            tmdb_id: row.get(1)?,
            mal_id: row.get(2)?,
            tipo: row.get(3)?,
            es_anime: row.get::<_, Option<i64>>(4)?.unwrap_or(0) == 1,
            titulo: row.get(5)?,
            titulo_original: row.get(6)?,
            poster_url,
            sinopsis: row.get(8)?,
            fecha_estreno: row.get(9)?,
            creado_en: row.get(10)?,
            fetch_recently_failed: row.get::<_, Option<i64>>(11).ok().flatten().map(|v| v == 1),
            generos: row.get::<_, Option<String>>(12).ok().flatten(),
        });
    }

    println!("Found {} contents in catalogo_offline", available_contents.len());

    if available_contents.is_empty() {
        println!("catalogo_offline está vacío. Devolviendo null para que Frontend muestre 'Buscando...'");
        return Ok(None);
    }

    // Pick random
    let mut rng = rand::thread_rng();
    let selected = available_contents.choose(&mut rng).cloned();
    
    let res = selected.map(|c| RecomendacionHoy {
        contenido: c,
        estado: None,
        progreso: None,
    });
    println!("[DEBUG Hoy] Recomendación encontrada en catalogo_offline: {:?}", res);
    Ok(res)
}

fn map_recomendacion(row: &rusqlite::Row) -> rusqlite::Result<RecomendacionHoy> {
    let id: i64 = row.get(0)?;
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

    let contenido = Contenido {
        id: Some(id),
        tmdb_id: row.get(1)?,
        mal_id: row.get(2)?,
        tipo: row.get(3)?,
        es_anime: row.get::<_, Option<i64>>(4)?.unwrap_or(0) == 1,
        titulo: row.get(5)?,
        titulo_original: row.get(6)?,
        poster_url,
        sinopsis: row.get(8)?,
        fecha_estreno: row.get(9)?,
        creado_en: row.get(10)?,
        fetch_recently_failed: row.get::<usize, Option<i64>>(18).ok().flatten().map(|v| v == 1),
        generos: row.get::<usize, Option<String>>(19).ok().flatten(),
    };
    
    let uc_id: i64 = row.get(11)?;
    let estado: String = row.get(12)?;
    
    let progreso = if let Ok(ps_id) = row.get::<_, i64>(13) {
        Some(ProgresoSerie {
            id: Some(ps_id),
            usuario_contenido_id: uc_id,
            temporada_actual: row.get(14)?,
            episodio_actual: row.get(15)?,
            episodio_absoluto_actual: row.get(16)?,
            actualizado_en: None,
            metadata_temporadas: row.get::<_, Option<String>>(17).ok().flatten(),
        })
    } else {
        None
    };
    
    Ok(RecomendacionHoy {
        contenido,
        estado: Some(estado),
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
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-24 hours')) as fetch_recently_failed,
                c.generos
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
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-24 hours')) as fetch_recently_failed,
                c.generos
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
                (c.last_metadata_fetch_error IS NOT NULL AND c.last_metadata_fetch_error > datetime('now', 'localtime', '-24 hours')) as fetch_recently_failed,
                c.generos
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
                tm.temporadas_json
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
        return Ok(crate::models::AvanceResultado { temporada: temp_actual, episodio: ep_actual + 1, terminada: is_terminada });
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

    tx.commit()?;
    Ok(crate::models::AvanceResultado {
        temporada: temp_actual,
        episodio: ep_actual,
        terminada: is_terminada
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
