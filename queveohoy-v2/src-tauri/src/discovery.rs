use rusqlite::params;
use serde_json::Value;
use rand::Rng;
use crate::db::get_connection;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static JIKAN_FALLOS_RECIENTES: OnceLock<Mutex<HashMap<i32, Instant>>> = OnceLock::new();
static ULTIMA_REQUEST_JIKAN: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

pub fn spawn_discovery_worker() {
    tauri::async_runtime::spawn(async {
        // Ejecución inmediata
        println!("[Worker] Iniciando worker de recolección...");
        
        // Purgar ítems rotos del catálogo offline
        if let Ok(conn) = crate::db::get_connection() {
            let res = conn.execute(
                "DELETE FROM catalogo_offline WHERE contenido_id IN (SELECT id FROM contenido WHERE poster_url IS NULL OR TRIM(poster_url) = '' OR sinopsis IS NULL OR TRIM(sinopsis) = '')",
                []
            );
            if let Ok(deleted) = res {
                if deleted > 0 {
                    println!("[Worker] Se purgaron {} ítems rotos del catálogo offline", deleted);
                }
            }
        }

        if let Err(e) = run_discovery_cycle().await {
            eprintln!("[Worker] Error inicial: {}", e);
        }

        loop {
            // Revisamos cada 60 segundos si necesitamos recargar
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            
            let count = {
                if let Ok(conn) = crate::db::get_connection() {
                    conn.query_row(&format!("SELECT COUNT(*) FROM catalogo_offline co LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= date('now', 'localtime', '-{} days') WHERE uc.estado IS NULL AND os.contenido_id IS NULL", crate::db::DIAS_EXCLUSION_SKIP), [], |row| row.get::<_, i64>(0)).unwrap_or_else(|e| { eprintln!("[Worker] ERROR en query de conteo: {:?}", e); 0 })
                } else {
                    100 // Fallback
                }
            };

            if count < 10 {
                println!("[Worker] Catálogo offline bajo ({}), reponiendo...", count);
                if let Err(e) = run_discovery_cycle().await {
                    eprintln!("[Worker] Error en el ciclo de recolección: {}", e);
                }
            }
        }
    });
}

