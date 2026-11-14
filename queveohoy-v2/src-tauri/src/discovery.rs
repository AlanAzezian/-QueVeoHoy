use rusqlite::params;
use serde_json::Value;
use rand::Rng;
use crate::db::get_connection;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::sync::Notify;

#[allow(dead_code)]
static JIKAN_FALLOS_RECIENTES: OnceLock<Mutex<HashMap<i32, Instant>>> = OnceLock::new();
#[allow(dead_code)]
static ULTIMA_REQUEST_JIKAN: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

static DISCOVERY_NOTIFY: OnceLock<Notify> = OnceLock::new();
static WORKER_SPAWNED: AtomicBool = AtomicBool::new(false);

fn get_discovery_notify() -> &'static Notify {
    DISCOVERY_NOTIFY.get_or_init(Notify::new)
}

pub const TMDB_GENRE_MAP: &[(i64, &str)] = &[
    (28, "Acción"), (12, "Aventura"), (16, "Animación"), (35, "Comedia"),
    (80, "Crimen"), (99, "Documental"), (18, "Drama"), (10751, "Familia"),
    (14, "Fantasía"), (36, "Historia"), (27, "Terror"), (10402, "Música"),
    (9648, "Misterio"), (10749, "Romance"), (878, "Ciencia Ficción"),
    (10770, "Película de TV"), (53, "Suspenso"), (10752, "Bélico"), (37, "Western"),
    (10759, "Acción y Aventura"), (10762, "Niños"), (10763, "Noticias"),
    (10764, "Reality"), (10765, "Sci-Fi y Fantasía"), (10766, "Soap"),
    (10767, "Talk Show"), (10768, "Guerra y Política"),
];

pub fn trigger_discovery() {
    println!("[Worker] Notificación de trabajo enviada al worker único.");
    get_discovery_notify().notify_one();
}

pub fn spawn_discovery_worker() {
    if WORKER_SPAWNED.swap(true, Ordering::SeqCst) {
        eprintln!("[Worker] Advertencia: spawn_discovery_worker ya fue iniciado. Ignorando llamada duplicada.");
        return;
    }

    tauri::async_runtime::spawn(async {
        println!("[Worker] Iniciando worker único de descubrimiento...");
        
        // Nota Etapa 5.9.1B: Se retiró la purga automática de catalogo_offline por falta de póster o sinopsis.
        // Los metadatos incompletos no deben retirar una obra del catálogo offline.

        run_discovery_worker_loop().await;
    });
}

