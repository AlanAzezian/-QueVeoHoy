# Propuesta de Rediseño de Schema (Metadatos de TV/Anime)

## 1. Evaluación Arquitectónica: Relacional vs. Sobre-ingeniería

Actualmente, QuéVeoHoy 2.0 solo utiliza la metadata para dos cosas:
1. Renderizar los "chips" de temporadas (Ej: Temporada 1, Temporada 2).
2. Renderizar la grilla de capítulos (1 al 12) y la barra de progreso (% visto).

**¿Vale la pena guardar nombres y sinopsis de cada episodio?**
Si el objetivo a corto/mediano plazo sigue siendo solo "trackear el progreso" a través de steppers o grillas numéricas, **guardar cada episodio individual con su sinopsis es sobre-ingeniería**. Implicaría:
- Multiplicar las llamadas a la API de TMDB (1 request por temporada en lugar de 1 request por serie).
- Mayor lentitud en el fetch inicial.
- Mayor peso en la base de datos para texto que el usuario no va a leer.

**Sin embargo**, si a futuro planeás que al hacer hover en el capítulo "4" de la grilla aparezca un tooltip con el nombre del episodio ("El juicio de Sandman"), o querés mostrar una lista con las fechas de estreno de los próximos capítulos en emisión, el modelo relacional es la única salida limpia.

A continuación, presento el diseño **Relacional Completo**, que nos prepara para un tracker de primer nivel, soportando tanto TMDB (occidental) como Jikan (anime absoluto).

---

## 2. Propuesta de Schema (Tablas Nuevas)

Reemplazaríamos (o complementaríamos) la tabla monolítica `tv_metadata_cache` con dos tablas altamente normalizadas:

### Tabla `tv_temporadas`
```sql
CREATE TABLE tv_temporadas (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    contenido_id    INTEGER NOT NULL REFERENCES contenido(id) ON DELETE CASCADE,
    season_number   INTEGER NOT NULL,       -- Ej: 1, 2, 3
    name            TEXT,                   -- Ej: "Arco de la Sociedad de Almas"
    overview        TEXT,
    air_date        TEXT,
    
    UNIQUE(contenido_id, season_number)
);
```

### Tabla `tv_episodios`
```sql
CREATE TABLE tv_episodios (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id        INTEGER NOT NULL REFERENCES tv_temporadas(id) ON DELETE CASCADE,
    episode_number      INTEGER NOT NULL,   -- Numeración relativa (1 al 12/24) (TMDB)
    episode_absolute    INTEGER,            -- Numeración absoluta (1 al 500) (Jikan/MAL)
    name                TEXT,
    overview            TEXT,
    runtime             INTEGER,            -- En minutos
    air_date            TEXT,
    
    UNIQUE(temporada_id, episode_number)
);
CREATE INDEX idx_episodios_absolute ON tv_episodios(episode_absolute);
```

---

## 3. Integración TMDB vs Jikan (MAL)

El gran problema del Anime es que TMDB agrupa por temporadas (Ej: *Bleach T1, T2, T3* con capítulos del 1 al 20 cada una) mientras que la comunidad (MAL/Jikan) maneja numeración absoluta (Ej: *Bleach Capítulos 1 al 366*).

**¿Cómo lo resolvemos en este Schema?**
1. **Para TMDB (TV Occidental):**
   - Llenamos `season_number` y `episode_number`.
   - `episode_absolute` queda en `NULL`.
2. **Para Jikan (Anime):**
   - Agrupamos todo bajo un único registro en `tv_temporadas` con `season_number = 1` (Temporada Única Absoluta).
   - En `tv_episodios`, llenamos `episode_number` igual a `episode_absolute` (1 al 366).
   - O bien, si tenemos un mapping manual, llenamos ambos: `episode_number` (el 1 de la temporada 2) y `episode_absolute` (el 25 general).

Nuestra tabla `progreso_serie` **ya está preparada para esto** (posee `episodio_actual` y `episodio_absoluto_actual`).

---

## 4. Borrador del Script de Migración (NO EJECUTAR)

Este script:
1. Crea las nuevas tablas relacionales.
2. (Opcional) Dropea la vieja tabla de caché si decidimos abandonar el formato JSON.

```sql
-- 1. Crear tabla de temporadas
CREATE TABLE IF NOT EXISTS tv_temporadas (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    contenido_id    INTEGER NOT NULL,
    season_number   INTEGER NOT NULL,
    name            TEXT,
    overview        TEXT,
    air_date        TEXT,
    FOREIGN KEY(contenido_id) REFERENCES contenido(id) ON DELETE CASCADE,
    UNIQUE(contenido_id, season_number)
);

CREATE INDEX idx_tv_temporadas_contenido ON tv_temporadas(contenido_id);

-- 2. Crear tabla de episodios
CREATE TABLE IF NOT EXISTS tv_episodios (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    temporada_id        INTEGER NOT NULL,
    episode_number      INTEGER NOT NULL,
    episode_absolute    INTEGER,
    name                TEXT,
    overview            TEXT,
    runtime             INTEGER,
    air_date            TEXT,
    FOREIGN KEY(temporada_id) REFERENCES tv_temporadas(id) ON DELETE CASCADE,
    UNIQUE(temporada_id, episode_number)
);

CREATE INDEX idx_tv_episodios_temporada ON tv_episodios(temporada_id);

-- 3. (OPCIONAL) Descartar la vieja caché monolítica si se migran los datos con éxito
-- DROP TABLE tv_metadata_cache;
```
