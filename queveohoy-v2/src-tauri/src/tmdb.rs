use rusqlite::params;
use serde_json::Value;
use crate::db::get_connection;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::{OnceLock, Mutex};
use std::collections::HashSet;

static FETCH_LOCKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();

fn get_fetch_locks() -> &'static Mutex<HashSet<i64>> {
    FETCH_LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub async fn fetch_and_cache_temporadas(contenido_id: i64, tmdb_id: Option<i64>, mal_id: Option<i64>, es_anime: bool) -> Result<String, String> {
    {
        let mut locks = get_fetch_locks().lock().unwrap();
        if locks.contains(&contenido_id) {
            eprintln!("fetch_in_progress para contenido_id: {}", contenido_id);
            return Err("fetch_in_progress".to_string());
        }
        locks.insert(contenido_id);
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