pub const PAUSA_TRABAJO_LOCAL_SECS: u64 = 7;
pub const PAUSA_MINIMA_ENTRE_CICLOS_SECS: u64 = 15;
pub const BACKOFF_ERROR_SECS: u64 = 45;
pub const INTERVALO_MANTENIMIENTO_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct SnapshotPlanificador {
    pub inventario: crate::models::ConteoPorGrupo,
    pub candidatas: crate::models::ConteoPorGrupo,
    pub worker_state: crate::models::TmdbWorkerState,
    pub cooldowns: Vec<crate::models::TmdbGroupCooldown>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoCiclo {
    ErrorSqlite(String),
    TrabajoLocalRealizado {
        grupo: crate::models::GrupoContenido,
        recuperadas: usize,
        quedan_candidatas_locales: bool,
    },
    TrabajoExternoRealizado {
        grupo: crate::models::GrupoContenido,
        insertadas: usize,
        quedan_candidatas_locales: bool,
    },
    SinTrabajoLocalPendiente {
        grupos_pendientes_tmdb: Vec<crate::models::GrupoContenido>,
    },
    SoloMantenimientoPosters {
        posters_procesados: usize,
    },
    SinTrabajoPendiente,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RitmoEspera {
    ErrorBackoff {
        pausa_obligatoria_secs: u64,
        espera_maxima_notify_secs: u64,
    },
    TrabajoLocalContinuo {
        pausa_secs: u64,
    },
    Reposo {
        pausa_minima_secs: u64,
        espera_maxima_notify_secs: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct PlanificadorGrupos {
    pub cursor_seleccion: usize,
    pub ultimo_grupo_atendido: Option<crate::models::GrupoContenido>,
    pub sin_progreso_en_ciclo: [bool; 4],
}

impl PlanificadorGrupos {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iniciar_ciclo(&mut self) {
        self.sin_progreso_en_ciclo = [false; 4];
    }

    pub fn seleccionar_siguiente_grupo(
        &self,
        inventario: &crate::models::ConteoPorGrupo,
        candidatas: &crate::models::ConteoPorGrupo,
    ) -> Option<(usize, crate::models::GrupoContenido, usize)> {
        let grupos = crate::models::GrupoContenido::all();
        let inicio = self.cursor_seleccion % grupos.len();

        for offset in 0..grupos.len() {
            let idx = (inicio + offset) % grupos.len();
            if self.sin_progreso_en_ciclo[idx] {
                continue;
            }
            let grupo = grupos[idx];
            let lote = calcular_lote_local(inventario.get(grupo));
            if lote == 0 {
                continue;
            }
            if candidatas.get(grupo) <= 0 {
                continue;
            }
            return Some((idx, grupo, lote));
        }

        None
    }

    pub fn marcar_grupo_atendido(&mut self, idx: usize, grupo: crate::models::GrupoContenido) {
        self.ultimo_grupo_atendido = Some(grupo);
        self.cursor_seleccion = (idx + 1) % 4;
    }

    pub fn marcar_sin_progreso(&mut self, idx: usize, grupo: GrupoContenidoAlias) {
        self.sin_progreso_en_ciclo[idx] = true;
        self.ultimo_grupo_atendido = Some(grupo);
        self.cursor_seleccion = (idx + 1) % 4;
    }

    pub fn hay_trabajo_local_disponible(
        &self,
        inventario: &crate::models::ConteoPorGrupo,
        candidatas: &crate::models::ConteoPorGrupo,
    ) -> bool {
        crate::models::GrupoContenido::all().iter().any(|&grupo| {
            calcular_lote_local(inventario.get(grupo)) > 0 && candidatas.get(grupo) > 0
        })
    }

    pub fn grupos_con_deficit_sin_candidatas(
        &self,
        inventario: &crate::models::ConteoPorGrupo,
        candidatas: &crate::models::ConteoPorGrupo,
    ) -> Vec<crate::models::GrupoContenido> {
        crate::models::GrupoContenido::all()
            .into_iter()
            .filter(|&grupo| {
                inventario.get(grupo) < crate::db::OBJETIVO_INVENTARIO_POR_GRUPO
                    && candidatas.get(grupo) <= 0
            })
            .collect()
    }
}

type GrupoContenidoAlias = crate::models::GrupoContenido;

pub fn calcular_lote_local(inventario_actual: i64) -> usize {
    let objetivo = crate::db::OBJETIVO_INVENTARIO_POR_GRUPO;
    if inventario_actual >= objetivo {
        return 0;
    }
    let maximo = match inventario_actual {
        i64::MIN..=149 => 50,
        150..=249 => 35,
        250..=324 => 25,
        325..=374 => 15,
        _ => 0,
    };
    let deficit = (objetivo - inventario_actual).max(0);
    maximo.min(deficit) as usize
}

pub fn calcular_ritmo_siguiente(resultado: &ResultadoCiclo) -> RitmoEspera {
    match resultado {
        ResultadoCiclo::ErrorSqlite(_) => RitmoEspera::ErrorBackoff {
            pausa_obligatoria_secs: BACKOFF_ERROR_SECS,
            espera_maxima_notify_secs: INTERVALO_MANTENIMIENTO_SECS,
        },
        ResultadoCiclo::TrabajoLocalRealizado {
            quedan_candidatas_locales: true,
            ..
        }
        | ResultadoCiclo::TrabajoExternoRealizado {
            quedan_candidatas_locales: true,
            ..
        } => RitmoEspera::TrabajoLocalContinuo {
            pausa_secs: PAUSA_TRABAJO_LOCAL_SECS,
        },
        ResultadoCiclo::TrabajoLocalRealizado {
            quedan_candidatas_locales: false,
            ..
        }
        | ResultadoCiclo::TrabajoExternoRealizado {
            quedan_candidatas_locales: false,
            ..
        }
        | ResultadoCiclo::SoloMantenimientoPosters { .. }
        | ResultadoCiclo::SinTrabajoLocalPendiente { .. }
        | ResultadoCiclo::SinTrabajoPendiente => RitmoEspera::Reposo {
            pausa_minima_secs: PAUSA_MINIMA_ENTRE_CICLOS_SECS,
            espera_maxima_notify_secs: INTERVALO_MANTENIMIENTO_SECS,
        },
    }
}

async fn leer_snapshot_planificador_blocking() -> Result<SnapshotPlanificador, String> {
    tokio::task::spawn_blocking(|| -> Result<SnapshotPlanificador, String> {
        let inventario = crate::db::get_inventario_por_grupo()
            .map_err(|e| format!("Error leyendo inventario por grupo: {}", e))?;
        let candidatas = crate::db::get_candidatas_locales_por_grupo()
            .map_err(|e| format!("Error leyendo candidatas locales por grupo: {}", e))?;
        let worker_state = crate::db::get_tmdb_worker_state()
            .map_err(|e| format!("Error leyendo estado global TMDB: {}", e))?;
        let cooldowns = crate::db::get_todos_cooldowns_grupo()
            .map_err(|e| format!("Error leyendo cooldowns por grupo: {}", e))?;
        Ok(SnapshotPlanificador {
            inventario,
            candidatas,
            worker_state,
            cooldowns,
        })
    })
    .await
    .map_err(|e| format!("Error JoinError en spawn_blocking (snapshot): {}", e))?
}

async fn recuperar_local_blocking(
    grupo: crate::models::GrupoContenido,
    limite: usize,
) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || -> Result<usize, String> {
        crate::db::recuperar_obras_locales_por_grupo(grupo, limite)
            .map_err(|e| format!("Error recuperando obras locales ({:?}): {}", grupo, e))
    })
    .await
    .map_err(|e| format!("Error JoinError en spawn_blocking (recuperar_local): {}", e))?
}

#[cfg(test)]
pub async fn ejecutar_ciclo_planificador_con<FSnap, FSnapFut, FRec, FRecFut>(
    planificador: &mut PlanificadorGrupos,
    mut leer_snapshot: FSnap,
    mut recuperar_local: FRec,
) -> ResultadoCiclo
where
    FSnap: FnMut() -> FSnapFut,
    FSnapFut: std::future::Future<Output = Result<SnapshotPlanificador, String>>,
    FRec: FnMut(crate::models::GrupoContenido, usize) -> FRecFut,
    FRecFut: std::future::Future<Output = Result<usize, String>>,
{
    planificador.iniciar_ciclo();

    let mut snapshot = match leer_snapshot().await {
        Ok(s) => s,
        Err(e) => return ResultadoCiclo::ErrorSqlite(e),
    };

    while let Some((idx, grupo, lote)) =
        planificador.seleccionar_siguiente_grupo(&snapshot.inventario, &snapshot.candidatas)
    {
        match recuperar_local(grupo, lote).await {
            Err(e) => return ResultadoCiclo::ErrorSqlite(e),
            Ok(recuperadas) if recuperadas > 0 => {
                planificador.marcar_grupo_atendido(idx, grupo);
                let nuevo_snapshot = match leer_snapshot().await {
                    Ok(s) => s,
                    Err(e) => return ResultadoCiclo::ErrorSqlite(e),
                };
                let quedan_candidatas_locales = planificador.hay_trabajo_local_disponible(
                    &nuevo_snapshot.inventario,
                    &nuevo_snapshot.candidatas,
                );
                println!(
                    "[Worker] Recuperación local ({:?}): +{} obras (inventario={}/{}, quedan_candidatas_locales={}).",
                    grupo,
                    recuperadas,
                    nuevo_snapshot.inventario.get(grupo),
                    crate::db::OBJETIVO_INVENTARIO_POR_GRUPO,
                    quedan_candidatas_locales
                );
                return ResultadoCiclo::TrabajoLocalRealizado {
                    grupo,
                    recuperadas,
                    quedan_candidatas_locales,
                };
            }
            Ok(_) => {
                // La recuperación devolvió 0: reconsultar candidatas y marcar sin progreso en este ciclo
                snapshot = match leer_snapshot().await {
                    Ok(s) => s,
                    Err(e) => return ResultadoCiclo::ErrorSqlite(e),
                };
                planificador.marcar_sin_progreso(idx, grupo);
            }
        }
    }

    let grupos_pendientes_tmdb = planificador
        .grupos_con_deficit_sin_candidatas(&snapshot.inventario, &snapshot.candidatas);

    if !grupos_pendientes_tmdb.is_empty() {
        println!(
            "[Worker] Sin candidatas locales disponibles. Grupos con déficit pendientes de TMDB (D-3): {:?}",
            grupos_pendientes_tmdb
        );
        ResultadoCiclo::SinTrabajoLocalPendiente {
            grupos_pendientes_tmdb,
        }
    } else {
        ResultadoCiclo::SinTrabajoPendiente
    }
}

async fn run_discovery_worker_loop() {
    dotenvy::dotenv().ok();
    let cache_dir = crate::db::get_cache_posters_dir();
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("[Worker] Advertencia: no se pudo crear directorio de caché de pósters {:?}: {}", cache_dir, e);
    }
    let tmdb_api_key = std::env::var("TMDB_API_KEY")
        .unwrap_or_else(|_| "239957e2e77a70d1dd415586f3f551b0".to_string());
    let client = reqwest::Client::builder()
        .user_agent("QueVeoHoy/2.0 (https://github.com/alan/queveohoy)")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut planificador = PlanificadorGrupos::new();

    loop {
        let resultado = run_one_discovery_iteration(&mut planificador, &client, &tmdb_api_key).await;

        match calcular_ritmo_siguiente(&resultado) {
            RitmoEspera::ErrorBackoff {
                pausa_obligatoria_secs,
                espera_maxima_notify_secs,
            } => {
                if let ResultadoCiclo::ErrorSqlite(ref err) = resultado {
                    eprintln!(
                        "[Worker] Error SQLite en ciclo: {}. Backoff obligatorio ({}s).",
                        err, pausa_obligatoria_secs
                    );
                }
                tokio::time::sleep(std::time::Duration::from_secs(pausa_obligatoria_secs)).await;
                tokio::select! {
                    _ = get_discovery_notify().notified() => {
                        println!("[Worker] Despertado por señal legítima tras backoff...");
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(espera_maxima_notify_secs)) => {
                        println!("[Worker] Intervalo de mantenimiento periódico ({}s) tras backoff...", espera_maxima_notify_secs);
                    }
                }
            }
            RitmoEspera::TrabajoLocalContinuo { pausa_secs } => {
                tokio::time::sleep(std::time::Duration::from_secs(pausa_secs)).await;
            }
            RitmoEspera::Reposo {
                pausa_minima_secs,
                espera_maxima_notify_secs,
            } => {
                tokio::time::sleep(std::time::Duration::from_secs(pausa_minima_secs)).await;
                tokio::select! {
                    _ = get_discovery_notify().notified() => {
                        println!("[Worker] Despertado por señal legítima (trigger_discovery)...");
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_secs(espera_maxima_notify_secs)) => {
                        println!("[Worker] Intervalo de mantenimiento periódico ({}s)...", espera_maxima_notify_secs);
                    }
                }
            }
        }
    }
}

async fn run_one_discovery_iteration(
    planificador: &mut PlanificadorGrupos,
    client: &reqwest::Client,
    tmdb_api_key: &str,
) -> ResultadoCiclo {
    let now = crate::db::current_unix_timestamp();
    let cache_dir = crate::db::get_cache_posters_dir();
    ejecutar_ciclo_completo_con_posters(
        planificador,
        now,
        leer_snapshot_planificador_blocking,
        recuperar_local_blocking,
        reservar_peticion_tmdb_blocking,
        move |grupo| ejecutar_operacion_tmdb_real(client, tmdb_api_key, grupo),
        persistir_resultado_operacion_tmdb_blocking,
        move || {
            let cache_dir = cache_dir.clone();
            async move {
                procesar_cola_posters_con(
                    &cache_dir,
                    MAX_POSTERS_POR_CICLO,
                    leer_posters_pendientes_blocking,
                    |contenido_id, url| {
                        let cache_dir = cache_dir.clone();
                        async move {
                            cache_poster_image_en_dir(client, &cache_dir, contenido_id, &url).await
                        }
                    },
                    persistir_resultados_posters_blocking,
                )
                .await
            }
        },
    )
    .await
}

pub const MAX_PETICIONES_POR_OPERACION: usize = 8;
pub const COOLDOWN_EXITO_INSERCION_SECS: i64 = 120;
pub const COOLDOWN_SIN_APORTE_SECS: [i64; 4] = [300, 600, 1200, 1800];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstrategiaDescubrimiento {
    ConocidasBienValoradas,
    MenosPopularesDescubrimientos,
    DiversidadGeneros,
    ClasicosCulto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObraTmdbDescubierta {
    pub tmdb_id: i64,
    pub tipo: String,
    pub es_anime: bool,
    pub titulo: String,
    pub titulo_original: Option<String>,
    pub sinopsis: Option<String>,
    pub fecha_estreno: Option<String>,
    pub poster_path: Option<String>,
    pub generos: Option<String>,
    pub backdrop_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoOperacionTmdb {
    Completada {
        insertadas: usize,
        completada_sin_corte: bool,
    },
    CortadaPorPresupuesto {
        insertadas: usize,
    },
    Http429 {
        insertadas: usize,
        retry_after_secs: Option<i64>,
    },
    HttpError5xxORed {
        insertadas: usize,
        motivo: String,
        es_5xx: bool,
    },
    ErrorSqlite(String),
}

impl ResultadoOperacionTmdb {
    pub fn insertadas(&self) -> usize {
        match self {
            ResultadoOperacionTmdb::Completada { insertadas, .. }
            | ResultadoOperacionTmdb::CortadaPorPresupuesto { insertadas }
            | ResultadoOperacionTmdb::Http429 { insertadas, .. }
            | ResultadoOperacionTmdb::HttpError5xxORed { insertadas, .. } => *insertadas,
            ResultadoOperacionTmdb::ErrorSqlite(_) => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControladorPresupuestoOperacion {
    pub ops_requests_usadas: usize,
    pub max_por_operacion: usize,
}

impl ControladorPresupuestoOperacion {
    pub fn nuevo_tras_inicio() -> Self {
        Self {
            ops_requests_usadas: 1,
            max_por_operacion: MAX_PETICIONES_POR_OPERACION,
        }
    }

    pub fn puede_consumir_otra(&self) -> bool {
        self.ops_requests_usadas < self.max_por_operacion
    }

    pub async fn intentar_reservar_interna<F, Fut>(
        &mut self,
        endpoint: crate::models::TmdbEndpointKind,
        mut reservar_fn: F,
    ) -> Result<bool, String>
    where
        F: FnMut(crate::models::TmdbEndpointKind, bool) -> Fut,
        Fut: std::future::Future<
            Output = Result<crate::models::ReservaTmdbResultado, String>,
        >,
    {
        if !self.puede_consumir_otra() {
            return Ok(false);
        }
        match reservar_fn(endpoint, false).await? {
            crate::models::ReservaTmdbResultado::Reservada { .. } => {
                self.ops_requests_usadas += 1;
                Ok(true)
            }
            crate::models::ReservaTmdbResultado::PresupuestoAgotado { .. }
            | crate::models::ReservaTmdbResultado::BloqueoActivo { .. }
            | crate::models::ReservaTmdbResultado::IntervaloOperacionNoCumplido { .. } => {
                Ok(false)
            }
        }
    }
}

pub fn seleccionar_estrategia_descubrimiento(roll: u8) -> EstrategiaDescubrimiento {
    match roll % 100 {
        0..=39 => EstrategiaDescubrimiento::ConocidasBienValoradas,
        40..=69 => EstrategiaDescubrimiento::MenosPopularesDescubrimientos,
        70..=89 => EstrategiaDescubrimiento::DiversidadGeneros,
        _ => EstrategiaDescubrimiento::ClasicosCulto,
    }
}

pub fn normalizar_poster_path(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Evitar duplicación de prefijo TMDB si viniera malformado
    const TMDB_PREFIX_W500: &str = "https://image.tmdb.org/t/p/w500";
    let mut cleaned = trimmed;
    while let Some(rest) = cleaned.strip_prefix(TMDB_PREFIX_W500) {
        if rest.starts_with("http://") || rest.starts_with("https://") {
            cleaned = rest;
        } else {
            break;
        }
    }

    if cleaned.starts_with("http://") || cleaned.starts_with("https://") || cleaned.starts_with('/') {
        Some(cleaned.to_string())
    } else {
        Some(format!("/{}", cleaned))
    }
}

pub fn normalizar_backdrop_url(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Some(trimmed.to_string())
    } else if trimmed.starts_with('/') {
        Some(format!("https://image.tmdb.org/t/p/w1280{}", trimmed))
    } else {
        Some(format!("https://image.tmdb.org/t/p/w1280/{}", trimmed))
    }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn parse_month_abbrev(s: &str) -> Option<i64> {
    match s.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn parse_tz_offset_secs(tz: &str) -> Option<i64> {
    let upper = tz.trim().to_ascii_uppercase();
    if upper == "GMT" || upper == "UTC" || upper == "UT" || upper == "Z" {
        return Some(0);
    }
    if upper.len() == 5 && (upper.starts_with('+') || upper.starts_with('-')) {
        let sign = if upper.starts_with('+') { 1 } else { -1 };
        let hh: i64 = upper[1..3].parse().ok()?;
        let mm: i64 = upper[3..5].parse().ok()?;
        if hh <= 23 && mm <= 59 {
            return Some(sign * (hh * 3600 + mm * 60));
        }
    }
    None
}

/// Parsea el header HTTP `Retry-After` sin dependencias externas:
/// - Segundos enteros no negativos.
/// - Fecha HTTP IMF-fixdate / RFC 2822 (ej. `Wed, 21 Oct 2015 07:28:00 GMT` o `+0000`).
///
/// Devuelve `Some(segundos_restantes)` (0 si la fecha ya venció) o `None` si el formato es inválido.
pub fn parse_retry_after(header_val: &str, now_unix: i64) -> Option<i64> {
    let s = header_val.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(secs) = s.parse::<i64>() {
        if secs >= 0 {
            return Some(secs);
        }
        return None;
    }

    // Quitar prefijo opcional de día de semana ("Wed, ")
    let after_comma = if let Some((_, rest)) = s.split_once(',') {
        rest.trim()
    } else {
        s
    };

    let parts: Vec<&str> = after_comma.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    let day: i64 = parts[0].parse().ok()?;
    let month = parse_month_abbrev(parts[1])?;
    let year: i64 = parts[2].parse().ok()?;
    if !(1970..=9999).contains(&year) || day < 1 || day > days_in_month(year, month) {
        return None;
    }

    let time_parts: Vec<&str> = parts[3].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;
    let second: i64 = time_parts[2].parse().ok()?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return None;
    }

    let tz_offset = parse_tz_offset_secs(parts[4])?;
    let days = days_from_civil(year, month, day);
    let target_unix = days * 86400 + hour * 3600 + minute * 60 + second - tz_offset;
    Some((target_unix - now_unix).max(0))
}

pub fn calcular_cooldown_sin_aporte_secs(fallos_sin_aporte: i64) -> i64 {
    let idx = (fallos_sin_aporte.saturating_sub(1)).clamp(0, 3) as usize;
    COOLDOWN_SIN_APORTE_SECS[idx]
}

pub fn calcular_backoff_error_http_secs(errores_consecutivos: i64) -> i64 {
    let exp = (errores_consecutivos.saturating_sub(1)).clamp(0, 4) as u32;
    60 * 2_i64.pow(exp)
}

pub fn puede_iniciar_tmdb_en_snapshot(
    snapshot: &SnapshotPlanificador,
    grupo: crate::models::GrupoContenido,
    now: i64,
) -> bool {
    // A. Déficit real
    if snapshot.inventario.get(grupo) >= crate::db::OBJETIVO_INVENTARIO_POR_GRUPO {
        return false;
    }
    // B. Recuperación local agotada
    if snapshot.candidatas.get(grupo) > 0 {
        return false;
    }
    // C. Sin bloqueo global
    if let Some(hasta) = snapshot.worker_state.tmdb_bloqueado_hasta {
        if hasta > now {
            return false;
        }
    }
    // D. Sin cooldown del grupo
    if let Some(cd) = snapshot.cooldowns.iter().find(|c| c.grupo == grupo) {
        if cd.bloqueado_hasta > now {
            return false;
        }
    }
    true
}

async fn reservar_peticion_tmdb_blocking(
    endpoint_kind: crate::models::TmdbEndpointKind,
    es_inicio_operacion: bool,
) -> Result<crate::models::ReservaTmdbResultado, String> {
    tokio::task::spawn_blocking(move || -> Result<crate::models::ReservaTmdbResultado, String> {
        crate::db::reservar_peticion_tmdb(endpoint_kind.as_str(), es_inicio_operacion)
            .map_err(|e| format!("Error SQLite en reserva TMDB: {}", e))
    })
    .await
    .map_err(|e| format!("Error JoinError en spawn_blocking (reserva_tmdb): {}", e))?
}

async fn persistir_resultado_operacion_tmdb_blocking(
    grupo: crate::models::GrupoContenido,
    resultado: ResultadoOperacionTmdb,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = crate::db::get_connection()
            .map_err(|e| format!("Error abriendo conexión SQLite (persistir TMDB): {}", e))?;
        let now = crate::db::current_unix_timestamp();
        persistir_resultado_operacion_tmdb_conn(&mut conn, grupo, &resultado, now)
            .map_err(|e| format!("Error SQLite persistiendo resultado TMDB: {}", e))
    })
    .await
    .map_err(|e| format!("Error JoinError en spawn_blocking (persistir TMDB): {}", e))?
}

pub fn persistir_resultado_operacion_tmdb_conn(
    conn: &mut rusqlite::Connection,
    grupo: crate::models::GrupoContenido,
    resultado: &ResultadoOperacionTmdb,
    now: i64,
) -> rusqlite::Result<()> {
    let cooldown_actual = crate::db::get_cooldown_grupo_conn(conn, grupo)?;
    let fallos_previos = cooldown_actual.map(|c| c.fallos_sin_aporte).unwrap_or(0);

    match resultado {
        ResultadoOperacionTmdb::Completada {
            insertadas,
            completada_sin_corte,
        } => {
            if *insertadas > 0 {
                crate::db::restablecer_errores_tmdb_conn(conn)?;
                crate::db::registrar_cooldown_grupo_conn(
                    conn,
                    grupo,
                    now + COOLDOWN_EXITO_INSERCION_SECS,
                    0,
                    Some("Operación TMDB con inserciones"),
                )?;
            } else if *completada_sin_corte {
                crate::db::restablecer_errores_tmdb_conn(conn)?;
                let nuevos_fallos = fallos_previos + 1;
                let cd_secs = calcular_cooldown_sin_aporte_secs(nuevos_fallos);
                crate::db::registrar_cooldown_grupo_conn(
                    conn,
                    grupo,
                    now + cd_secs,
                    nuevos_fallos,
                    Some("Operación TMDB válida sin aportes"),
                )?;
            }
        }
        ResultadoOperacionTmdb::CortadaPorPresupuesto { insertadas } => {
            if *insertadas > 0 {
                crate::db::restablecer_errores_tmdb_conn(conn)?;
                crate::db::registrar_cooldown_grupo_conn(
                    conn,
                    grupo,
                    now + COOLDOWN_EXITO_INSERCION_SECS,
                    0,
                    Some("Operación TMDB parcial con inserciones"),
                )?;
            }
            // Si insertadas == 0 y se cortó por presupuesto antes de completar, no incrementar fallos_sin_aporte
        }
        ResultadoOperacionTmdb::Http429 {
            insertadas,
            retry_after_secs,
        } => {
            let err_count = crate::db::registrar_error_tmdb_conn(conn, "TMDB HTTP 429")?;
            let wait_secs = match retry_after_secs {
                Some(s) if *s > 0 => *s,
                _ => calcular_backoff_error_http_secs(err_count),
            };
            crate::db::registrar_bloqueo_tmdb_conn(conn, now + wait_secs, "HTTP 429 Too Many Requests")?;
            let nuevos_fallos = if *insertadas > 0 { 0 } else { fallos_previos + 1 };
            let cd_secs = calcular_cooldown_sin_aporte_secs(nuevos_fallos.max(1));
            crate::db::registrar_cooldown_grupo_conn(
                conn,
                grupo,
                now + cd_secs,
                nuevos_fallos,
                Some("HTTP 429"),
            )?;
        }
        ResultadoOperacionTmdb::HttpError5xxORed {
            insertadas,
            motivo,
            es_5xx,
        } => {
            let err_count = crate::db::registrar_error_tmdb_conn(conn, motivo)?;
            if *es_5xx || err_count >= 3 {
                let wait_secs = calcular_backoff_error_http_secs(err_count);
                crate::db::registrar_bloqueo_tmdb_conn(conn, now + wait_secs, motivo)?;
            }
            let nuevos_fallos = if *insertadas > 0 { 0 } else { fallos_previos + 1 };
            let cd_secs = calcular_cooldown_sin_aporte_secs(nuevos_fallos.max(1));
            crate::db::registrar_cooldown_grupo_conn(
                conn,
                grupo,
                now + cd_secs,
                nuevos_fallos,
                Some(motivo.as_str()),
            )?;
        }
        ResultadoOperacionTmdb::ErrorSqlite(_) => {}
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn ejecutar_ciclo_completo_con<
    FSnap,
    FSnapFut,
    FRec,
    FRecFut,
    FRes,
    FResFut,
    FTmdb,
    FTmdbFut,
    FPers,
    FPersFut,
>(
    planificador: &mut PlanificadorGrupos,
    now: i64,
    leer_snapshot: FSnap,
    recuperar_local: FRec,
    reservar_tmdb: FRes,
    ejecutar_tmdb: FTmdb,
    persistir_tmdb: FPers,
) -> ResultadoCiclo
where
    FSnap: FnMut() -> FSnapFut,
    FSnapFut: std::future::Future<Output = Result<SnapshotPlanificador, String>>,
    FRec: FnMut(crate::models::GrupoContenido, usize) -> FRecFut,
    FRecFut: std::future::Future<Output = Result<usize, String>>,
    FRes: FnMut(crate::models::TmdbEndpointKind, bool) -> FResFut,
    FResFut: std::future::Future<Output = Result<crate::models::ReservaTmdbResultado, String>>,
    FTmdb: FnMut(crate::models::GrupoContenido) -> FTmdbFut,
    FTmdbFut: std::future::Future<Output = ResultadoOperacionTmdb>,
    FPers: FnMut(crate::models::GrupoContenido, ResultadoOperacionTmdb) -> FPersFut,
    FPersFut: std::future::Future<Output = Result<(), String>>,
{
    ejecutar_ciclo_completo_con_posters(
        planificador,
        now,
        leer_snapshot,
        recuperar_local,
        reservar_tmdb,
        ejecutar_tmdb,
        persistir_tmdb,
        || async { Ok(0usize) },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn ejecutar_ciclo_completo_con_posters<
    FSnap,
    FSnapFut,
    FRec,
    FRecFut,
    FRes,
    FResFut,
    FTmdb,
    FTmdbFut,
    FPers,
    FPersFut,
    FPost,
    FPostFut,
>(
    planificador: &mut PlanificadorGrupos,
    now: i64,
    mut leer_snapshot: FSnap,
    mut recuperar_local: FRec,
    mut reservar_tmdb: FRes,
    mut ejecutar_tmdb: FTmdb,
    mut persistir_tmdb: FPers,
    mut mantener_posters: FPost,
) -> ResultadoCiclo
where
    FSnap: FnMut() -> FSnapFut,
    FSnapFut: std::future::Future<Output = Result<SnapshotPlanificador, String>>,
    FRec: FnMut(crate::models::GrupoContenido, usize) -> FRecFut,
    FRecFut: std::future::Future<Output = Result<usize, String>>,
    FRes: FnMut(crate::models::TmdbEndpointKind, bool) -> FResFut,
    FResFut: std::future::Future<Output = Result<crate::models::ReservaTmdbResultado, String>>,
    FTmdb: FnMut(crate::models::GrupoContenido) -> FTmdbFut,
    FTmdbFut: std::future::Future<Output = ResultadoOperacionTmdb>,
    FPers: FnMut(crate::models::GrupoContenido, ResultadoOperacionTmdb) -> FPersFut,
    FPersFut: std::future::Future<Output = Result<(), String>>,
    FPost: FnMut() -> FPostFut,
    FPostFut: std::future::Future<Output = Result<usize, String>>,
{
    planificador.iniciar_ciclo();

    let mut snapshot = match leer_snapshot().await {
        Ok(s) => s,
        Err(e) => return ResultadoCiclo::ErrorSqlite(e),
    };

    // 1. Prioridad absoluta: recuperación local
    while let Some((idx, grupo, lote)) =
        planificador.seleccionar_siguiente_grupo(&snapshot.inventario, &snapshot.candidatas)
    {
        match recuperar_local(grupo, lote).await {
            Err(e) => return ResultadoCiclo::ErrorSqlite(e),
            Ok(recuperadas) if recuperadas > 0 => {
                planificador.marcar_grupo_atendido(idx, grupo);
                let nuevo_snapshot = match leer_snapshot().await {
                    Ok(s) => s,
                    Err(e) => return ResultadoCiclo::ErrorSqlite(e),
                };
                let quedan_candidatas_locales = planificador.hay_trabajo_local_disponible(
                    &nuevo_snapshot.inventario,
                    &nuevo_snapshot.candidatas,
                );
                return ResultadoCiclo::TrabajoLocalRealizado {
                    grupo,
                    recuperadas,
                    quedan_candidatas_locales,
                };
            }
            Ok(_) => {
                snapshot = match leer_snapshot().await {
                    Ok(s) => s,
                    Err(e) => return ResultadoCiclo::ErrorSqlite(e),
                };
                planificador.marcar_sin_progreso(idx, grupo);
            }
        }
    }

    // 2. Fase externa TMDB condicionada (AND estricto A..E)
    let grupos = crate::models::GrupoContenido::all();
    let inicio = planificador.cursor_seleccion % grupos.len();

    for offset in 0..grupos.len() {
        let idx = (inicio + offset) % grupos.len();
        let grupo = grupos[idx];

        if !puede_iniciar_tmdb_en_snapshot(&snapshot, grupo, now) {
            planificador.marcar_sin_progreso(idx, grupo);
            continue;
        }

        // Condición E: Reserva atómica inicial (es_inicio_operacion = true)
        match reservar_tmdb(crate::models::TmdbEndpointKind::Discover, true).await {
            Err(e) => return ResultadoCiclo::ErrorSqlite(e),
            Ok(crate::models::ReservaTmdbResultado::Reservada { .. }) => {
                let op_res = ejecutar_tmdb(grupo).await;
                if let ResultadoOperacionTmdb::ErrorSqlite(err) = op_res {
                    return ResultadoCiclo::ErrorSqlite(err);
                }
                let insertadas = op_res.insertadas();
                if let Err(e) = persistir_tmdb(grupo, op_res).await {
                    return ResultadoCiclo::ErrorSqlite(e);
                }
                planificador.marcar_grupo_atendido(idx, grupo);
                let nuevo_snapshot = match leer_snapshot().await {
                    Ok(s) => s,
                    Err(e) => return ResultadoCiclo::ErrorSqlite(e),
                };
                let quedan_candidatas_locales = planificador.hay_trabajo_local_disponible(
                    &nuevo_snapshot.inventario,
                    &nuevo_snapshot.candidatas,
                );
                if let Err(err) = mantener_posters().await {
                    eprintln!(
                        "[Worker] Advertencia: error SQLite en mantenimiento de pósteres tras operación TMDB: {}",
                        err
                    );
                }
                return ResultadoCiclo::TrabajoExternoRealizado {
                    grupo,
                    insertadas,
                    quedan_candidatas_locales,
                };
            }
            Ok(crate::models::ReservaTmdbResultado::BloqueoActivo { .. })
            | Ok(crate::models::ReservaTmdbResultado::PresupuestoAgotado { .. })
            | Ok(crate::models::ReservaTmdbResultado::IntervaloOperacionNoCumplido { .. }) => {
                planificador.marcar_sin_progreso(idx, grupo);
                break;
            }
        }
    }

    match mantener_posters().await {
        Err(e) => return ResultadoCiclo::ErrorSqlite(e),
        Ok(posters_procesados) if posters_procesados > 0 => {
            return ResultadoCiclo::SoloMantenimientoPosters { posters_procesados };
        }
        Ok(_) => {}
    }

    let grupos_pendientes_tmdb = planificador
        .grupos_con_deficit_sin_candidatas(&snapshot.inventario, &snapshot.candidatas);
    if !grupos_pendientes_tmdb.is_empty() {
        ResultadoCiclo::SinTrabajoLocalPendiente {
            grupos_pendientes_tmdb,
        }
    } else {
        ResultadoCiclo::SinTrabajoPendiente
    }
}

pub fn construir_url_discover_grupo(
    api_key: &str,
    grupo: crate::models::GrupoContenido,
    estrategia: EstrategiaDescubrimiento,
    year: i32,
    genero_id: i64,
) -> String {
    let (media_type, es_anime) = match grupo {
        crate::models::GrupoContenido::PeliculasNoAnime => ("movie", false),
        crate::models::GrupoContenido::SeriesNoAnime => ("tv", false),
        crate::models::GrupoContenido::PeliculasAnime => ("movie", true),
        crate::models::GrupoContenido::SeriesAnime => ("tv", true),
    };
    let year_param = if media_type == "tv" {
        "first_air_date_year"
    } else {
        "primary_release_year"
    };

    let mut url = format!(
        "https://api.themoviedb.org/3/discover/{}?api_key={}&language=es-ES&include_adult=false",
        media_type, api_key
    );

    if es_anime {
        url.push_str("&with_genres=16&with_origin_country=JP");
    }

    match estrategia {
        EstrategiaDescubrimiento::ConocidasBienValoradas => {
            url.push_str(&format!(
                "&{}={}&sort_by=vote_average.desc&vote_count.gte=200",
                year_param, year
            ));
        }
        EstrategiaDescubrimiento::MenosPopularesDescubrimientos => {
            url.push_str(&format!(
                "&{}={}&sort_by=popularity.asc&vote_count.gte=20",
                year_param, year
            ));
        }
        EstrategiaDescubrimiento::DiversidadGeneros => {
            if es_anime {
                url.push_str(&format!("&{}={}&with_genres=16,{}", year_param, year, genero_id));
            } else {
                url.push_str(&format!("&{}={}&with_genres={}", year_param, year, genero_id));
            }
        }
        EstrategiaDescubrimiento::ClasicosCulto => {
            let classic_year = 1950 + (year.abs() % 50);
            url.push_str(&format!(
                "&{}={}&sort_by=vote_average.desc&vote_count.gte=100",
                year_param, classic_year
            ));
        }
    }

    url
}

fn extraer_generos_json(item: &Value) -> Option<String> {
    let gids = item["genre_ids"].as_array()?;
    let nombres: Vec<&str> = gids
        .iter()
        .filter_map(|g| g.as_i64())
        .filter_map(|id| {
            TMDB_GENRE_MAP
                .iter()
                .find(|(gid, _)| *gid == id)
                .map(|(_, name)| *name)
        })
        .take(4)
        .collect();
    if nombres.is_empty() {
        None
    } else {
        Some(format!(
            "[{}]",
            nombres
                .iter()
                .map(|n| format!("\"{}\"", n))
                .collect::<Vec<_>>()
                .join(",")
        ))
    }
}

fn parsear_obra_tmdb(
    item: &Value,
    tipo: &str,
    es_anime: bool,
) -> Option<ObraTmdbDescubierta> {
    let tmdb_id = item["id"].as_i64().unwrap_or(0);
    if tmdb_id == 0 {
        return None;
    }

    let titulo = item["title"]
        .as_str()
        .or(item["name"].as_str())
        .unwrap_or("Sin Título")
        .trim()
        .to_string();
    let titulo_original = item["original_title"]
        .as_str()
        .or(item["original_name"].as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let sinopsis = item["overview"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let fecha_estreno = item["release_date"]
        .as_str()
        .or(item["first_air_date"].as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let poster_path = normalizar_poster_path(item["poster_path"].as_str());
    let backdrop_url = normalizar_backdrop_url(item["backdrop_path"].as_str());
    let generos = extraer_generos_json(item);

    Some(ObraTmdbDescubierta {
        tmdb_id,
        tipo: tipo.to_string(),
        es_anime,
        titulo,
        titulo_original,
        sinopsis,
        fecha_estreno,
        poster_path,
        generos,
        backdrop_url,
    })
}

async fn ejecutar_operacion_tmdb_real(
    client: &reqwest::Client,
    api_key: &str,
    grupo: crate::models::GrupoContenido,
) -> ResultadoOperacionTmdb {
    match grupo {
        crate::models::GrupoContenido::PeliculasNoAnime
        | crate::models::GrupoContenido::SeriesNoAnime => {
            fetch_tmdb_discover(client, api_key, grupo).await
        }
        crate::models::GrupoContenido::PeliculasAnime
        | crate::models::GrupoContenido::SeriesAnime => {
            fetch_tmdb_anime(client, api_key, grupo).await
        }
    }
}

async fn fetch_tmdb_discover(
    client: &reqwest::Client,
    api_key: &str,
    grupo: crate::models::GrupoContenido,
) -> ResultadoOperacionTmdb {
    ejecutar_discover_grupo(client, api_key, grupo).await
}

async fn fetch_tmdb_anime(
    client: &reqwest::Client,
    api_key: &str,
    grupo: crate::models::GrupoContenido,
) -> ResultadoOperacionTmdb {
    ejecutar_discover_grupo(client, api_key, grupo).await
}

async fn ejecutar_discover_grupo(
    client: &reqwest::Client,
    api_key: &str,
    grupo: crate::models::GrupoContenido,
) -> ResultadoOperacionTmdb {
    let (year, roll, genero_idx) = {
        let mut rng = rand::thread_rng();
        (
            rng.gen_range(1990..=2026),
            rng.gen_range(0..100u8),
            rng.gen_range(0..TMDB_GENRE_MAP.len()),
        )
    };
    let estrategia = seleccionar_estrategia_descubrimiento(roll);
    let genero_id = TMDB_GENRE_MAP[genero_idx].0;
    let init_url = construir_url_discover_grupo(api_key, grupo, estrategia, year, genero_id);

    let mut presupuesto = ControladorPresupuestoOperacion::nuevo_tras_inicio();
    let mut insertadas: usize = 0;

    let resp = match client.get(&init_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return ResultadoOperacionTmdb::HttpError5xxORed {
                insertadas,
                motivo: format!("Error de red TMDB discover: {}", e),
                es_5xx: false,
            };
        }
    };

    let status = resp.status();
    if status.as_u16() == 429 {
        let now = crate::db::current_unix_timestamp();
        let retry_after_secs = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| parse_retry_after(s, now));
        return ResultadoOperacionTmdb::Http429 {
            insertadas,
            retry_after_secs,
        };
    }
    if status.is_server_error() {
        return ResultadoOperacionTmdb::HttpError5xxORed {
            insertadas,
            motivo: format!("TMDB discover HTTP {}", status),
            es_5xx: true,
        };
    }
    if !status.is_success() {
        return ResultadoOperacionTmdb::HttpError5xxORed {
            insertadas,
            motivo: format!("TMDB discover HTTP {}", status),
            es_5xx: false,
        };
    }

    let init_json: Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            return ResultadoOperacionTmdb::HttpError5xxORed {
                insertadas,
                motivo: format!("Error JSON TMDB discover: {}", e),
                es_5xx: false,
            };
        }
    };

    let total_pages = init_json["total_pages"].as_i64().unwrap_or(1).min(20) as i32;
    if total_pages < 1 {
        return ResultadoOperacionTmdb::Completada {
            insertadas: 0,
            completada_sin_corte: true,
        };
    }

    let random_page = rand::thread_rng().gen_range(1..=total_pages);
    let page_json = if random_page == 1 {
        init_json
    } else {
        match presupuesto
            .intentar_reservar_interna(
                crate::models::TmdbEndpointKind::Discover,
                reservar_peticion_tmdb_blocking,
            )
            .await
        {
            Err(e) => return ResultadoOperacionTmdb::ErrorSqlite(e),
            Ok(false) => {
                return ResultadoOperacionTmdb::CortadaPorPresupuesto { insertadas };
            }
            Ok(true) => {}
        }

        let page_url = format!("{}&page={}", init_url, random_page);
        let p_resp = match client.get(&page_url).send().await {
            Ok(r) => r,
            Err(e) => {
                return ResultadoOperacionTmdb::HttpError5xxORed {
                    insertadas,
                    motivo: format!("Error de red TMDB page: {}", e),
                    es_5xx: false,
                };
            }
        };
        let p_status = p_resp.status();
        if p_status.as_u16() == 429 {
            let now = crate::db::current_unix_timestamp();
            let retry_after_secs = p_resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| parse_retry_after(s, now));
            return ResultadoOperacionTmdb::Http429 {
                insertadas,
                retry_after_secs,
            };
        }
        if p_status.is_server_error() {
            return ResultadoOperacionTmdb::HttpError5xxORed {
                insertadas,
                motivo: format!("TMDB page HTTP {}", p_status),
                es_5xx: true,
            };
        }
        if !p_status.is_success() {
            return ResultadoOperacionTmdb::HttpError5xxORed {
                insertadas,
                motivo: format!("TMDB page HTTP {}", p_status),
                es_5xx: false,
            };
        }
        match p_resp.json::<Value>().await {
            Ok(j) => j,
            Err(e) => {
                return ResultadoOperacionTmdb::HttpError5xxORed {
                    insertadas,
                    motivo: format!("Error JSON TMDB page: {}", e),
                    es_5xx: false,
                };
            }
        }
    };

    let (tipo, es_anime) = grupo.db_filter();
    let es_anime_bool = es_anime == 1;

    if let Some(results) = page_json["results"].as_array() {
        let mut sample = results.clone();
        let to_take = {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            sample.shuffle(&mut rng);
            rng.gen_range(1..=3).min(sample.len())
        };

        for item in sample.into_iter().take(to_take) {
            let Some(obra) = parsear_obra_tmdb(&item, tipo, es_anime_bool) else {
                continue;
            };

            // Validación de estado para series no anime (/3/tv/{id} dentro del presupuesto)
            if tipo == "TV" && !es_anime_bool {
                match presupuesto
                    .intentar_reservar_interna(
                        crate::models::TmdbEndpointKind::TvDetail,
                        reservar_peticion_tmdb_blocking,
                    )
                    .await
                {
                    Err(e) => return ResultadoOperacionTmdb::ErrorSqlite(e),
                    Ok(false) => {
                        return ResultadoOperacionTmdb::CortadaPorPresupuesto { insertadas };
                    }
                    Ok(true) => {}
                }

                let detail_url = format!(
                    "https://api.themoviedb.org/3/tv/{}?api_key={}&language=es-ES",
                    obra.tmdb_id, api_key
                );
                let det_resp = match client.get(&detail_url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        return ResultadoOperacionTmdb::HttpError5xxORed {
                            insertadas,
                            motivo: format!("Error de red TMDB tv detail: {}", e),
                            es_5xx: false,
                        };
                    }
                };
                let det_status = det_resp.status();
                if det_status.as_u16() == 429 {
                    let now = crate::db::current_unix_timestamp();
                    let retry_after_secs = det_resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| parse_retry_after(s, now));
                    return ResultadoOperacionTmdb::Http429 {
                        insertadas,
                        retry_after_secs,
                    };
                }
                if det_status.is_server_error() {
                    return ResultadoOperacionTmdb::HttpError5xxORed {
                        insertadas,
                        motivo: format!("TMDB tv detail HTTP {}", det_status),
                        es_5xx: true,
                    };
                }
                if !det_status.is_success() {
                    continue;
                }
                if let Ok(det_json) = det_resp.json::<Value>().await {
                    let st = det_json["status"].as_str().unwrap_or("");
                    if st != "Ended" && st != "Canceled" && st != "Cancelled" && st != "Miniseries"
                    {
                        continue;
                    }
                }
            }

            match insert_to_catalog_blocking(obra).await {
                Ok(true) => insertadas += 1,
                Ok(false) => {}
                Err(e) => return ResultadoOperacionTmdb::ErrorSqlite(e),
            }
        }
    }

    ResultadoOperacionTmdb::Completada {
        insertadas,
        completada_sin_corte: true,
    }
}

#[allow(dead_code)]
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
        let resp_result = client.get(&url).timeout(std::time::Duration::from_secs(20)).send().await;
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

            if insert_to_catalog(client, None, Some(mal_id), tipo, true, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos, None).await {
                inserted_count += 1;
            }
        }
    }
    Ok(inserted_count)
}

pub fn insert_to_catalog_conn(
    conn: &rusqlite::Connection,
    obra: &ObraTmdbDescubierta,
) -> rusqlite::Result<bool> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM contenido WHERE tmdb_id = ?1 AND tmdb_id IS NOT NULL",
            params![obra.tmdb_id],
            |row| row.get(0),
        )
        .ok();

    let contenido_id = if let Some(id) = exists {
        if let Some(ref gen) = obra.generos {
            conn.execute(
                "UPDATE contenido SET generos = ?1 WHERE id = ?2 AND (generos IS NULL OR generos = '')",
                params![gen, id],
            )?;
        }
        if let Some(ref b_url) = obra.backdrop_url {
            conn.execute(
                "UPDATE contenido SET backdrop_url = ?1 WHERE id = ?2 AND (backdrop_url IS NULL OR backdrop_url = '')",
                params![b_url, id],
            )?;
        }
        id
    } else {
        conn.execute(
            "INSERT INTO contenido (tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original, sinopsis, fecha_estreno, poster_url, generos, backdrop_url)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                obra.tmdb_id,
                obra.tipo,
                if obra.es_anime { 1 } else { 0 },
                obra.titulo,
                obra.titulo_original,
                obra.sinopsis,
                obra.fecha_estreno,
                obra.poster_path,
                obra.generos,
                obra.backdrop_url,
            ],
        )?;
        conn.last_insert_rowid()
    };

    // Etapa 5.9.2A: Blindaje de inserción — no ingresar al catálogo offline
    // obras que ya pertenecen a la biblioteca del usuario (usuario_contenido).
    let rows_affected = conn.execute(
        "INSERT OR IGNORE INTO catalogo_offline (contenido_id, fecha_agregado)
         SELECT ?1, datetime('now', 'localtime')
         WHERE NOT EXISTS (
             SELECT 1 FROM usuario_contenido WHERE contenido_id = ?1
         )",
        params![contenido_id],
    )?;

    Ok(rows_affected > 0)
}

