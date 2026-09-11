# Auditoría de Integración API: Jikan / MyAnimeList (MAL)

## Análisis de Endpoints
Tras una revisión exhaustiva del código fuente (backend en Rust `src-tauri/src/` y frontend en React `src/`), se confirma que **actualmente no hay ninguna integración implementada con la API de Jikan o MyAnimeList en la versión 2.0 (Tauri)**.

- No existen llamadas HTTP (`reqwest` en Rust o `fetch`/`axios` en TS) apuntando a `api.jikan.moe` o `myanimelist.net`.
- No hay comandos de Tauri expuestos para resolver `mal_id` ni buscar temporadas absolutas de anime.
- El único código relacionado es a nivel de base de datos (la columna `mal_id` en la tabla `contenido` y las columnas `episodio_absoluto` y `episodio_absoluto_actual` en el tracking de progreso), pero estas columnas actúan actualmente como placeholders sin lógica de negocio que las popule o consuma desde internet.

**Conclusión:** Todo el manejo de animes recae actualmente en la misma lógica genérica de fallback (stepper manual) o en TMDB si el anime cuenta con `tmdb_id`, ya que la migración o desarrollo del feature de Jikan en la v2 aún no se ha llevado a cabo.
