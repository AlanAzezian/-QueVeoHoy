use rusqlite::params;
use serde_json::Value;
use crate::db::get_connection;
use crate::models::ResultadoBusqueda;
use crate::discovery::TMDB_GENRE_MAP;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::{OnceLock, Mutex};
use std::collections::HashSet;

static FETCH_LOCKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();

fn get_fetch_locks() -> &'static Mutex<HashSet<i64>> {
    FETCH_LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub async fn fetch_and_cache_temporadas(contenido_id: i64, tmdb_id: Option<i64>, mal_id: Option<i64>, es_anime: bool) -> Result<String, String> {
    let mut wait_retries = 30; // 15 seconds
    loop {
        {
            let mut locks = get_fetch_locks().lock().unwrap();
            if !locks.contains(&contenido_id) {
                locks.insert(contenido_id);
                break;
            }
        }
        if wait_retries <= 0 {
            eprintln!("fetch_in_progress timeout para contenido_id: {}", contenido_id);
            return Err("fetch_in_progress".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        wait_retries -= 1;
        
        // Si otro hilo ya terminó, la metadata debería estar disponible
        if let Ok(Some(json)) = crate::db::get_metadata_json(contenido_id) {
            if json != "{}" {
                return Ok(json);
            }
        }
    }
    
    struct FetchGuard {
        id: i64,
    }
    impl Drop for FetchGuard {
        fn drop(&mut self) {
            if let Ok(mut locks) = get_fetch_locks().lock() {
                locks.remove(&self.id);
            }
        }
    }
    
    let _guard = FetchGuard { id: contenido_id };

    let result = if es_anime && mal_id.is_some() {
        fetch_jikan_anime(contenido_id, mal_id.unwrap()).await
    } else if let Some(t_id) = tmdb_id {
        fetch_tmdb_series(contenido_id, t_id).await
    } else {
        Err("No tmdb_id or mal_id provided".to_string())
    };

    let conn = crate::db::get_connection().map_err(|e| e.to_string())?;
    match result {
        Ok(v) => {
            if let Err(e) = conn.execute("UPDATE contenido SET last_metadata_fetch_error = NULL WHERE id = ?1", rusqlite::params![contenido_id]) {
                eprintln!("Error limpiando last_metadata_fetch_error para contenido {}: {}", contenido_id, e);
            }
            Ok(v)
        }
        Err(e) => {
            if let Err(err) = conn.execute("UPDATE contenido SET last_metadata_fetch_error = datetime('now', 'localtime') WHERE id = ?1", rusqlite::params![contenido_id]) {
                eprintln!("Error marcando last_metadata_fetch_error para contenido {}: {}", contenido_id, err);
            }
            Err(e)
        }
    }
}

async fn fetch_jikan_anime(contenido_id: i64, mal_id: i64) -> Result<String, String> {
    println!("[Jikan] Iniciando fetch para contenido_id: {}, mal_id: {}", contenido_id, mal_id);
    let mut page = 1;
    let mut has_next_page = true;
    let mut episodes = Vec::new();
    let client = reqwest::Client::new();

    while has_next_page {
        let url = format!("https://api.jikan.moe/v4/anime/{}/episodes?page={}", mal_id, page);
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
        
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            println!("[Jikan] Rate limit alcanzado. Esperando 1 segundo...");
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        
        if !resp.status().is_success() {
            return Err(format!("Jikan API Error: {}", resp.status()));
        }

        let json: Value = resp.json().await.map_err(|e| e.to_string())?;
        
        if let Some(data) = json["data"].as_array() {
            for ep in data {
                if let Some(mal_ep_id) = ep["mal_id"].as_i64() {
                    let title = ep["title"].as_str().unwrap_or("").to_string();
                    let aired = ep["aired"].as_str().unwrap_or("").to_string();
                    episodes.push((mal_ep_id, title, aired));
                }
            }
        }
        
        has_next_page = json["pagination"]["has_next_page"].as_bool().unwrap_or(false);
        if has_next_page {
            page += 1;
            sleep(Duration::from_millis(350)).await; // ~3 requests per second limit
        }
    }

    let conn = get_connection().map_err(|e| e.to_string())?;
    
    // Insert single season for anime
    let temporada_id: i64 = conn.query_row(
        "INSERT INTO tv_temporadas (contenido_id, season_number, name) VALUES (?1, ?2, ?3) ON CONFLICT(contenido_id, season_number) DO UPDATE SET name=excluded.name RETURNING id",
        params![contenido_id, 1, "Temporada 1"],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    for (mal_ep_id, title, aired) in &episodes {
        conn.execute(
            "INSERT INTO tv_episodios (temporada_id, episode_number, episode_absolute, name, air_date) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(temporada_id, episode_number) DO UPDATE SET episode_absolute=excluded.episode_absolute, name=excluded.name, air_date=excluded.air_date",
            params![temporada_id, mal_ep_id, mal_ep_id, title, aired]
        ).map_err(|e| e.to_string())?;
    }

    let mut map = serde_json::Map::new();
    map.insert("1".to_string(), Value::Number(episodes.len().into()));
    
    println!("[Jikan] OK! Insertados {} episodios.", episodes.len());
    Ok(Value::Object(map).to_string())
}

async fn fetch_tmdb_series(contenido_id: i64, tmdb_id: i64) -> Result<String, String> {
    println!("[TMDB] Iniciando fetch para contenido_id: {}, tmdb_id: {}", contenido_id, tmdb_id);
    dotenvy::dotenv().ok();
    let api_key = std::env::var("TMDB_API_KEY").unwrap_or_else(|_| "239957e2e77a70d1dd415586f3f551b0".to_string());
    
    let client = reqwest::Client::new();
    let url = format!("https://api.themoviedb.org/3/tv/{}?api_key={}&language=es-ES", tmdb_id, api_key);
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !resp.status().is_success() {
        return Err(format!("TMDB API Error: {}", resp.status()));
    }
    
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;

    // Actualizar total_temporadas si está disponible
    if let Some(total_seasons) = json["number_of_seasons"].as_i64() {
        if let Ok(conn) = get_connection() {
            let _ = conn.execute("UPDATE contenido SET total_temporadas = ?1 WHERE id = ?2 AND total_temporadas IS NULL", params![total_seasons, contenido_id]);
        }
    }

    let seasons = json["seasons"].as_array().ok_or_else(|| "No seasons array found".to_string())?;
    
    let mut seasons_map = serde_json::Map::new();

    for s in seasons {
        if let (Some(season_number), Some(episode_count)) = (s["season_number"].as_i64(), s["episode_count"].as_i64()) {
            if season_number > 0 && episode_count > 0 {
                let name = s["name"].as_str().unwrap_or("").to_string();
                let overview = s["overview"].as_str().unwrap_or("").to_string();
                let air_date = s["air_date"].as_str().unwrap_or("").to_string();

                let temporada_id: i64 = {
                    let conn = get_connection().map_err(|e| e.to_string())?;
                    conn.query_row(
                        "INSERT INTO tv_temporadas (contenido_id, season_number, name, overview, air_date) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(contenido_id, season_number) DO UPDATE SET name=excluded.name, overview=excluded.overview, air_date=excluded.air_date RETURNING id",
                        params![contenido_id, season_number, name, overview, air_date],
                        |row| row.get(0)
                    ).map_err(|e| e.to_string())?
                };
                
                // Fetch episodes for this season
                let season_url = format!("https://api.themoviedb.org/3/tv/{}/season/{}?api_key={}&language=es-ES", tmdb_id, season_number, api_key);
                if let Ok(season_resp) = client.get(&season_url).send().await {
                    if let Ok(season_json) = season_resp.json::<Value>().await {
                        if let Some(episodes) = season_json["episodes"].as_array() {
                            let conn = get_connection().map_err(|e| e.to_string())?;
                            for ep in episodes {
                                if let Some(episode_number) = ep["episode_number"].as_i64() {
                                    let ep_name = ep["name"].as_str().unwrap_or("").to_string();
                                    let ep_overview = ep["overview"].as_str().unwrap_or("").to_string();
                                    let ep_air_date = ep["air_date"].as_str().unwrap_or("").to_string();
                                    let ep_runtime = ep["runtime"].as_i64().unwrap_or(0);
                                    
                                    let _ = conn.execute(
                                        "INSERT INTO tv_episodios (temporada_id, episode_number, name, overview, air_date, runtime) VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(temporada_id, episode_number) DO UPDATE SET name=excluded.name, overview=excluded.overview, air_date=excluded.air_date, runtime=excluded.runtime",
                                        params![temporada_id, episode_number, ep_name, ep_overview, ep_air_date, ep_runtime]
                                    );
                                }
                            }
                        }
                    }
                }
                
                seasons_map.insert(season_number.to_string(), Value::Number(episode_count.into()));
            }
        }
    }

    println!("[TMDB] OK! Terminado.");
    Ok(Value::Object(seasons_map).to_string())
}

#[derive(serde::Serialize)]
pub struct BackfillReport {
    total: i32,
    completados: i32,
    fallidos: i32,
    errores: Vec<String>,
}

pub async fn backfill_metadata_cache() -> Result<BackfillReport, String> {
    let to_process = {
        let conn = get_connection().map_err(|e| e.to_string())?;
        
        // Select items that are TV or anime and don't have seasons yet
        let mut stmt = conn.prepare(
            "SELECT c.id, c.tmdb_id, c.mal_id, c.es_anime, c.titulo FROM contenido c
             LEFT JOIN tv_temporadas tt ON c.id = tt.contenido_id
             WHERE (c.tipo = 'TV' OR c.es_anime = 1) AND tt.id IS NULL"
        ).map_err(|e| e.to_string())?;
        
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut to_process = Vec::new();
        
        while let Some(row) = rows.next().unwrap_or(None) {
            let id: i64 = row.get(0).unwrap_or(0);
            let tmdb_id: Option<i64> = row.get(1).unwrap_or(None);
            let mal_id: Option<i64> = row.get(2).unwrap_or(None);
            let es_anime: bool = row.get::<_, Option<i64>>(3).unwrap_or(Some(0)).unwrap_or(0) == 1;
            let titulo: String = row.get(4).unwrap_or_default();
            to_process.push((id, tmdb_id, mal_id, es_anime, titulo));
        }
        to_process
    };
    
    let mut completados = 0;
    let mut fallidos = 0;
    let mut errores = Vec::new();
    let total = to_process.len() as i32;
    
    for (id, tmdb_id, mal_id, es_anime, titulo) in to_process {
        match fetch_and_cache_temporadas(id, tmdb_id, mal_id, es_anime).await {
            Ok(_) => completados += 1,
            Err(e) => {
                fallidos += 1;
                errores.push(format!("{} (ID {}): {}", titulo, id, e));
            }
        }
    }
    
    Ok(BackfillReport {
        total,
        completados,
        fallidos,
        errores,
    })
}



pub async fn buscar_online(query: String, tipo_filtro: Option<String>) -> Result<Vec<ResultadoBusqueda>, String> {
    let query_clean = query.trim().to_string();
    if query_clean.is_empty() {
        return Ok(Vec::new());
    }

    let filtro = tipo_filtro.as_deref().unwrap_or("todos").to_lowercase();
    dotenvy::dotenv().ok();
    let api_key = std::env::var("TMDB_API_KEY").unwrap_or_else(|_| "239957e2e77a70d1dd415586f3f551b0".to_string());

    let client = reqwest::Client::builder()
        .user_agent("QueVeoHoy/2.0 (https://github.com/alan/queveohoy)")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let tmdb_task = {
        let client = client.clone();
        let api_key = api_key.clone();
        let query_str = query_clean.clone();
        async move {
            let url = "https://api.themoviedb.org/3/search/multi";
            let resp = client.get(url)
                .query(&[
                    ("api_key", api_key.as_str()),
                    ("query", query_str.as_str()),
                    ("language", "es-ES"),
                    ("include_adult", "false"),
                ])
                .send()
                .await;
            match resp {
                Ok(r) if r.status().is_success() => r.json::<Value>().await.ok(),
                _ => None,
            }
        }
    };

    let should_query_jikan = filtro == "todos" || filtro == "anime" || filtro == "all";
    let jikan_task = {
        let client = client.clone();
        let query_str = query_clean.clone();
        async move {
            if !should_query_jikan {
                return None;
            }
            let url = "https://api.jikan.moe/v4/anime";
            let resp = client.get(url)
                .query(&[
                    ("q", query_str.as_str()),
                    ("sfw", "true"),
                    ("limit", "15"),
                ])
                .timeout(Duration::from_secs(6))
                .send()
                .await;
            match resp {
                Ok(r) if r.status().is_success() => r.json::<Value>().await.ok(),
                _ => None,
            }
        }
    };

    let tmdb_handle = tauri::async_runtime::spawn(tmdb_task);
    let jikan_handle = tauri::async_runtime::spawn(jikan_task);
    let tmdb_json = tmdb_handle.await.ok().flatten();
    let jikan_json = jikan_handle.await.ok().flatten();

    let mut resultados: Vec<ResultadoBusqueda> = Vec::new();
    let mut seen_titles: HashSet<String> = HashSet::new();

    if let Some(json) = tmdb_json {
        if let Some(results) = json["results"].as_array() {
            for item in results {
                let media_type = item["media_type"].as_str().unwrap_or("");
                if media_type != "movie" && media_type != "tv" {
                    continue;
                }

                let tmdb_id = item["id"].as_i64().unwrap_or(0);
                if tmdb_id == 0 {
                    continue;
                }

                let tipo = if media_type == "tv" { "TV" } else { "MOVIE" };

                let mut es_anime = false;
                if let Some(genres) = item["genre_ids"].as_array() {
                    if genres.iter().any(|g| g.as_i64() == Some(16)) {
                        if let Some(origin) = item["origin_country"].as_array() {
                            if origin.iter().any(|c| c.as_str() == Some("JP")) {
                                es_anime = true;
                            }
                        } else if item["original_language"].as_str() == Some("ja") {
                            es_anime = true;
                        }
                    }
                }

                if (filtro == "peliculas" || filtro == "movie") && tipo != "MOVIE" {
                    continue;
                }
                if (filtro == "series" || filtro == "tv") && (tipo != "TV" || es_anime) {
                    continue;
                }
                if filtro == "anime" && !es_anime {
                    continue;
                }

                let titulo = item["title"].as_str().or(item["name"].as_str()).unwrap_or("").trim().to_string();
                if titulo.is_empty() {
                    continue;
                }

                let poster_path = item["poster_path"].as_str().unwrap_or("").trim();
                if poster_path.is_empty() {
                    continue;
                }
                let poster_url = Some(format!("https://image.tmdb.org/t/p/w500{}", poster_path));

                let backdrop_path = item["backdrop_path"].as_str().map(|s| s.trim()).filter(|s| !s.is_empty());
                let backdrop_url = backdrop_path.map(|bp| {
                    if bp.starts_with("http://") || bp.starts_with("https://") {
                        bp.to_string()
                    } else if bp.starts_with('/') {
                        format!("https://image.tmdb.org/t/p/w1280{}", bp)
                    } else {
                        format!("https://image.tmdb.org/t/p/w1280/{}", bp)
                    }
                });

                let sinopsis = item["overview"].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                let titulo_original = item["original_title"].as_str().or(item["original_name"].as_str()).map(|s| s.trim().to_string());
                let fecha_estreno = item["release_date"].as_str().or(item["first_air_date"].as_str()).map(|s| s.trim().to_string());

                let generos: Option<String> = if let Some(gids) = item["genre_ids"].as_array() {
                    let nombres: Vec<&str> = gids.iter()
                        .filter_map(|g| g.as_i64())
                        .filter_map(|id| TMDB_GENRE_MAP.iter().find(|(gid, _)| *gid == id).map(|(_, name)| *name))
                        .take(4)
                        .collect();
                    if nombres.is_empty() { None } else { Some(format!("[{}]", nombres.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(","))) }
                } else { None };

                seen_titles.insert(titulo.to_lowercase());

                resultados.push(ResultadoBusqueda {
                    tmdb_id: Some(tmdb_id),
                    mal_id: None,
                    tipo: tipo.to_string(),
                    es_anime,
                    titulo,
                    titulo_original,
                    poster_url,
                    backdrop_url,
                    sinopsis,
                    fecha_estreno,
                    generos,
                    ya_en_biblioteca: false,
                    estado_actual: None,
                });
            }
        }
    }

    if let Some(json) = jikan_json {
        if let Some(data) = json["data"].as_array() {
            for item in data {
                let mal_id = item["mal_id"].as_i64().unwrap_or(0);
                if mal_id == 0 {
                    continue;
                }

                let raw_type = item["type"].as_str().unwrap_or("TV");
                let tipo = if raw_type == "Movie" { "MOVIE" } else { "TV" };

                if (filtro == "peliculas" || filtro == "movie") && tipo != "MOVIE" {
                    continue;
                }
                if (filtro == "series" || filtro == "tv") && tipo != "TV" {
                    continue;
                }

                let titulo = item["title"].as_str().unwrap_or("").trim().to_string();
                if titulo.is_empty() {
                    continue;
                }

                let title_lower = titulo.to_lowercase();
                if seen_titles.contains(&title_lower) {
                    continue;
                }
                seen_titles.insert(title_lower);

                let poster_url = item["images"]["jpg"]["large_image_url"]
                    .as_str()
                    .or(item["images"]["jpg"]["image_url"].as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                if poster_url.is_none() {
                    continue;
                }

                let sinopsis = item["synopsis"].as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                let titulo_original = item["title_japanese"].as_str().map(|s| s.trim().to_string());
                let fecha_estreno = item["aired"]["from"].as_str().map(|s| s.split('T').next().unwrap_or("").to_string());

                let generos: Option<String> = if let Some(gens) = item["genres"].as_array() {
                    let nombres: Vec<String> = gens.iter()
                        .filter_map(|g| g["name"].as_str())
                        .take(4)
                        .map(|n| format!("\"{}\"", n))
                        .collect();
                    if nombres.is_empty() { None } else { Some(format!("[{}]", nombres.join(","))) }
                } else { None };

                resultados.push(ResultadoBusqueda {
                    tmdb_id: None,
                    mal_id: Some(mal_id),
                    tipo: tipo.to_string(),
                    es_anime: true,
                    titulo,
                    titulo_original,
                    poster_url,
                    backdrop_url: None,
                    sinopsis,
                    fecha_estreno,
                    generos,
                    ya_en_biblioteca: false,
                    estado_actual: None,
                });
            }
        }
    }

    if let Ok(conn) = get_connection() {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT uc.estado
             FROM contenido c
             JOIN usuario_contenido uc ON c.id = uc.contenido_id
             WHERE (?1 IS NOT NULL AND c.tmdb_id = ?1) OR (?2 IS NOT NULL AND c.mal_id = ?2)
             LIMIT 1"
        ) {
            for res in &mut resultados {
                let estado: Option<String> = stmt.query_row(
                    params![res.tmdb_id, res.mal_id],
                    |row| row.get(0)
                ).ok();

                if let Some(est) = estado {
                    res.ya_en_biblioteca = true;
                    res.estado_actual = Some(est);
                } else {
                    res.ya_en_biblioteca = false;
                    res.estado_actual = None;
                }
            }
        }
    }

    Ok(resultados)
}

pub async fn fetch_and_apply_enrichment(contenido_id: i64) -> Result<crate::models::Contenido, String> {
    let (tmdb_id_opt, _mal_id, tipo, _es_anime) = crate::db::get_contenido_enrichment_params(contenido_id)
        .map_err(|e| format!("Error leyendo parámetros de enriquecimiento para {}: {}", contenido_id, e))?;

    let tmdb_id = match tmdb_id_opt {
        Some(id) => id,
        None => {
            // Contenido sin tmdb_id (ej: anime de Jikan sin TMDB)
            return crate::db::get_recomendacion_by_contenido_id(contenido_id)
                .map(|r| r.contenido)
                .map_err(|e| e.to_string());
        }
    };

    dotenvy::dotenv().ok();
    let api_key = std::env::var("TMDB_API_KEY").unwrap_or_else(|_| "239957e2e77a70d1dd415586f3f551b0".to_string());
    let client = reqwest::Client::builder()
        .user_agent("QueVeoHoy/2.0 (https://github.com/alan/queveohoy)")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let is_tv = tipo == "TV";
    let endpoint = if is_tv { "tv" } else { "movie" };
    let url = format!(
        "https://api.themoviedb.org/3/{}/{}?api_key={}&language=es-ES&append_to_response=credits,watch/providers",
        endpoint, tmdb_id, api_key
    );

    let record_error = |cid: i64, msg: &str| {
        eprintln!("[TMDB Enrichment] {}", msg);
        if let Ok(conn) = crate::db::get_connection() {
            if let Err(err) = conn.execute(
                "UPDATE contenido SET last_metadata_fetch_error = datetime('now', 'localtime') WHERE id = ?1",
                rusqlite::params![cid],
            ) {
                eprintln!("[TMDB Enrichment] Error marcando last_metadata_fetch_error para contenido {}: {}", cid, err);
            }
        }
    };

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            record_error(contenido_id, &format!("Error de red consultando TMDB {} ({}): {}", tmdb_id, tipo, e));
            return crate::db::get_recomendacion_by_contenido_id(contenido_id)
                .map(|r| r.contenido)
                .map_err(|err| err.to_string());
        }
    };

    if !resp.status().is_success() {
        record_error(contenido_id, &format!("TMDB devolvió status {} para {} (ID {})", resp.status(), tmdb_id, contenido_id));
        return crate::db::get_recomendacion_by_contenido_id(contenido_id)
            .map(|r| r.contenido)
            .map_err(|err| err.to_string());
    }

    let json: Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            record_error(contenido_id, &format!("Error parseando JSON para TMDB {}: {}", tmdb_id, e));
            return crate::db::get_recomendacion_by_contenido_id(contenido_id)
                .map(|r| r.contenido)
                .map_err(|err| err.to_string());
        }
    };

    let mut director: Option<String> = None;
    let mut creadores: Option<String> = None;
    let mut total_temporadas: Option<i64> = None;
    let status: Option<String> = json["status"].as_str().map(|s| s.trim().to_string());
    let mut last_air_date: Option<String> = None;
    let mut duracion: Option<i64> = None;

    if is_tv {
        // Creadores de serie (array de strings)
        if let Some(created_by) = json["created_by"].as_array() {
            let names: Vec<String> = created_by
                .iter()
                .filter_map(|c| c["name"].as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
            if !names.is_empty() {
                creadores = serde_json::to_string(&names).ok();
            }
        }
        total_temporadas = json["number_of_seasons"].as_i64();
        last_air_date = json["last_air_date"].as_str().map(|s| s.trim().to_string());
    } else {
        // Director de película (búsqueda en credits.crew job == 'Director')
        if let Some(crew) = json["credits"]["crew"].as_array() {
            let directores: Vec<String> = crew
                .iter()
                .filter(|m| m["job"].as_str() == Some("Director"))
                .filter_map(|m| m["name"].as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
            if !directores.is_empty() {
                director = Some(directores.join(", "));
            }
        }
        duracion = json["runtime"].as_i64().filter(|&r| r > 0);
    }

    // Reparto principal (hasta 10 integrantes)
    let mut cast_items = Vec::new();
    if let Some(cast) = json["credits"]["cast"].as_array() {
        for (i, actor) in cast.iter().take(10).enumerate() {
            let nombre = actor["name"].as_str().unwrap_or("").trim().to_string();
            let personaje = actor["character"].as_str().unwrap_or("").trim().to_string();
            let orden = actor["order"].as_i64().unwrap_or(i as i64);
            if !nombre.is_empty() {
                cast_items.push(crate::models::MiembroReparto {
                    nombre,
                    personaje,
                    orden,
                });
            }
        }
    }
    let reparto = if !cast_items.is_empty() {
        serde_json::to_string(&cast_items).ok()
    } else {
        None
    };

    // Plataformas de streaming para Argentina (AR)
    let parse_providers = |list_opt: Option<&Value>, tipo_cat: &str| -> Vec<crate::models::ProveedorStreaming> {
        let mut list = Vec::new();
        if let Some(items) = list_opt.and_then(|v| v.as_array()) {
            for it in items {
                let id = it["provider_id"].as_i64().unwrap_or(0);
                let nombre = it["provider_name"].as_str().unwrap_or("").trim().to_string();
                let logo_path = it["logo_path"].as_str().map(|s| s.trim().to_string());
                if !nombre.is_empty() {
                    list.push(crate::models::ProveedorStreaming {
                        id,
                        nombre,
                        logo_path,
                        tipo: tipo_cat.to_string(),
                    });
                }
            }
        }
        list
    };

    let (flatrate, rent, buy) = if let Some(ar_node) = json["watch/providers"]["results"]["AR"].as_object() {
        (
            parse_providers(ar_node.get("flatrate"), "flatrate"),
            parse_providers(ar_node.get("rent"), "rent"),
            parse_providers(ar_node.get("buy"), "buy"),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };

    // Respuesta exitosa: siempre persistimos explícitamente el objeto de plataformas (incluso si está vacío)
    let plat_struct = crate::models::PlataformasStreaming { flatrate, rent, buy };
    let plataformas_str = serde_json::to_string(&plat_struct)
        .unwrap_or_else(|_| r#"{"flatrate":[],"rent":[],"buy":[]}"#.to_string());
    let plataformas = Some(plataformas_str);

    let providers_updated_at: Option<String> = if let Ok(conn) = crate::db::get_connection() {
        conn.query_row("SELECT datetime('now', 'localtime')", [], |row| row.get(0)).ok()
    } else {
        None
    };

    // Persistir en SQLite
    if let Err(e) = crate::db::update_contenido_enrichment(
        contenido_id,
        director.as_deref(),
        creadores.as_deref(),
        reparto.as_deref(),
        total_temporadas,
        plataformas.as_deref(),
        providers_updated_at.as_deref(),
        status.as_deref(),
        last_air_date.as_deref(),
        duracion,
    ) {
        eprintln!("[TMDB Enrichment] Error actualizando BD para {}: {}", contenido_id, e);
    }

    crate::db::get_recomendacion_by_contenido_id(contenido_id)
        .map(|r| r.contenido)
        .map_err(|e| format!("Error recuperando contenido enriquecido: {}", e))
}