pub async fn insert_to_catalog_blocking(obra: ObraTmdbDescubierta) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || -> Result<bool, String> {
        let conn = get_connection()
            .map_err(|e| format!("Error abriendo conexión SQLite (insert_to_catalog): {}", e))?;
        insert_to_catalog_conn(&conn, &obra)
            .map_err(|e| format!("Error SQLite en insert_to_catalog: {}", e))
    })
    .await
    .map_err(|e| format!("Error JoinError en spawn_blocking (insert_to_catalog): {}", e))?
}

#[allow(dead_code, clippy::too_many_arguments)]
async fn insert_to_catalog(
    _client: &reqwest::Client,
    tmdb_id: Option<i64>,
    _mal_id: Option<i64>,
    tipo: &str,
    es_anime: bool,
    titulo: String,
    titulo_original: Option<String>,
    sinopsis: Option<String>,
    fecha_estreno: Option<String>,
    poster_url: Option<String>,
    generos: Option<String>,
    backdrop_url: Option<String>,
) -> bool {
    let Some(tid) = tmdb_id else {
        return false;
    };
    let obra = ObraTmdbDescubierta {
        tmdb_id: tid,
        tipo: tipo.to_string(),
        es_anime,
        titulo,
        titulo_original,
        sinopsis,
        fecha_estreno,
        poster_path: normalizar_poster_path(poster_url.as_deref()),
        generos,
        backdrop_url,
    };
    insert_to_catalog_blocking(obra).await.unwrap_or(false)
}

