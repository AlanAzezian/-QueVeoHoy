mod models;
pub mod db;
mod tmdb;
mod discovery;

use std::sync::Mutex;

use tauri::State;
use db::DbState;
use models::RecomendacionHoy;

#[tauri::command]
async fn fetch_and_cache_temporadas(contenido_id: i64, tmdb_id: Option<i64>, mal_id: Option<i64>, es_anime: bool) -> Result<String, String> {
    tmdb::fetch_and_cache_temporadas(contenido_id, tmdb_id, mal_id, es_anime).await
}

#[tauri::command]
async fn fetch_metadata_temporadas(id: i64) -> Result<String, String> {
    if let Ok(Some(cached_json)) = db::get_metadata_json(id) {
        if cached_json != "{}" {
            return Ok(cached_json);
        }
    }
    
    let (tmdb_id, mal_id, es_anime) = db::get_metadata_fetch_params(id).map_err(|e| {
        eprintln!("DB ERROR (fetch_metadata_temporadas): {:?}", e);
        e.to_string()
    })?;
    tmdb::fetch_and_cache_temporadas(id, tmdb_id, mal_id, es_anime).await
}

#[tauri::command]
async fn backfill_metadata_cache() -> Result<tmdb::BackfillReport, String> {
    tmdb::backfill_metadata_cache().await
}

