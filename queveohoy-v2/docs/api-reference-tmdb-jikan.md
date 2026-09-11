# Documentación de Referencia APIs: TMDB y Jikan (MyAnimeList)

---

## 1. TMDB (The Movie Database)

### Autenticación
TMDB soporta dos métodos de autenticación principal:
1. **API Key (Query Parameter):** Se envía como `?api_key=TU_KEY` en cada request. Ideal para aplicaciones server-to-server o clientes de solo lectura.
2. **Access Token (Bearer):** Un JWT enviado en el header `Authorization: Bearer TU_TOKEN`. Se usa habitualmente para autenticar a un usuario específico y hacer acciones de escritura (v4) o como alternativa oficial más segura en v3.
* **Aplicado a QuéVeoHoy:** El uso de **API Key** es perfectamente válido y suficiente, ya que solo hacemos consultas públicas de lectura (Read-Only) de la base de datos de TMDB. 

### Endpoints Clave para TV
1. **Detalle de Serie (`/tv/{series_id}`):**
   - Devuelve información general (título, sinopsis general, géneros, estado) y un array `seasons` que **sólo** tiene un resumen de cada temporada (id, `season_number`, `episode_count`).
   - *NO* trae la lista de episodios ni sus nombres individuales.

2. **Detalle de Temporada (`/tv/{series_id}/season/{season_number}`):**
   - Este es el endpoint vital que nos falta en la app hoy. Devuelve información de una temporada específica y, lo más importante, un array `episodes` con el detalle completo de cada capítulo: `episode_number`, `name`, `overview` (sinopsis), `air_date` y `runtime`.

3. **Detalle de Episodio (`/tv/{series_id}/season/{season_number}/episode/{episode_number}`):**
   - Sirve para consultar un único episodio. Generalmente no lo necesitamos si ya llamamos al de la temporada, a menos que queramos actualizar datos de un solo capítulo en particular.

### Rate Limits y Throttling
TMDB actualizó sus límites recientemente y ahora es extremadamente permisivo:
- **50 requests por segundo (50 req/s)**.
- Anteriormente eran 40 reqs por 10 segundos, pero eso ya no aplica. 
- Si se excede el límite (muy difícil en una app personal), la API retorna HTTP `429 Too Many Requests`.

### Formato de Errores
TMDB retorna un JSON con un formato predecible en caso de fallo (401 Unauthorized, 404 Not Found, etc.):
```json
{
  "success": false,
  "status_code": 34,
  "status_message": "The resource you requested could not be found."
}
```

---

## 2. Jikan (MyAnimeList Unofficial API)

### Resolución de Anime y Endpoints
1. **Buscar Anime (`/anime?q={titulo}`):** Para buscar un anime por texto y obtener su `mal_id`.
2. **Detalle de Anime (`/anime/{mal_id}`):** Equivale a `/tv/{id}` de TMDB.
3. **Episodios del Anime (`/anime/{mal_id}/episodes`):**
   - Jikan/MAL **no** divide los animes en "Temporadas 1, 2, 3" dentro del mismo ID de la misma forma que TMDB (occidental). MAL suele crear un `mal_id` enteramente nuevo para cada temporada (ej. "Attack on Titan" y "Attack on Titan Season 2" son IDs distintos).
   - Por esto, este endpoint trae los episodios con **numeración absoluta** (1 al 100+).
   - Devuelve un paginado con `mal_id` del episodio, `title` (nombre) y si es filler o canon, pero **no** trae la sinopsis aquí. Para la sinopsis hay que llamar a `/anime/{id}/episodes/{episode_id}`.

### Rate Limits (¡Crítico!)
Jikan es provisto por una comunidad y sus servidores son muy estrictos comparados con TMDB:
- **3 requests por segundo (3 req/s)**.
- **60 requests por minuto (60 req/min)**.
Si se excede, retorna `429 Too Many Requests`. Es mandatorio implementar un mecanismo de backoff (pausas) y delays en cualquier script masivo o de backfill que consuma Jikan.

### Formato de Errores
Jikan retorna errores con la siguiente estructura JSON:
```json
{
  "status": 404,
  "type": "BadResponseException",
  "message": "Resource not found",
  "error": "Not Found"
}
```

---

## 3. Aplicado a QuéVeoHoy (Flujo Óptimo)

Para poblar y mostrar correctamente la información rica (nombres y sinopsis de episodios) en la aplicación, el orden de requests a nivel backend debería ser el siguiente:

### Para Series (Vía TMDB)
1. Llamar a `/tv/{id}` para obtener la lista de `season_number`.
2. Por cada `season_number` mayor a 0, hacer un request iterativo a `/tv/{id}/season/{season_number}`.
3. Extraer el array `episodes` de cada respuesta.
4. Persistir los nombres y sinopsis en la BD local.
*(Dada la cuota de 50 req/s de TMDB, podemos hacer estos requests de temporadas en paralelo sin riesgo de rate limit).*

### Para Animes (Vía Jikan)
1. Para un `mal_id` dado, llamar a `/anime/{mal_id}/episodes`.
2. Extraer los títulos y números absolutos. Almacenarlos asociados a una "Temporada 1" lógica (o a la temporada correspondiente si implementamos un mapping manual TMDB <-> MAL) en nuestra base de datos.
*(Aquí la concurrencia debe limitarse estrictamente para respetar los 3 req/s).*