pub const MAX_POSTERS_POR_CICLO: usize = 5;

pub fn resolver_url_descarga_poster(raw: &str) -> Option<String> {
    if !crate::db::es_url_poster_valida(raw) {
        return None;
    }
    crate::db::normalizar_poster_url_salida(Some(raw.to_string()))
}

async fn leer_posters_pendientes_blocking(
    limite: usize,
) -> Result<Vec<crate::models::PosterRetryState>, String> {
    tokio::task::spawn_blocking(move || {
        crate::db::get_posters_pendientes_mantenimiento(limite)
            .map_err(|e| format!("Error SQLite en get_posters_pendientes_mantenimiento: {}", e))
    })
    .await
    .map_err(|e| {
        format!(
            "Error JoinError en spawn_blocking (get_posters_pendientes_mantenimiento): {}",
            e
        )
    })?
}

async fn persistir_resultados_posters_blocking(
    resultados: Vec<crate::db::ResultadoDescargaPoster>,
) -> Result<(), String> {
    if resultados.is_empty() {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || {
        crate::db::persistir_resultados_posters(&resultados)
            .map_err(|e| format!("Error SQLite en persistir_resultados_posters: {}", e))
    })
    .await
    .map_err(|e| {
        format!(
            "Error JoinError en spawn_blocking (persistir_resultados_posters): {}",
            e
        )
    })?
}