async fn run_discovery_cycle() -> Result<(), String> {
    dotenvy::dotenv().ok();
    let tmdb_api_key = std::env::var("TMDB_API_KEY").unwrap_or_else(|_| "239957e2e77a70d1dd415586f3f551b0".to_string());
    let client = reqwest::Client::new();

    loop {
        let count = {
            let conn = crate::db::get_connection().map_err(|e| e.to_string())?;
            conn.query_row(&format!("SELECT COUNT(*) FROM catalogo_offline co LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= date('now', 'localtime', '-{} days') WHERE uc.estado IS NULL AND os.contenido_id IS NULL", crate::db::DIAS_EXCLUSION_SKIP), [], |row| row.get::<_, i64>(0)).unwrap_or_else(|e| { eprintln!("[Worker] ERROR en query de conteo: {:?}", e); 0 })
        };
        
        if count >= 100 {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            continue;
        }

        let (year, type_choice) = {
            let mut rng = rand::thread_rng();
            (rng.gen_range(1990..=2026), rng.gen_range(0..3))
        }; // 0: MOVIE, 1: TV, 2: ANIME
        
        let type_str = match type_choice {
            0 => "Película",
            1 => "Serie TV",
            _ => "Anime"
        };
        println!("[Worker] Iniciando búsqueda para año {}, tipo {}...", year, type_str);

        let res = match type_choice {
            0 => fetch_tmdb_discover(&client, &tmdb_api_key, "movie", year).await,
            1 => fetch_tmdb_discover(&client, &tmdb_api_key, "tv", year).await,
            2 => fetch_jikan_anime(&client, year).await,
            _ => Ok(0),
        };

        match res {
            Ok(insertados) => {
                let current_total = {
                    if let Ok(conn) = crate::db::get_connection() {
                        conn.query_row(&format!("SELECT COUNT(*) FROM catalogo_offline co LEFT JOIN usuario_contenido uc ON co.contenido_id = uc.contenido_id LEFT JOIN omitidos_sesion os ON co.contenido_id = os.contenido_id AND os.fecha >= date('now', 'localtime', '-{} days') WHERE uc.estado IS NULL AND os.contenido_id IS NULL", crate::db::DIAS_EXCLUSION_SKIP), [], |row| row.get::<_, i64>(0)).unwrap_or_else(|e| { eprintln!("[Worker] ERROR en query de conteo: {:?}", e); 0 })
                    } else {
                        count + insertados as i64
                    }
                };
                println!("[Worker] Insertados {} ítems en catalogo_offline. Total acumulado: {}", insertados, current_total);
            }
            Err(e) => {
                eprintln!("[Worker] Error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        
        // Gentle API delay
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    
    Ok(())
}

async fn fetch_tmdb_discover(client: &reqwest::Client, api_key: &str, media_type: &str, year: i32) -> Result<usize, String> {
    let year_param = if media_type == "tv" { "first_air_date_year" } else { "primary_release_year" };
    let init_url = format!("https://api.themoviedb.org/3/discover/{}?api_key={}&language=es-ES&include_adult=false&{}={}", media_type, api_key, year_param, year);
    
    let resp = client.get(&init_url).send().await.map_err(|e| e.to_string())?;
    println!("[Worker] HTTP Status: {}", resp.status());
    if !resp.status().is_success() {
        return Ok(0); // Ignore and continue
    }
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    
    let total_pages = json["total_pages"].as_i64().unwrap_or(1).min(20) as i32;
    if total_pages < 1 { return Ok(0); }
    
    let random_page = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=total_pages)
    };
    
    let url = format!("{}&page={}", init_url, random_page);
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    println!("[Worker] HTTP Status: {}", resp.status());
    if !resp.status().is_success() {
        return Ok(0);
    }
    
    let mut inserted_count = 0;
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(results) = json["results"].as_array() {
        let mut sample = results.clone();
        let to_take = {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            sample.shuffle(&mut rng);
            rng.gen_range(1..=3).min(sample.len())
        };
        
        for item in sample.into_iter().take(to_take) {
            let tmdb_id = item["id"].as_i64().unwrap_or(0);
            if tmdb_id == 0 { continue; }
            
            let mut es_anime = false;
            // Check genres for anime heuristic
            if let Some(genres) = item["genre_ids"].as_array() {
                if genres.iter().any(|g| g.as_i64() == Some(16)) {
                    if let Some(origin) = item["origin_country"].as_array() {
                        if origin.iter().any(|c| c.as_str() == Some("JP")) {
                            es_anime = true;
                        }
                    }
                }
            }

            let tipo = if media_type == "tv" { "TV" } else { "MOVIE" };

            // Status check for TV
            if tipo == "TV" && !es_anime {
                let detail_url = format!("https://api.themoviedb.org/3/tv/{}?api_key={}&language=es-ES", tmdb_id, api_key);
                if let Ok(det_resp) = client.get(&detail_url).send().await {
                    if let Ok(det_json) = det_resp.json::<Value>().await {
                        let status = det_json["status"].as_str().unwrap_or("");
                        if status != "Ended" && status != "Canceled" && status != "Cancelled" && status != "Miniseries" {
                            println!("[Worker] Descartando serie TMDB {} por status: {}", tmdb_id, status);
                            continue;
                        }
                    }
                }
            }

            let sinopsis = item["overview"].as_str().map(|s| s.trim().to_string());
            let poster_url = item["poster_path"].as_str().map(|s| s.trim().to_string());

            // REGLA DE SANIDAD: Si no hay póster o no hay sinopsis, descartar la obra.
            if poster_url.as_deref().unwrap_or("").is_empty() || sinopsis.as_deref().unwrap_or("").len() < 15 {
                println!("[Worker] Descartando TMDB {} por falta de poster o sinopsis vacía/corta", tmdb_id);
                continue;
            }

            // Traducir genre_ids TMDB → nombres en español
            let tmdb_genre_map: &[( i64, &str)] = &[
                (28, "Acción"), (12, "Aventura"), (16, "Animación"), (35, "Comedia"),
                (80, "Crimen"), (99, "Documental"), (18, "Drama"), (10751, "Familia"),
                (14, "Fantasía"), (36, "Historia"), (27, "Terror"), (10402, "Música"),
                (9648, "Misterio"), (10749, "Romance"), (878, "Ciencia Ficción"),
                (10770, "Película de TV"), (53, "Suspenso"), (10752, "Bélico"), (37, "Western"),
                (10759, "Acción y Aventura"), (10762, "Niños"), (10763, "Noticias"),
                (10764, "Reality"), (10765, "Sci-Fi y Fantasía"), (10766, "Soap"),
                (10767, "Talk Show"), (10768, "Guerra y Política"),
            ];
            let generos: Option<String> = if let Some(gids) = item["genre_ids"].as_array() {
                let nombres: Vec<&str> = gids.iter()
                    .filter_map(|g| g.as_i64())
                    .filter_map(|id| tmdb_genre_map.iter().find(|(gid, _)| *gid == id).map(|(_, name)| *name))
                    .take(4)
                    .collect();
                if nombres.is_empty() { None } else { Some(format!("[{}]", nombres.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(","))) }
            } else { None };

            let titulo = item["title"].as_str().or(item["name"].as_str()).unwrap_or("Sin Título").to_string();
            let titulo_original = item["original_title"].as_str().or(item["original_name"].as_str()).map(|s| s.to_string());
            let fecha_estreno = item["release_date"].as_str().or(item["first_air_date"].as_str()).map(|s| s.to_string());

            if insert_to_catalog(Some(tmdb_id), None, tipo, es_anime, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos) {
                inserted_count += 1;
            }
        }
    }
    Ok(inserted_count)
}

async fn fetch_jikan_anime(client: &reqwest::Client, year: i32) -> Result<usize, String> {
    {
        let fallos_lock = JIKAN_FALLOS_RECIENTES.get_or_init(|| Mutex::new(HashMap::new()));
        let fallos = fallos_lock.lock().unwrap();
        if let Some(ultimo_fallo) = fallos.get(&year) {
            if ultimo_fallo.elapsed() < std::time::Duration::from_secs(300) {
                println!("[Worker] Saltando year={} (falló hace menos de 5 min)", year);
                return Ok(0);
            }
        }
    }

    {
        let ultima_lock = ULTIMA_REQUEST_JIKAN.get_or_init(|| Mutex::new(None));
        let espera = {
            let ultima = ultima_lock.lock().unwrap();
            if let Some(t) = *ultima {
                let transcurrido = t.elapsed();
                if transcurrido < std::time::Duration::from_secs(1) {
                    Some(std::time::Duration::from_secs(1) - transcurrido)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(d) = espera {
            tokio::time::sleep(d).await;
        }

        let mut ultima = ultima_lock.lock().unwrap();
        *ultima = Some(Instant::now());
    }

    // Top anime filtering by year using season endpoint or simple search
    let seasons = ["winter", "spring", "summer", "fall"];
    let season = seasons[rand::thread_rng().gen_range(0..4)];
    let url = format!("https://api.jikan.moe/v4/seasons/{}/{}?limit=25&sfw=true", year, season);
    
    const MAX_REINTENTOS: u32 = 3;
    const DELAYS_MS: [u64; 3] = [500, 1500, 3000];

    let mut resp_opt = None;
    for intento in 0..MAX_REINTENTOS {
        let resp_result = client.get(&url).timeout(std::time::Duration::from_secs(8)).send().await;
        match resp_result {
            Ok(r) if r.status().is_success() => {
                resp_opt = Some(r);
                break;
            }
            Ok(r) if matches!(r.status().as_u16(), 429 | 502 | 503 | 504) => {
                println!("[Worker] Jikan {} (intento {}/{}), reintentando en {}ms...", 
                    r.status(), intento + 1, MAX_REINTENTOS, DELAYS_MS[intento as usize]);
                tokio::time::sleep(std::time::Duration::from_millis(DELAYS_MS[intento as usize])).await;
                continue;
            }
            Ok(r) => {
                println!("[Worker] API Jikan devolvió HTTP {}. Ignorando silenciosamente.", r.status());
                return Ok(0);
            }
            Err(e) => {
                println!("[Worker] API Jikan inalcanzable ({}). Ignorando silenciosamente.", e);
                return Ok(0);
            }
        }
    }
    
    let resp = match resp_opt {
        Some(r) => r,
        None => {
            println!("[Worker] Jikan agotó reintentos para year={}. Ignorando silenciosamente.", year);
            let fallos_lock = JIKAN_FALLOS_RECIENTES.get_or_init(|| Mutex::new(HashMap::new()));
            fallos_lock.lock().unwrap().insert(year, Instant::now());
            return Ok(0);
        }
    };
    
    let mut inserted_count = 0;
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(data) = json["data"].as_array() {
        if data.is_empty() { return Ok(0); }
        
        let mut sample = data.clone();
        let to_take = {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            sample.shuffle(&mut rng);
            rng.gen_range(1..=3).min(sample.len())
        };

        for item in sample.into_iter().take(to_take) {
            let mal_id = item["mal_id"].as_i64().unwrap_or(0);
            if mal_id == 0 { continue; }
            
            let raw_type = item["type"].as_str().unwrap_or("TV");
            let tipo = if raw_type == "Movie" { "MOVIE" } else { "TV" };
            
            // Jikan status filter for TV
            if tipo == "TV" {
                let status = item["status"].as_str().unwrap_or("");
                if status != "Finished Airing" {
                    println!("[Worker] Descartando anime MAL {} por status: {}", mal_id, status);
                    continue;
                }
            }

            let sinopsis = item["synopsis"].as_str().map(|s| s.trim().to_string());
            let poster_url = item["images"]["jpg"]["large_image_url"].as_str().map(|s| s.trim().to_string());

            // REGLA DE SANIDAD: Si no hay póster o no hay sinopsis, descartar la obra.
            if poster_url.as_deref().unwrap_or("").is_empty() || sinopsis.as_deref().unwrap_or("").len() < 15 {
                println!("[Worker] Descartando Jikan MAL {} por falta de poster o sinopsis vacía/corta", mal_id);
                continue;
            }

            let titulo = item["title"].as_str().unwrap_or("Sin Título").to_string();
            let titulo_original = item["title_japanese"].as_str().map(|s| s.to_string());
            
            // Format aired from string
            let fecha_estreno = item["aired"]["from"].as_str().map(|s| s.split('T').next().unwrap_or("").to_string());

            // Capturar géneros de Jikan
            let generos: Option<String> = if let Some(gens) = item["genres"].as_array() {
                let nombres: Vec<String> = gens.iter()
                    .filter_map(|g| g["name"].as_str())
                    .take(4)
                    .map(|n| format!("\"{}\"", n))
                    .collect();
                if nombres.is_empty() { None } else { Some(format!("[{}]", nombres.join(","))) }
            } else { None };

            if insert_to_catalog(None, Some(mal_id), tipo, true, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos) {
                inserted_count += 1;
            }
        }
    }
    Ok(inserted_count)
}

fn insert_to_catalog(
    tmdb_id: Option<i64>, 
    mal_id: Option<i64>, 
    tipo: &str, 
    es_anime: bool, 
    titulo: String, 
    titulo_original: Option<String>,
    sinopsis: Option<String>,
    fecha_estreno: Option<String>,
    poster_url: Option<String>,
    generos: Option<String>,
) -> bool {
    if let Ok(conn) = get_connection() {
        let exists: Option<i64> = conn.query_row(
            "SELECT id FROM contenido WHERE (tmdb_id = ?1 AND tmdb_id IS NOT NULL) OR (mal_id = ?2 AND mal_id IS NOT NULL)",
            params![tmdb_id, mal_id],
            |row| row.get(0)
        ).ok();

        let contenido_id = if let Some(id) = exists {
            // Actualizar generos si aún no los tiene
            let _ = conn.execute(
                "UPDATE contenido SET generos = ?1 WHERE id = ?2 AND (generos IS NULL OR generos = '')",
                params![generos, id]
            );
            id
        } else {
            let res = conn.execute(
                "INSERT INTO contenido (tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![tmdb_id, mal_id, tipo, if es_anime { 1 } else { 0 }, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos]
            );
            if let Ok(_) = res {
                conn.last_insert_rowid()
            } else {
                return false;
            }
        };

        let res = conn.execute("INSERT OR IGNORE INTO catalogo_offline (contenido_id) VALUES (?1)", params![contenido_id]);
        if res.is_ok() && res.unwrap() > 0 {
            return true;
        }
    }
    false
}
