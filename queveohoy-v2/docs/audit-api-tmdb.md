# Auditoría de Integración API: TMDB

## Endpoints Utilizados
En todo el código fuente de QuéVeoHoy 2.0 (Tauri), el único endpoint de TMDB que se está consultando es:

- **Endpoint:** `GET https://api.themoviedb.org/3/tv/{id}?api_key={key}&language=es-ES`
- **Ubicación en código:** `src-tauri/src/tmdb.rs` -> Función `fetch_and_cache_temporadas()`

---

## Análisis de la Respuesta de la API vs Lo que Guardamos

### Campos que devuelve la API (Respuesta completa)
El endpoint `/tv/{id}` devuelve un objeto JSON extenso con los siguientes campos a nivel raíz:
`adult`, `backdrop_path`, `created_by`, `episode_run_time`, `first_air_date`, `genres`, `homepage`, `id`, `in_production`, `languages`, `last_air_date`, `last_episode_to_air`, `name`, `next_episode_to_air`, `networks`, `number_of_episodes`, `number_of_seasons`, `origin_country`, `original_language`, `original_name`, `overview`, `popularity`, `poster_path`, `production_companies`, `production_countries`, `seasons` (Array), `softcore`, `spoken_languages`, `status`, `tagline`, `type`, `vote_average`, `vote_count`.

El array `seasons` contiene objetos con:
`air_date`, `episode_count`, `id`, `name`, `overview`, `poster_path`, `season_number`, `vote_average`.

### Campos que EFECTIVAMENTE parseamos y guardamos
De toda la respuesta, nuestro parser en Rust **solo itera sobre el array `seasons`** y extrae:
1. `season_number`
2. `episode_count`

**Dónde se guarda:**
- Generamos un JSON Map (`{"1": 11, "2": 12}`) descartando la temporada 0 (Especiales).
- Guardamos la cantidad de keys en la columna `total_temporadas`.
- Guardamos el string JSON serializado en la columna `temporadas_json`.
- Todo esto va a la tabla `tv_metadata_cache`.

### Campos que NO estamos guardando (Ignorados)
- **A nivel show:** No estamos actualizando `status` (Ej. para saber si finalizó o sigue emitiendo), `number_of_seasons` general, ni `number_of_episodes`.
- **A nivel temporada (`seasons`):** Ignoramos `name` (ej. "Arco del Distrito Rojo"), `air_date` (fecha de estreno de la temporada), y `overview` (sinopsis de la temporada).
- **A nivel episodio (Nombres de episodios, sinopsis, duración):** TMDB **NO** devuelve esta información en el endpoint `/tv/{id}`. Para obtener los nombres y sinopsis de cada episodio, la API requiere llamar al endpoint `/tv/{id}/season/{season_number}`. Como **nunca** hacemos esa llamada en nuestro código actual, no poseemos ni persistimos los nombres, descripciones ni duraciones de los capítulos individuales.

---

## Gap Analysis: ¿Por qué la base está vacía/desactualizada?
La razón principal por la cual la tabla `tv_metadata_cache` se encuentra vacía o con valores `NULL` para la inmensa mayoría del catálogo obedece a tres fallas estructurales encadenadas:

1. **Diseño "On-Demand" sin bulk-fetch:** La función `fetch_and_cache_temporadas` NO se ejecuta al agregar contenido nuevo al catálogo. Solo es invocada mediante IPC (`invoke`) desde el Frontend cuando el usuario decide explícitamente abrir el modal de edición ("el lapicito") de una serie en particular.
2. **El comando de Backfill es código muerto (Dead Code):** Existe una función `backfill_metadata_cache` en Rust diseñada específicamente para poblar masivamente todos los `NULL` de la tabla iterando sobre la base. Sin embargo, este comando **no está cableado a ninguna interfaz**. No hay ningún botón en el Frontend que lo dispare, ni se ejecuta automáticamente en el startup (en `main.rs`). Jamás se ejecutó en producción.
3. **Falla técnica del On-Demand (Error de entorno):** Hasta los cambios recientes, cuando el usuario abría un modal y disparaba la búsqueda on-demand, el backend fallaba silenciosamente en runtime. Como la app se lanza vía `iniciar_app.bat`, el Current Working Directory (CWD) difería del esperado por Tauri, causando que `dotenvy` no encontrara el archivo `.env`. Al no resolver el `TMDB_API_KEY`, la llamada de Rust paniqueaba o retornaba error, el frontend atajaba la excepción, caía al "modo manual" (stepper +/-), y **el intento de guardado en base de datos nunca se completaba**. 

Como resultado, los registros históricos jamás tuvieron metadata, el proceso masivo para curarlos nunca se corrió, y los intentos individuales fallaban por un error de variables de entorno.