pub async fn procesar_cola_posters_con<FRead, FReadFut, FDown, FDownFut, FSave, FSaveFut>(
    cache_dir: &std::path::Path,
    limite: usize,
    mut leer_pendientes: FRead,
    mut descargar_poster: FDown,
    mut persistir_resultados: FSave,
) -> Result<usize, String>
where
    FRead: FnMut(usize) -> FReadFut,
    FReadFut: std::future::Future<Output = Result<Vec<crate::models::PosterRetryState>, String>>,
    FDown: FnMut(i64, String) -> FDownFut,
    FDownFut: std::future::Future<Output = Result<(), String>>,
    FSave: FnMut(Vec<crate::db::ResultadoDescargaPoster>) -> FSaveFut,
    FSaveFut: std::future::Future<Output = Result<(), String>>,
{
    if limite == 0 {
        return Ok(0);
    }

    let pendientes = leer_pendientes(limite).await?;
    if pendientes.is_empty() {
        return Ok(0);
    }

    let mut vistos = std::collections::HashSet::new();
    let mut resultados = Vec::new();
    let mut procesados = 0usize;

    for item in pendientes {
        if procesados >= limite {
            break;
        }
        if !vistos.insert(item.contenido_id) {
            continue;
        }

        let poster_path = cache_dir.join(format!("{}.jpg", item.contenido_id));
        if poster_path.exists() {
            resultados.push(crate::db::ResultadoDescargaPoster::Exito {
                contenido_id: item.contenido_id,
            });
            procesados += 1;
            continue;
        }

        let Some(url_descarga) = resolver_url_descarga_poster(&item.url_fallida) else {
            continue;
        };

        procesados += 1;
        match descargar_poster(item.contenido_id, url_descarga).await {
            Ok(()) => {
                resultados.push(crate::db::ResultadoDescargaPoster::Exito {
                    contenido_id: item.contenido_id,
                });
            }
            Err(error) => {
                resultados.push(crate::db::ResultadoDescargaPoster::Fallo {
                    contenido_id: item.contenido_id,
                    url: item.url_fallida,
                    error,
                });
            }
        }
    }

    if !resultados.is_empty() {
        persistir_resultados(resultados).await?;
    }

    Ok(procesados)
}

pub async fn cache_poster_image_en_dir(
    client: &reqwest::Client,
    cache_dir: &std::path::Path,
    contenido_id: i64,
    url: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("No se pudo crear directorio de caché {:?}: {}", cache_dir, e))?;

    let poster_path = cache_dir.join(format!("{}.jpg", contenido_id));
    if poster_path.exists() {
        return Ok(());
    }

    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Fallo de red al descargar póster {}: {}", contenido_id, e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "HTTP status {} al descargar póster {}",
            resp.status(),
            contenido_id
        ));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Error leyendo bytes del póster {}: {}", contenido_id, e))?;

    let tmp_path = cache_dir.join(format!("{}.jpg.tmp", contenido_id));
    std::fs::write(&tmp_path, &bytes).map_err(|e| {
        format!(
            "Fallo al escribir archivo temporal de póster {:?}: {}",
            tmp_path, e
        )
    })?;

    if let Err(e) = std::fs::rename(&tmp_path, &poster_path) {
        let _ = std::fs::remove_file(&tmp_path);
        std::fs::write(&poster_path, &bytes).map_err(|write_err| {
            format!(
                "Fallo en rename ({}) y escritura directa de póster {:?}: {}",
                e, poster_path, write_err
            )
        })?;
    }

    Ok(())
}

