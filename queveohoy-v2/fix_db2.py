# -*- coding: utf-8 -*-
import io

tail = '''fn internal_actualizar_progreso(conn: &Connection, contenido_id: i64, temporada: i64, episodio: i64, fallback_accion: &str) -> Result<bool> {
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
                    eprintln!("Fallo al realizar fetch_and_cache_temporadas en avanzar_episodio (Error: {}), se usará modo optimista.", e);
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
        let is_terminada = internal_actualizar_progreso(&tx, contenido_id, temp_actual, ep_actual + 1, "AJUSTE_PROGRESO")?;
        tx.commit()?;
        return Ok(crate::models::AvanceResultado { temporada: temp_actual, episodio: ep_actual + 1, terminada: is_terminada });
    }

    let logged_action = "AJUSTE_PROGRESO";
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
             VALUES (?1, datetime('now', 'localtime'), 'AJUSTE_PROGRESO', 1)
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
'''

with io.open('src-tauri/src/db.rs', 'r', encoding='utf-8') as f:
    text = f.read()

idx = text.find('fn internal_actualizar_progreso(conn: &Connection')
if idx != -1:
    head = text[:idx]
    with io.open('src-tauri/src/db.rs', 'w', encoding='utf-8') as f:
        f.write(head + tail)
    print("Done")
else:
    print("Could not find start")