#[tauri::command]
async fn get_recommendation(
    filtros: Option<models::FiltrosRecomendacion>,
    generos: Option<Vec<String>>,
    state: State<'_, DbState>
) -> Result<Option<RecomendacionHoy>, String> {
    let effective_filtros = filtros.or_else(|| {
        generos.map(|g| models::FiltrosRecomendacion {
            tipos: None,
            generos: Some(g),
        })
    });
    let rec = db::get_recomendacion_de_hoy(&state, effective_filtros).map_err(|e| {
        eprintln!("DB ERROR: {:?}", e);
        e.to_string()
    })?;

    if let Some(mut r) = rec {
        let should_enrich = r.contenido.id.is_some()
            && r.contenido.tmdb_id.is_some()
            && r.contenido.fetch_recently_failed != Some(true)
            && (
                r.contenido.providers_updated_at.is_none()
                || r.contenido.backdrop_url.is_none()
                || (r.contenido.tipo == "MOVIE" && (r.contenido.duracion.is_none() || r.contenido.duracion == Some(0)))
                || (r.contenido.tipo != "MOVIE" && (r.contenido.status.is_none() || r.contenido.total_temporadas.is_none()))
            );

        if should_enrich {
            if let Some(cid) = r.contenido.id {
                let enrich_task = tokio::spawn(async move {
                    tmdb::fetch_and_apply_enrichment(cid).await
                });
                match tokio::time::timeout(std::time::Duration::from_millis(3500), enrich_task).await {
                    Ok(Ok(Ok(enriched))) => {
                        r.contenido = enriched;
                    }
                    Ok(Ok(Err(e))) => {
                        eprintln!("[get_recommendation] Error en enriquecimiento preventivo: {}", e);
                    }
                    Ok(Err(join_err)) => {
                        eprintln!("[get_recommendation] Error de join en enriquecimiento: {}", join_err);
                    }
                    Err(_) => {
                        eprintln!("[get_recommendation] Timeout (3.5s) en enriquecimiento preventivo para contenido {} (continuará en background)", cid);
                    }
                }
            }
        }
        Ok(Some(r))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn mark_as_omitted(id: i64, registrar_en_bitacora: bool, state: State<'_, DbState>) -> Result<(), String> {
    let estado = db::get_estado_contenido(id).unwrap_or_else(|_| "".to_string());
    
    if estado != "en_progreso" {
        let mut omitted = state.omitted_in_session.lock().unwrap();
        omitted.insert(id);
        let _ = db::add_omitido_sesion(id);
    }
    
    if registrar_en_bitacora {
        db::registrar_accion(id, "SIGUIENTE").map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
fn get_en_progreso() -> Result<Vec<RecomendacionHoy>, String> {
    db::get_en_progreso().map_err(|e| {
        eprintln!("DB ERROR (en progreso): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn get_biblioteca() -> Result<Vec<RecomendacionHoy>, String> {
    db::get_biblioteca().map_err(|e| {
        eprintln!("DB ERROR (biblioteca): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn get_historial() -> Result<Vec<RecomendacionHoy>, String> {
    db::get_historial().map_err(|e| {
        eprintln!("DB ERROR (historial): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn get_terminadas() -> Result<Vec<RecomendacionHoy>, String> {
    db::get_terminadas().map_err(|e| {
        eprintln!("DB ERROR (terminadas): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn update_estado(contenido_id: i64, nuevo_estado: &str) -> Result<(), String> {
    db::update_estado(contenido_id, nuevo_estado).map_err(|e| {
        eprintln!("DB ERROR (update_estado): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn actualizar_progreso(contenido_id: i64, temporada: i64, episodio: i64) -> Result<(), String> {
    db::actualizar_progreso(contenido_id, temporada, episodio).map_err(|e| {
        eprintln!("DB ERROR (actualizar_progreso): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn avanzar_episodio(contenido_id: i64) -> Result<models::AvanceResultado, String> {
    db::avanzar_episodio(contenido_id).map_err(|e| {
        eprintln!("DB ERROR (avanzar_episodio): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn omitir_todos_en_progreso(state: tauri::State<'_, DbState>) -> Result<(), String> {
    db::omitir_todos_en_progreso(&state).map_err(|e| {
        eprintln!("DB ERROR (omitir_todos_en_progreso): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn omitir_en_progreso(id: i64, state: tauri::State<'_, DbState>) -> Result<(), String> {
    db::omitir_en_progreso(&state, id).map_err(|e| {
        eprintln!("DB ERROR (omitir_en_progreso): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn get_catalogo() -> Result<Vec<models::ResultadoBusqueda>, String> {
    db::get_catalogo().map_err(|e| {
        eprintln!("DB ERROR (get_catalogo): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
fn abandonar_multiples(ids: Vec<i64>) -> Result<(), String> {
    db::abandonar_multiples(ids).map_err(|e| {
        eprintln!("DB ERROR (abandonar_multiples): {:?}", e);
        e.to_string()
    })
}

#[tauri::command]
async fn buscar_online(query: String, tipo_filtro: Option<String>) -> Result<Vec<models::ResultadoBusqueda>, String> {
    tmdb::buscar_online(query, tipo_filtro).await
}

#[tauri::command]
async fn guardar_obra_busqueda(obra: models::ObraBusquedaPayload, estado: String) -> Result<models::RecomendacionHoy, String> {
    db::guardar_obra_busqueda(obra, estado).await
}

#[tauri::command]
async fn enriquecer_contenido(id: i64) -> Result<models::Contenido, String> {
    tmdb::fetch_and_apply_enrichment(id).await
}



#[tauri::command]
fn get_generos_disponibles_hoy() -> Result<Vec<models::GeneroConteo>, String> {
    db::get_generos_disponibles_hoy().map_err(|e| {
        eprintln!("DB ERROR (get_generos_disponibles_hoy): {:?}", e);
        e.to_string()
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = db::init_db() {
        eprintln!("Error inicializando BD: {}", e);
    }

    // Iniciar worker de discovery en background
    discovery::spawn_discovery_worker();

    let (omitted, last_date) = db::get_omitidos_recientes(db::DIAS_EXCLUSION_SKIP).unwrap_or_else(|_| (std::collections::HashSet::new(), "".to_string()));

    tauri::Builder::default()
        .manage(DbState {
            omitted_in_session: Mutex::new(omitted),
            omitted_en_progreso_session: Mutex::new(std::collections::HashSet::new()),
            ultimo_reset_fecha: Mutex::new(last_date),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_recommendation,
            mark_as_omitted,
            get_en_progreso,
            get_biblioteca,
            get_historial,
            get_terminadas,
            update_estado,
            actualizar_progreso,
            avanzar_episodio,
            fetch_and_cache_temporadas,
            fetch_metadata_temporadas,
            backfill_metadata_cache,
            omitir_todos_en_progreso,
            omitir_en_progreso,
            get_catalogo,
            abandonar_multiples,
            get_generos_disponibles_hoy,
            buscar_online,
            guardar_obra_busqueda,
            enriquecer_contenido
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