pub async fn cache_poster_image(client: &reqwest::Client, contenido_id: i64, url: &str) {
    let cache_dir = crate::db::get_cache_posters_dir();
    if let Err(e) = cache_poster_image_en_dir(client, &cache_dir, contenido_id, url).await {
        eprintln!("[Worker] Advertencia ({}): {}", url, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConteoPorGrupo, GrupoContenido, TmdbEndpointKind, TmdbWorkerState};
    use std::sync::{Arc, Mutex};

    fn make_snapshot(inv: [i64; 4], cand: [i64; 4]) -> SnapshotPlanificador {
        SnapshotPlanificador {
            inventario: ConteoPorGrupo {
                peliculas_no_anime: inv[0],
                series_no_anime: inv[1],
                peliculas_anime: inv[2],
                series_anime: inv[3],
                total: inv.iter().sum(),
            },
            candidatas: ConteoPorGrupo {
                peliculas_no_anime: cand[0],
                series_no_anime: cand[1],
                peliculas_anime: cand[2],
                series_anime: cand[3],
                total: cand.iter().sum(),
            },
            worker_state: TmdbWorkerState::default(),
            cooldowns: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_01_rotacion_determinista_cuatro_grupos() {
        let mut plan = PlanificadorGrupos::new();
        let inv = Arc::new(Mutex::new([100, 100, 100, 100]));
        let cand = Arc::new(Mutex::new([10, 10, 10, 10]));

        let mut atendidos = Vec::new();
        for _ in 0..4 {
            let inv_c = Arc::clone(&inv);
            let cand_c = Arc::clone(&cand);
            let res = ejecutar_ciclo_planificador_con(
                &mut plan,
                move || {
                    let i = *inv_c.lock().unwrap();
                    let c = *cand_c.lock().unwrap();
                    async move { Ok(make_snapshot(i, c)) }
                },
                |_g, _l| async move { Ok(10) },
            )
            .await;

            match res {
                ResultadoCiclo::TrabajoLocalRealizado { grupo, .. } => atendidos.push(grupo),
                other => panic!("Resultado inesperado: {:?}", other),
            }
        }

        assert_eq!(
            atendidos,
            vec![
                GrupoContenido::PeliculasNoAnime,
                GrupoContenido::SeriesNoAnime,
                GrupoContenido::PeliculasAnime,
                GrupoContenido::SeriesAnime,
            ]
        );
    }

    #[test]
    fn test_02_respeto_cursor_seleccion() {
        let mut plan = PlanificadorGrupos::new();
        plan.cursor_seleccion = 2; // PeliculasAnime
        let snap = make_snapshot([10, 10, 10, 10], [5, 5, 5, 5]);

        let sel = plan.seleccionar_siguiente_grupo(&snap.inventario, &snap.candidatas);
        assert_eq!(sel, Some((2, GrupoContenido::PeliculasAnime, 50)));
    }

    #[tokio::test]
    async fn test_03_avance_correcto_tras_marcar_grupo_sin_progreso() {
        let mut plan = PlanificadorGrupos::new();
        let llamadas = Arc::new(Mutex::new(Vec::new()));
        let llamadas_c = Arc::clone(&llamadas);

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            || async { Ok(make_snapshot([10, 10, 10, 10], [5, 5, 0, 0])) },
            move |grupo, _lote| {
                llamadas_c.lock().unwrap().push(grupo);
                async move {
                    if grupo == GrupoContenido::PeliculasNoAnime {
                        Ok(0) // Sin progreso en grupo 0 -> debe avanzar a SeriesNoAnime
                    } else {
                        Ok(5)
                    }
                }
            },
        )
        .await;

        assert_eq!(
            *llamadas.lock().unwrap(),
            vec![
                GrupoContenido::PeliculasNoAnime,
                GrupoContenido::SeriesNoAnime
            ]
        );
        assert!(plan.sin_progreso_en_ciclo[0]);
        assert!(!plan.sin_progreso_en_ciclo[1]);
        assert_eq!(
            res,
            ResultadoCiclo::TrabajoLocalRealizado {
                grupo: GrupoContenido::SeriesNoAnime,
                recuperadas: 5,
                quedan_candidatas_locales: true,
            }
        );
    }

    #[tokio::test]
    async fn test_04_no_repetir_grupo_agotado_dentro_del_mismo_ciclo() {
        let mut plan = PlanificadorGrupos::new();
        let llamadas = Arc::new(Mutex::new(Vec::new()));
        let cand = Arc::new(Mutex::new([5i64, 5, 5, 5]));
        let llamadas_c = Arc::clone(&llamadas);
        let cand_s = Arc::clone(&cand);
        let cand_r = Arc::clone(&cand);

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            move || {
                let c = *cand_s.lock().unwrap();
                async move { Ok(make_snapshot([10, 10, 10, 10], c)) }
            },
            move |grupo, _lote| {
                llamadas_c.lock().unwrap().push(grupo);
                let idx = GrupoContenido::all()
                    .iter()
                    .position(|&g| g == grupo)
                    .unwrap();
                cand_r.lock().unwrap()[idx] = 0;
                async move { Ok(0) }
            },
        )
        .await;

        // Cada grupo se intenta como máximo una vez por ciclo
        assert_eq!(
            *llamadas.lock().unwrap(),
            vec![
                GrupoContenido::PeliculasNoAnime,
                GrupoContenido::SeriesNoAnime,
                GrupoContenido::PeliculasAnime,
                GrupoContenido::SeriesAnime,
            ]
        );
        assert!(matches!(res, ResultadoCiclo::SinTrabajoLocalPendiente { .. }));
    }

    #[test]
    fn test_05_calculo_lotes_50_35_25_15_y_0() {
        assert_eq!(calcular_lote_local(0), 50);
        assert_eq!(calcular_lote_local(149), 50);
        assert_eq!(calcular_lote_local(150), 35);
        assert_eq!(calcular_lote_local(249), 35);
        assert_eq!(calcular_lote_local(250), 25);
        assert_eq!(calcular_lote_local(324), 25);
        assert_eq!(calcular_lote_local(325), 15);
        assert_eq!(calcular_lote_local(359), 15);
        assert_eq!(calcular_lote_local(375), 0);
        assert_eq!(calcular_lote_local(500), 0);
    }

    #[test]
    fn test_06_acotacion_estricta_lote_al_deficit_restante() {
        assert_eq!(calcular_lote_local(360), 15);
        assert_eq!(calcular_lote_local(365), 10);
        assert_eq!(calcular_lote_local(370), 5);
        assert_eq!(calcular_lote_local(374), 1);
    }

    #[test]
    fn test_07_no_superar_375_obras() {
        for inv in 0..=400 {
            let lote = calcular_lote_local(inv);
            if inv < 375 {
                assert!(inv + lote as i64 <= 375, "inv={} lote={}", inv, lote);
            } else {
                assert_eq!(lote, 0);
            }
        }
    }

    #[test]
    fn test_08_grupo_en_375_deja_de_ser_elegible() {
        let plan = PlanificadorGrupos::new();
        let snap = make_snapshot([375, 100, 380, 375], [50, 10, 50, 50]);

        let sel = plan.seleccionar_siguiente_grupo(&snap.inventario, &snap.candidatas);
        assert_eq!(sel, Some((1, GrupoContenido::SeriesNoAnime, 50)));
    }

    #[tokio::test]
    async fn test_09_recuperacion_local_exitosa_avanza_planificador() {
        let mut plan = PlanificadorGrupos::new();
        let inv = Arc::new(Mutex::new([370, 100, 375, 375]));
        let cand = Arc::new(Mutex::new([10, 10, 0, 0]));

        let inv_r = Arc::clone(&inv);
        let cand_r = Arc::clone(&cand);
        let inv_s = Arc::clone(&inv);
        let cand_s = Arc::clone(&cand);

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            move || {
                let i = *inv_s.lock().unwrap();
                let c = *cand_s.lock().unwrap();
                async move { Ok(make_snapshot(i, c)) }
            },
            move |_grupo, lote| {
                let mut i = inv_r.lock().unwrap();
                let mut c = cand_r.lock().unwrap();
                i[0] += lote as i64;
                c[0] -= lote as i64;
                async move { Ok(lote) }
            },
        )
        .await;

        assert_eq!(
            res,
            ResultadoCiclo::TrabajoLocalRealizado {
                grupo: GrupoContenido::PeliculasNoAnime,
                recuperadas: 5,
                quedan_candidatas_locales: true,
            }
        );
        assert_eq!(inv.lock().unwrap()[0], 375);
        assert_eq!(plan.cursor_seleccion, 1);
        assert_eq!(
            plan.ultimo_grupo_atendido,
            Some(GrupoContenido::PeliculasNoAnime)
        );
    }

    #[tokio::test]
    async fn test_10_grupo_sin_candidatas_permite_continuar_con_siguiente() {
        let mut plan = PlanificadorGrupos::new();
        let snap = make_snapshot([100, 100, 100, 100], [0, 0, 20, 0]);

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            move || {
                let s = snap.clone();
                async move { Ok(s) }
            },
            |grupo, lote| async move {
                assert_eq!(grupo, GrupoContenido::PeliculasAnime);
                assert_eq!(lote, 50);
                Ok(20)
            },
        )
        .await;

        assert_eq!(
            res,
            ResultadoCiclo::TrabajoLocalRealizado {
                grupo: GrupoContenido::PeliculasAnime,
                recuperadas: 20,
                quedan_candidatas_locales: true,
            }
        );
        assert_eq!(plan.cursor_seleccion, 3);
    }

    #[tokio::test]
    async fn test_11_error_sqlite_se_propaga_como_error_sqlite() {
        let mut plan = PlanificadorGrupos::new();

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            || async { Ok(make_snapshot([10, 10, 10, 10], [10, 10, 10, 10])) },
            |_g, _l| async { Err("SQLITE_CORRUPT simulado".to_string()) },
        )
        .await;

        assert_eq!(
            res,
            ResultadoCiclo::ErrorSqlite("SQLITE_CORRUPT simulado".to_string())
        );
        // No debe marcar el grupo como sin progreso ante un error SQLite
        assert_eq!(plan.sin_progreso_en_ciclo, [false; 4]);
    }

    #[tokio::test]
    async fn test_12_error_spawn_blocking_no_se_interpreta_como_sin_candidatas() {
        let mut plan = PlanificadorGrupos::new();

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            || async {
                let join_res: Result<Result<SnapshotPlanificador, String>, tokio::task::JoinError> =
                    tokio::task::spawn_blocking(|| -> Result<SnapshotPlanificador, String> {
                        panic!("fallo simulado en spawn_blocking");
                    })
                    .await;
                match join_res {
                    Ok(inner) => inner,
                    Err(e) => Err(format!("JoinError: {}", e)),
                }
            },
            |_g, _l| async { Ok(0) },
        )
        .await;

        match res {
            ResultadoCiclo::ErrorSqlite(msg) => assert!(msg.contains("JoinError")),
            other => panic!("Se esperaba ErrorSqlite y se obtuvo: {:?}", other),
        }
    }

    #[test]
    fn test_13_seleccion_ritmo_7_segundos_con_trabajo_local() {
        let res = ResultadoCiclo::TrabajoLocalRealizado {
            grupo: GrupoContenido::PeliculasNoAnime,
            recuperadas: 50,
            quedan_candidatas_locales: true,
        };
        assert_eq!(
            calcular_ritmo_siguiente(&res),
            RitmoEspera::TrabajoLocalContinuo { pausa_secs: 7 }
        );
    }

    #[test]
    fn test_14_seleccion_ritmo_15_300_sin_trabajo_pendiente() {
        let res_sin_local = ResultadoCiclo::SinTrabajoLocalPendiente {
            grupos_pendientes_tmdb: vec![GrupoContenido::SeriesAnime],
        };
        assert_eq!(
            calcular_ritmo_siguiente(&res_sin_local),
            RitmoEspera::Reposo {
                pausa_minima_secs: 15,
                espera_maxima_notify_secs: 300,
            }
        );

        assert_eq!(
            calcular_ritmo_siguiente(&ResultadoCiclo::SinTrabajoPendiente),
            RitmoEspera::Reposo {
                pausa_minima_secs: 15,
                espera_maxima_notify_secs: 300,
            }
        );
    }

    #[test]
    fn test_15_seleccion_backoff_45_segundos_ante_error_sqlite() {
        let res = ResultadoCiclo::ErrorSqlite("database is locked".to_string());
        assert_eq!(
            calcular_ritmo_siguiente(&res),
            RitmoEspera::ErrorBackoff {
                pausa_obligatoria_secs: 45,
                espera_maxima_notify_secs: 300,
            }
        );
    }

    #[tokio::test]
    async fn test_16_ausencia_candidatas_locales_no_dispara_http_y_retorna_pendiente_d3() {
        let mut plan = PlanificadorGrupos::new();
        let inv_llamadas = Arc::new(Mutex::new(0usize));
        let rec_llamadas = Arc::clone(&inv_llamadas);

        let res = ejecutar_ciclo_planificador_con(
            &mut plan,
            || async { Ok(make_snapshot([100, 375, 200, 375], [0, 0, 0, 0])) },
            move |_g, _l| {
                *rec_llamadas.lock().unwrap() += 1;
                async { Ok(0) }
            },
        )
        .await;

        assert_eq!(*inv_llamadas.lock().unwrap(), 0);
        assert_eq!(
            res,
            ResultadoCiclo::SinTrabajoLocalPendiente {
                grupos_pendientes_tmdb: vec![
                    GrupoContenido::PeliculasNoAnime,
                    GrupoContenido::PeliculasAnime,
                ],
            }
        );
        assert_eq!(TmdbEndpointKind::Discover.as_str(), "discover");
        assert_eq!(TmdbEndpointKind::TvDetail.as_str(), "tv_detail");
    }

    // ------------------------------------------------------------------------
    // Etapa 5.9.2D-3: Pruebas unitarias de integración TMDB y presupuesto HTTP
    // ------------------------------------------------------------------------

    fn setup_in_memory_db_d3() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("abrir SQLite en memoria");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
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
        .unwrap();
        crate::db::init_db_conn(&conn).unwrap();
        conn
    }

    #[tokio::test]
    async fn test_17_condicion_and_cumplida_habilita_tmdb() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            || async { Ok(make_snapshot([100, 375, 375, 375], [0, 0, 0, 0])) },
            |_g, _l| async { Ok(0) },
            |_ep, _inicio| async {
                Ok(crate::models::ReservaTmdbResultado::Reservada {
                    reserva_id: 1,
                    peticiones_en_ventana: 1,
                    restante_en_ventana: 17,
                })
            },
            move |grupo| {
                *http_c.lock().unwrap() += 1;
                assert_eq!(grupo, GrupoContenido::PeliculasNoAnime);
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 3,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 1);
        assert_eq!(
            res,
            ResultadoCiclo::TrabajoExternoRealizado {
                grupo: GrupoContenido::PeliculasNoAnime,
                insertadas: 3,
                quedan_candidatas_locales: false,
            }
        );
    }

    #[tokio::test]
    async fn test_18_no_http_si_existen_candidatas_locales() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            || async { Ok(make_snapshot([100, 375, 375, 375], [5, 0, 0, 0])) },
            |_g, _l| async { Ok(5) },
            |_ep, _inicio| async { panic!("No debe reservar TMDB si hay candidatas locales") },
            move |_grupo| {
                *http_c.lock().unwrap() += 1;
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 1,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 0);
        assert!(matches!(res, ResultadoCiclo::TrabajoLocalRealizado { .. }));
    }

    #[tokio::test]
    async fn test_19_no_http_si_inventario_alcanzo_375() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            || async { Ok(make_snapshot([375, 375, 375, 375], [0, 0, 0, 0])) },
            |_g, _l| async { Ok(0) },
            |_ep, _inicio| async { panic!("No debe reservar TMDB si inventario == 375") },
            move |_grupo| {
                *http_c.lock().unwrap() += 1;
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 0,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 0);
        assert_eq!(res, ResultadoCiclo::SinTrabajoPendiente);
    }

    #[tokio::test]
    async fn test_20_no_http_si_bloqueo_global_activo() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            move || async move {
                let mut s = make_snapshot([100, 375, 375, 375], [0, 0, 0, 0]);
                s.worker_state.tmdb_bloqueado_hasta = Some(now + 120);
                Ok(s)
            },
            |_g, _l| async { Ok(0) },
            |_ep, _inicio| async { panic!("No debe reservar si hay bloqueo global") },
            move |_grupo| {
                *http_c.lock().unwrap() += 1;
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 0,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 0);
        assert!(matches!(res, ResultadoCiclo::SinTrabajoLocalPendiente { .. }));
    }

    #[tokio::test]
    async fn test_21_no_http_si_cooldown_de_grupo_activo() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            move || async move {
                let mut s = make_snapshot([100, 375, 375, 375], [0, 0, 0, 0]);
                s.cooldowns.push(crate::models::TmdbGroupCooldown {
                    grupo: GrupoContenido::PeliculasNoAnime,
                    bloqueado_hasta: now + 300,
                    fallos_sin_aporte: 1,
                    motivo: Some("Cooldown".to_string()),
                });
                Ok(s)
            },
            |_g, _l| async { Ok(0) },
            |_ep, _inicio| async { panic!("No debe reservar si grupo en cooldown") },
            move |_grupo| {
                *http_c.lock().unwrap() += 1;
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 0,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 0);
        assert!(matches!(res, ResultadoCiclo::SinTrabajoLocalPendiente { .. }));
    }

    #[tokio::test]
    async fn test_22_no_http_si_presupuesto_no_autoriza() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;
        let http_calls = Arc::new(Mutex::new(0usize));
        let http_c = Arc::clone(&http_calls);

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            || async { Ok(make_snapshot([100, 375, 375, 375], [0, 0, 0, 0])) },
            |_g, _l| async { Ok(0) },
            |_ep, _inicio| async {
                Ok(crate::models::ReservaTmdbResultado::PresupuestoAgotado {
                    peticiones_en_ventana: 17,
                    limite_aplicado: 16,
                })
            },
            move |_grupo| {
                *http_c.lock().unwrap() += 1;
                async {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 0,
                        completada_sin_corte: true,
                    }
                }
            },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(*http_calls.lock().unwrap(), 0);
        assert!(matches!(res, ResultadoCiclo::SinTrabajoLocalPendiente { .. }));
    }

    #[tokio::test]
    async fn test_23_maximo_8_peticiones_por_operacion_y_corte_limpio() {
        let mut ctrl = ControladorPresupuestoOperacion::nuevo_tras_inicio();
        assert_eq!(ctrl.ops_requests_usadas, 1);

        // Permite 7 internas (total 8)
        for _ in 0..7 {
            let ok = ctrl
                .intentar_reservar_interna(TmdbEndpointKind::TvDetail, |_ep, inicio| async move {
                    assert!(!inicio);
                    Ok(crate::models::ReservaTmdbResultado::Reservada {
                        reserva_id: 1,
                        peticiones_en_ventana: 2,
                        restante_en_ventana: 16,
                    })
                })
                .await
                .unwrap();
            assert!(ok);
        }
        assert_eq!(ctrl.ops_requests_usadas, 8);

        // La petición 9 debe cortarse limpiamente sin intentar reservar ni hacer HTTP
        let mut llamo_reserva = false;
        let ok_9 = ctrl
            .intentar_reservar_interna(TmdbEndpointKind::TvDetail, |_ep, _inicio| {
                llamo_reserva = true;
                async {
                    Ok(crate::models::ReservaTmdbResultado::Reservada {
                        reserva_id: 9,
                        peticiones_en_ventana: 9,
                        restante_en_ventana: 9,
                    })
                }
            })
            .await
            .unwrap();
        assert!(!ok_9);
        assert!(!llamo_reserva);
        assert_eq!(ctrl.ops_requests_usadas, 8);
    }

    #[test]
    fn test_24_limite_global_18_en_300s_y_intervalo_60s_e_internas() {
        let mut conn = setup_in_memory_db_d3();
        let now = 1_700_000_000;

        // Inicio de operación
        let r1 = crate::db::reservar_peticion_tmdb_at(
            &mut conn,
            TmdbEndpointKind::Discover.as_str(),
            true,
            now,
        )
        .unwrap();
        assert!(matches!(r1, crate::models::ReservaTmdbResultado::Reservada { .. }));

        // Otro inicio a +30s falla por intervalo de 60s
        let r_early = crate::db::reservar_peticion_tmdb_at(
            &mut conn,
            TmdbEndpointKind::Discover.as_str(),
            true,
            now + 30,
        )
        .unwrap();
        assert!(matches!(
            r_early,
            crate::models::ReservaTmdbResultado::IntervaloOperacionNoCumplido { .. }
        ));

        // Internas dentro de los 60s sí están permitidas hasta 18 totales
        for i in 1..18 {
            let r_int = crate::db::reservar_peticion_tmdb_at(
                &mut conn,
                TmdbEndpointKind::TvDetail.as_str(),
                false,
                now + i,
            )
            .unwrap();
            assert!(matches!(
                r_int,
                crate::models::ReservaTmdbResultado::Reservada { .. }
            ));
        }

        // La 19 es rechazada
        let r_19 = crate::db::reservar_peticion_tmdb_at(
            &mut conn,
            TmdbEndpointKind::TvDetail.as_str(),
            false,
            now + 20,
        )
        .unwrap();
        assert!(matches!(
            r_19,
            crate::models::ReservaTmdbResultado::PresupuestoAgotado { .. }
        ));
    }

    #[test]
    fn test_25_parseo_retry_after_segundos() {
        let now = 1_700_000_000;
        assert_eq!(parse_retry_after("120", now), Some(120));
        assert_eq!(parse_retry_after("  0  ", now), Some(0));
        assert_eq!(parse_retry_after("7200", now), Some(7200)); // No recorta arbitrariamente
    }

    #[test]
    fn test_26_parseo_retry_after_fecha_http() {
        // 21 Oct 2015 07:28:00 GMT = 1445412480
        let target = 1_445_412_480;
        let now = target - 45;
        assert_eq!(
            parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT", now),
            Some(45)
        );
        assert_eq!(
            parse_retry_after("Wed, 21 Oct 2015 07:28:00 +0000", now),
            Some(45)
        );
    }

    #[test]
    fn test_27_parseo_retry_after_invalido_o_vencido() {
        let target = 1_445_412_480;
        // Vencido -> 0 segundos restantes
        assert_eq!(
            parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT", target + 100),
            Some(0)
        );
        // Inválidos -> None
        assert_eq!(parse_retry_after("-5", target), None);
        assert_eq!(parse_retry_after("", target), None);
        assert_eq!(parse_retry_after("not-a-date", target), None);
        assert_eq!(parse_retry_after("Wed, 32 Oct 2015 07:28:00 GMT", target), None);
    }

    #[test]
    fn test_28_persistencia_bloqueo_global_ante_429_y_cooldown_grupo() {
        let mut conn = setup_in_memory_db_d3();
        let now = 1_700_000_000;

        persistir_resultado_operacion_tmdb_conn(
            &mut conn,
            GrupoContenido::SeriesNoAnime,
            &ResultadoOperacionTmdb::Http429 {
                insertadas: 0,
                retry_after_secs: Some(90),
            },
            now,
        )
        .unwrap();

        let st = crate::db::get_tmdb_worker_state_conn(&conn).unwrap();
        assert_eq!(st.tmdb_bloqueado_hasta, Some(now + 90));
        assert_eq!(st.errores_consecutivos, 1);

        let cd = crate::db::get_cooldown_grupo_conn(&conn, GrupoContenido::SeriesNoAnime)
            .unwrap()
            .unwrap();
        assert_eq!(cd.bloqueado_hasta, now + 300);
        assert_eq!(cd.fallos_sin_aporte, 1);
    }

    #[test]
    fn test_29_backoff_progresivo_operaciones_validas_sin_inserciones() {
        let mut conn = setup_in_memory_db_d3();
        let now = 1_700_000_000;
        let esperado = [300_i64, 600, 1200, 1800, 1800];

        for (i, &secs) in esperado.iter().enumerate() {
            persistir_resultado_operacion_tmdb_conn(
                &mut conn,
                GrupoContenido::PeliculasAnime,
                &ResultadoOperacionTmdb::Completada {
                    insertadas: 0,
                    completada_sin_corte: true,
                },
                now,
            )
            .unwrap();

            let cd = crate::db::get_cooldown_grupo_conn(&conn, GrupoContenido::PeliculasAnime)
                .unwrap()
                .unwrap();
            assert_eq!(cd.fallos_sin_aporte, (i + 1) as i64);
            assert_eq!(cd.bloqueado_hasta, now + secs);
        }

        // Un corte por presupuesto con 0 inserciones NO incrementa fallos_sin_aporte
        persistir_resultado_operacion_tmdb_conn(
            &mut conn,
            GrupoContenido::PeliculasAnime,
            &ResultadoOperacionTmdb::CortadaPorPresupuesto { insertadas: 0 },
            now,
        )
        .unwrap();
        let cd_after = crate::db::get_cooldown_grupo_conn(&conn, GrupoContenido::PeliculasAnime)
            .unwrap()
            .unwrap();
        assert_eq!(cd_after.fallos_sin_aporte, 5);
    }

    #[test]
    fn test_30_error_red_o_5xx_conserva_resultados_previos() {
        let mut conn = setup_in_memory_db_d3();
        let now = 1_700_000_000;

        let obra = ObraTmdbDescubierta {
            tmdb_id: 999,
            tipo: "MOVIE".to_string(),
            es_anime: false,
            titulo: "Previa".to_string(),
            titulo_original: None,
            sinopsis: None,
            fecha_estreno: None,
            poster_path: Some("/prev.jpg".to_string()),
            generos: None,
            backdrop_url: None,
        };
        assert!(insert_to_catalog_conn(&conn, &obra).unwrap());

        persistir_resultado_operacion_tmdb_conn(
            &mut conn,
            GrupoContenido::PeliculasNoAnime,
            &ResultadoOperacionTmdb::HttpError5xxORed {
                insertadas: 1,
                motivo: "HTTP 503".to_string(),
                es_5xx: true,
            },
            now,
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM catalogo_offline", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let st = crate::db::get_tmdb_worker_state_conn(&conn).unwrap();
        assert!(st.tmdb_bloqueado_hasta.is_some());
        assert_eq!(st.errores_consecutivos, 1);
    }

    #[tokio::test]
    async fn test_31_continuidad_recuperacion_local_7s_cuando_tmdb_esta_bloqueado() {
        let mut plan = PlanificadorGrupos::new();
        let now = 1_700_000_000;

        let res = ejecutar_ciclo_completo_con(
            &mut plan,
            now,
            move || async move {
                let mut s = make_snapshot([100, 100, 375, 375], [0, 50, 0, 0]);
                s.worker_state.tmdb_bloqueado_hasta = Some(now + 600);
                Ok(s)
            },
            |grupo, _lote| async move {
                assert_eq!(grupo, GrupoContenido::SeriesNoAnime);
                Ok(50)
            },
            |_ep, _inicio| async { panic!("No debe tocar TMDB") },
            |_g| async { unreachable!() },
            |_g, _r| async { Ok(()) },
        )
        .await;

        assert_eq!(
            res,
            ResultadoCiclo::TrabajoLocalRealizado {
                grupo: GrupoContenido::SeriesNoAnime,
                recuperadas: 50,
                quedan_candidatas_locales: true,
            }
        );
        assert_eq!(
            calcular_ritmo_siguiente(&res),
            RitmoEspera::TrabajoLocalContinuo { pausa_secs: 7 }
        );
    }

    #[tokio::test]
    async fn test_32_aislamiento_insert_to_catalog_e_insercion_sin_metadatos() {
        let mut conn = setup_in_memory_db_d3();

        let obra_incompleta = ObraTmdbDescubierta {
            tmdb_id: 12345,
            tipo: "MOVIE".to_string(),
            es_anime: false,
            titulo: "Obra Sin Metadatos".to_string(),
            titulo_original: None,
            sinopsis: None,
            fecha_estreno: None,
            poster_path: None,
            generos: None,
            backdrop_url: None,
        };

        let inserted = insert_to_catalog_conn(&mut conn, &obra_incompleta).unwrap();
        assert!(inserted);

        let inv = crate::db::get_inventario_grupo_conn(&conn, GrupoContenido::PeliculasNoAnime)
            .unwrap();
        assert_eq!(inv, 1);

        // Verificar también que spawn_blocking no cruza conexión ni paniquea en tipo Send
        let handle = tokio::task::spawn_blocking(move || {
            let c = setup_in_memory_db_d3();
            insert_to_catalog_conn(&c, &obra_incompleta)
        });
        assert!(handle.await.unwrap().unwrap());
    }

    #[test]
    fn test_33_preservacion_poster_path_relativo_sin_duplicar_prefijos() {
        assert_eq!(
            normalizar_poster_path(Some("/abc.jpg")),
            Some("/abc.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_path(Some("abc.jpg")),
            Some("/abc.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_path(Some("https://image.tmdb.org/t/p/w500/abc.jpg")),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(
            normalizar_poster_path(Some(
                "https://image.tmdb.org/t/p/w500https://image.tmdb.org/t/p/w500/abc.jpg"
            )),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(normalizar_poster_path(Some("   ")), None);
        assert_eq!(normalizar_poster_path(None), None);
    }

    #[test]
    fn test_34_estrategias_descubrimiento_y_urls_por_grupo() {
        assert_eq!(
            seleccionar_estrategia_descubrimiento(10),
            EstrategiaDescubrimiento::ConocidasBienValoradas
        );
        assert_eq!(
            seleccionar_estrategia_descubrimiento(55),
            EstrategiaDescubrimiento::MenosPopularesDescubrimientos
        );
        assert_eq!(
            seleccionar_estrategia_descubrimiento(80),
            EstrategiaDescubrimiento::DiversidadGeneros
        );
        assert_eq!(
            seleccionar_estrategia_descubrimiento(95),
            EstrategiaDescubrimiento::ClasicosCulto
        );

        let url_anime = construir_url_discover_grupo(
            "KEY",
            GrupoContenido::SeriesAnime,
            EstrategiaDescubrimiento::ConocidasBienValoradas,
            2020,
            28,
        );
        assert!(url_anime.contains("/3/discover/tv"));
        assert!(url_anime.contains("with_genres=16&with_origin_country=JP"));
        assert!(url_anime.contains("include_adult=false"));
    }

    #[tokio::test]
    async fn test_35_cola_posters_maximo_5_sin_duplicados_y_archivo_existente() {
        let dir = std::env::temp_dir().join(format!(
            "queveohoy_d4_queue_test_{}_{}",
            std::process::id(),
            crate::db::current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // ID 10 ya existe en disco: no debe descargarse, pero cuenta en el límite de 5 y se marca éxito
        std::fs::write(dir.join("10.jpg"), b"existing").unwrap();

        let make_state = |id: i64, url: &str| crate::models::PosterRetryState {
            contenido_id: id,
            url_fallida: url.to_string(),
            intentos: 0,
            estado: "pendiente".to_string(),
            proximo_reintento: None,
            ultimo_error: None,
            actualizado_en: 0,
        };

        let candidatas = vec![
            make_state(10, "/10.jpg"),
            make_state(10, "/10.jpg"), // duplicado
            make_state(11, "/11.jpg"),
            make_state(12, "https://cdn.example.com/12.jpg"),
            make_state(13, "/13.jpg"),
            make_state(14, "/14.jpg"),
            make_state(15, "/15.jpg"), // 6to único, fuera del límite de 5
        ];

        let descargas = Arc::new(Mutex::new(Vec::<(i64, String)>::new()));
        let guardados = Arc::new(Mutex::new(Vec::<crate::db::ResultadoDescargaPoster>::new()));

        let descargas_cl = Arc::clone(&descargas);
        let guardados_cl = Arc::clone(&guardados);

        let procesados = procesar_cola_posters_con(
            &dir,
            MAX_POSTERS_POR_CICLO,
            move |_lim| {
                let c = candidatas.clone();
                async move { Ok(c) }
            },
            move |id, url| {
                let d = Arc::clone(&descargas_cl);
                async move {
                    d.lock().unwrap().push((id, url));
                    if id == 13 {
                        Err("HTTP 404".to_string())
                    } else {
                        Ok(())
                    }
                }
            },
            move |res| {
                let g = Arc::clone(&guardados_cl);
                async move {
                    *g.lock().unwrap() = res;
                    Ok(())
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(procesados, 5);
        let d = descargas.lock().unwrap();
        assert_eq!(
            *d,
            vec![
                (11, "https://image.tmdb.org/t/p/w500/11.jpg".to_string()),
                (12, "https://cdn.example.com/12.jpg".to_string()),
                (13, "https://image.tmdb.org/t/p/w500/13.jpg".to_string()),
                (14, "https://image.tmdb.org/t/p/w500/14.jpg".to_string()),
            ]
        );

        let g = guardados.lock().unwrap();
        assert_eq!(g.len(), 5);
        assert_eq!(
            g[0],
            crate::db::ResultadoDescargaPoster::Exito { contenido_id: 10 }
        );
        assert_eq!(
            g[3],
            crate::db::ResultadoDescargaPoster::Fallo {
                contenido_id: 13,
                url: "/13.jpg".to_string(),
                error: "HTTP 404".to_string(),
            }
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_36_cola_posters_resolucion_urls_relativas_y_absolutas() {
        assert_eq!(
            resolver_url_descarga_poster("/abc.jpg"),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(
            resolver_url_descarga_poster("https://image.tmdb.org/t/p/w500/abc.jpg"),
            Some("https://image.tmdb.org/t/p/w500/abc.jpg".to_string())
        );
        assert_eq!(
            resolver_url_descarga_poster("http://cdn.example.com/1.jpg"),
            Some("http://cdn.example.com/1.jpg".to_string())
        );
        assert_eq!(
            resolver_url_descarga_poster(
                "https://image.tmdb.org/t/p/w500https://cdn.myanimelist.net/1.jpg"
            ),
            Some("https://cdn.myanimelist.net/1.jpg".to_string())
        );
        assert_eq!(resolver_url_descarga_poster("abc.jpg"), None);
        assert_eq!(resolver_url_descarga_poster("   "), None);
    }

    #[tokio::test]
    async fn test_37_cola_posters_error_sqlite_no_se_registra_como_fallo_descarga() {
        let dir = std::env::temp_dir().join(format!(
            "queveohoy_d4_sqlerr_test_{}_{}",
            std::process::id(),
            crate::db::current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let descargas_llamadas = Arc::new(Mutex::new(0usize));
        let descargas_cl = Arc::clone(&descargas_llamadas);
        let err = procesar_cola_posters_con(
            &dir,
            MAX_POSTERS_POR_CICLO,
            |_| async { Err("SQLite busy".to_string()) },
            move |_, _| {
                let d = Arc::clone(&descargas_cl);
                async move {
                    *d.lock().unwrap() += 1;
                    Ok(())
                }
            },
            |_| async { Ok(()) },
        )
        .await
        .unwrap_err();
        assert!(err.contains("SQLite busy"));
        assert_eq!(*descargas_llamadas.lock().unwrap(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_38_planificador_posters_no_bloquean_local_y_corren_con_tmdb_bloqueado() {
        // 1. Si hay trabajo local, se atiende local y NO se ejecuta mantenimiento de pósteres
        let mut plan = PlanificadorGrupos::default();
        let posters_llamados = Arc::new(Mutex::new(0usize));
        let p_cl = Arc::clone(&posters_llamados);
        let res_local = ejecutar_ciclo_completo_con_posters(
            &mut plan,
            1_700_000_000,
            || async { Ok(make_snapshot([100, 375, 375, 375], [20, 0, 0, 0])) },
            |_g, _lote| async { Ok(10) },
            |_, _| async { unreachable!("no debe llamar TMDB") },
            |_| async { unreachable!("no debe llamar TMDB") },
            |_, _| async { unreachable!("no debe llamar TMDB") },
            move || {
                let p = Arc::clone(&p_cl);
                async move {
                    *p.lock().unwrap() += 1;
                    Ok(3)
                }
            },
        )
        .await;
        assert!(matches!(
            res_local,
            ResultadoCiclo::TrabajoLocalRealizado { .. }
        ));
        assert_eq!(*posters_llamados.lock().unwrap(), 0);
        assert_eq!(
            calcular_ritmo_siguiente(&res_local),
            RitmoEspera::TrabajoLocalContinuo { pausa_secs: 7 }
        );

        // 2. Si TMDB está bloqueado y no hay trabajo local, corre cola de pósteres y retorna SoloMantenimientoPosters (15s + Notify)
        let res_posters = ejecutar_ciclo_completo_con_posters(
            &mut plan,
            1_700_000_000,
            || async { Ok(make_snapshot([100, 375, 375, 375], [0, 0, 0, 0])) },
            |_, _| async { Ok(0) },
            |_, _| async {
                Ok(crate::models::ReservaTmdbResultado::BloqueoActivo {
                    bloqueado_hasta: 1_700_005_000,
                    motivo: Some("429".to_string()),
                })
            },
            |_| async { unreachable!("TMDB bloqueado") },
            |_, _| async { unreachable!("TMDB bloqueado") },
            || async { Ok(4) },
        )
        .await;
        assert_eq!(
            res_posters,
            ResultadoCiclo::SoloMantenimientoPosters {
                posters_procesados: 4
            }
        );
        assert_eq!(
            calcular_ritmo_siguiente(&res_posters),
            RitmoEspera::Reposo {
                pausa_minima_secs: 15,
                espera_maxima_notify_secs: 300,
            }
        );
    }

    #[tokio::test]
    async fn test_39_d4_1_post_tmdb_ejecuta_mantenimiento_posters_despues_de_persistir() {
        let mut plan = PlanificadorGrupos::default();
        let eventos = Arc::new(Mutex::new(Vec::<&'static str>::new()));

        let ev_exec = Arc::clone(&eventos);
        let ev_pers = Arc::clone(&eventos);
        let ev_post = Arc::clone(&eventos);

        let dir = std::env::temp_dir().join(format!(
            "queveohoy_d4_1_post_tmdb_{}_{}",
            std::process::id(),
            crate::db::current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // ID 100 ya existe en disco: debe reconocerse sin descargar y contar dentro del límite de 5
        std::fs::write(dir.join("100.jpg"), b"cached").unwrap();

        let descargas = Arc::new(Mutex::new(Vec::<i64>::new()));
        let descargas_cl = Arc::clone(&descargas);
        let dir_cl = dir.clone();

        let res = ejecutar_ciclo_completo_con_posters(
            &mut plan,
            1_700_000_000,
            || async { Ok(make_snapshot([375, 375, 50, 375], [0, 0, 0, 0])) },
            |_, _| async { Ok(0) },
            |_, es_inicio| async move {
                assert!(es_inicio);
                Ok(crate::models::ReservaTmdbResultado::Reservada {
                    reserva_id: 1,
                    peticiones_en_ventana: 1,
                    restante_en_ventana: 17,
                })
            },
            move |_grupo| {
                let ev = Arc::clone(&ev_exec);
                async move {
                    ev.lock().unwrap().push("ejecutar_tmdb");
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 3,
                        completada_sin_corte: true,
                    }
                }
            },
            move |_, _| {
                let ev = Arc::clone(&ev_pers);
                async move {
                    ev.lock().unwrap().push("persistir_tmdb");
                    Ok(())
                }
            },
            move || {
                let ev = Arc::clone(&ev_post);
                let d_cl = Arc::clone(&descargas_cl);
                let cache_dir = dir_cl.clone();
                async move {
                    ev.lock().unwrap().push("mantener_posters");
                    procesar_cola_posters_con(
                        &cache_dir,
                        MAX_POSTERS_POR_CICLO,
                        |_lim| async move {
                            Ok((100..=106)
                                .map(|id| crate::models::PosterRetryState {
                                    contenido_id: id,
                                    url_fallida: format!("/{}.jpg", id),
                                    intentos: 0,
                                    estado: "pendiente".to_string(),
                                    proximo_reintento: None,
                                    ultimo_error: None,
                                    actualizado_en: 0,
                                })
                                .collect())
                        },
                        move |id, _url| {
                            let d = Arc::clone(&d_cl);
                            async move {
                                d.lock().unwrap().push(id);
                                Ok(())
                            }
                        },
                        |_res| async { Ok(()) },
                    )
                    .await
                }
            },
        )
        .await;

        assert_eq!(
            res,
            ResultadoCiclo::TrabajoExternoRealizado {
                grupo: GrupoContenido::PeliculasAnime,
                insertadas: 3,
                quedan_candidatas_locales: false,
            }
        );
        assert_eq!(
            calcular_ritmo_siguiente(&res),
            RitmoEspera::Reposo {
                pausa_minima_secs: 15,
                espera_maxima_notify_secs: 300,
            }
        );
        assert_eq!(
            *eventos.lock().unwrap(),
            vec!["ejecutar_tmdb", "persistir_tmdb", "mantener_posters"]
        );
        // Máximo 5 procesados: 100 existente sin descarga + 101..=104 descargados (105 y 106 fuera del cupo)
        assert_eq!(*descargas.lock().unwrap(), vec![101, 102, 103, 104]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_40_d4_1_error_descarga_o_sqlite_posters_no_falla_trabajo_externo() {
        // Caso A: falla descarga de póster -> persiste fallo de póster y mantiene TrabajoExternoRealizado
        let mut plan = PlanificadorGrupos::default();
        let dir = std::env::temp_dir().join(format!(
            "queveohoy_d4_1_err_{}_{}",
            std::process::id(),
            crate::db::current_unix_timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dir_cl = dir.clone();
        let guardados = Arc::new(Mutex::new(Vec::<crate::db::ResultadoDescargaPoster>::new()));
        let g_cl = Arc::clone(&guardados);

        let res_down_err = ejecutar_ciclo_completo_con_posters(
            &mut plan,
            1_700_000_000,
            || async { Ok(make_snapshot([375, 375, 50, 375], [0, 0, 0, 0])) },
            |_, _| async { Ok(0) },
            |_, _| async {
                Ok(crate::models::ReservaTmdbResultado::Reservada {
                    reserva_id: 1,
                    peticiones_en_ventana: 1,
                    restante_en_ventana: 17,
                })
            },
            |_grupo| async move {
                ResultadoOperacionTmdb::Completada {
                    insertadas: 2,
                    completada_sin_corte: true,
                }
            },
            |_, _| async { Ok(()) },
            move || {
                let cache_dir = dir_cl.clone();
                let g = Arc::clone(&g_cl);
                async move {
                    procesar_cola_posters_con(
                        &cache_dir,
                        MAX_POSTERS_POR_CICLO,
                        |_| async move {
                            Ok(vec![crate::models::PosterRetryState {
                                contenido_id: 501,
                                url_fallida: "/501.jpg".to_string(),
                                intentos: 0,
                                estado: "pendiente".to_string(),
                                proximo_reintento: None,
                                ultimo_error: None,
                                actualizado_en: 0,
                            }])
                        },
                        |_id, _url| async { Err("HTTP 503".to_string()) },
                        move |batch| {
                            let g_inner = Arc::clone(&g);
                            async move {
                                *g_inner.lock().unwrap() = batch;
                                Ok(())
                            }
                        },
                    )
                    .await
                }
            },
        )
        .await;

        assert_eq!(
            res_down_err,
            ResultadoCiclo::TrabajoExternoRealizado {
                grupo: GrupoContenido::PeliculasAnime,
                insertadas: 2,
                quedan_candidatas_locales: false,
            }
        );
        assert_eq!(
            *guardados.lock().unwrap(),
            vec![crate::db::ResultadoDescargaPoster::Fallo {
                contenido_id: 501,
                url: "/501.jpg".to_string(),
                error: "HTTP 503".to_string(),
            }]
        );

        // Caso B: error SQLite en cola de pósteres tras persistir TMDB -> no convierte TMDB en ErrorSqlite
        let res_sql_err = ejecutar_ciclo_completo_con_posters(
            &mut plan,
            1_700_000_000,
            || async { Ok(make_snapshot([375, 375, 50, 375], [0, 0, 0, 0])) },
            |_, _| async { Ok(0) },
            |_, _| async {
                Ok(crate::models::ReservaTmdbResultado::Reservada {
                    reserva_id: 1,
                    peticiones_en_ventana: 1,
                    restante_en_ventana: 17,
                })
            },
            |_grupo| async move {
                ResultadoOperacionTmdb::Completada {
                    insertadas: 1,
                    completada_sin_corte: true,
                }
            },
            |_, _| async { Ok(()) },
            || async { Err("SQLite busy en poster_retry_state".to_string()) },
        )
        .await;

        assert!(matches!(
            res_sql_err,
            ResultadoCiclo::TrabajoExternoRealizado { insertadas: 1, .. }
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_41_d4_1_sucesivos_ciclos_tmdb_avanzan_cola_posters_sin_inanicion() {
        let mut plan = PlanificadorGrupos::default();
        let reservas = Arc::new(Mutex::new(0usize));
        let ciclos_posters = Arc::new(Mutex::new(0usize));

        for ciclo in 0..2 {
            let r_cl = Arc::clone(&reservas);
            let p_cl = Arc::clone(&ciclos_posters);
            let res = ejecutar_ciclo_completo_con_posters(
                &mut plan,
                1_700_000_000 + (ciclo as i64) * 315,
                || async { Ok(make_snapshot([375, 375, 50, 317], [0, 0, 0, 0])) },
                |_, _| async { Ok(0) },
                move |_, _| {
                    let r = Arc::clone(&r_cl);
                    async move {
                        *r.lock().unwrap() += 1;
                        Ok(crate::models::ReservaTmdbResultado::Reservada {
                            reserva_id: 1,
                            peticiones_en_ventana: 1,
                            restante_en_ventana: 17,
                        })
                    }
                },
                |_grupo| async move {
                    ResultadoOperacionTmdb::Completada {
                        insertadas: 2,
                        completada_sin_corte: true,
                    }
                },
                |_, _| async { Ok(()) },
                move || {
                    let p = Arc::clone(&p_cl);
                    async move {
                        *p.lock().unwrap() += 1;
                        Ok(5)
                    }
                },
            )
            .await;

            let expected_group = if ciclo == 0 {
                GrupoContenido::PeliculasAnime
            } else {
                GrupoContenido::SeriesAnime
            };
            assert_eq!(
                res,
                ResultadoCiclo::TrabajoExternoRealizado {
                    grupo: expected_group,
                    insertadas: 2,
                    quedan_candidatas_locales: false,
                }
            );
        }

        assert_eq!(*reservas.lock().unwrap(), 2);
        assert_eq!(*ciclos_posters.lock().unwrap(), 2);
    }
}
