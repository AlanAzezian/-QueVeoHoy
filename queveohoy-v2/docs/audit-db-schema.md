# Auditoría del Esquema de Base de Datos (SQLite)

Este es el dump real y completo del schema de SQLite actual en disco, incluyendo todas las tablas, columnas, tipos de datos, constraints, PKs, FKs e índices.

```sql
CREATE TABLE contenido (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    tmdb_id         INTEGER,                    -- id en TMDB (puede ser NULL si algún día se agrega contenido solo por MAL)
    mal_id          INTEGER,                    -- id en MyAnimeList / Jikan (NULL si no es anime o no se resolvió aún)
    tipo            TEXT NOT NULL CHECK (tipo IN ('MOVIE', 'TV')),
    es_anime        INTEGER NOT NULL DEFAULT 0, -- 0/1, derivado de género Animation + país/idioma origen
    titulo          TEXT NOT NULL,
    titulo_original TEXT,                       -- útil para anime (título en japonés/romaji)
    poster_url      TEXT,
    sinopsis        TEXT,
    fecha_estreno   TEXT,                       -- YYYY-MM-DD
    creado_en       TEXT NOT NULL DEFAULT (datetime('now')),  -- cuándo se cacheó localmente

    UNIQUE (tmdb_id, tipo)
);

CREATE INDEX idx_contenido_mal_id ON contenido (mal_id);

CREATE TABLE usuario_contenido (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    contenido_id        INTEGER NOT NULL REFERENCES contenido(id),
    estado              TEXT NOT NULL CHECK (
                            estado IN ('pendiente', 'en_progreso', 'pausada', 'abandonada', 'terminada')
                        ),
    fecha_agregado      TEXT NOT NULL DEFAULT (datetime('now')),
    fecha_inicio        TEXT,          -- cuándo pasó a en_progreso por primera vez
    fecha_finalizacion  TEXT,          -- cuándo pasó a terminada

    UNIQUE (contenido_id)  -- un solo registro de relación por contenido
);

CREATE INDEX idx_usuario_contenido_estado ON usuario_contenido (estado);

CREATE TABLE progreso_serie (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_contenido_id    INTEGER NOT NULL UNIQUE REFERENCES usuario_contenido(id),
    temporada_actual        INTEGER NOT NULL DEFAULT 1,
    episodio_actual         INTEGER NOT NULL DEFAULT 1,
    episodio_absoluto_actual INTEGER,      -- numeración absoluta (relevante para anime vía Jikan)
    actualizado_en          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE episodios_vistos (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_contenido_id    INTEGER NOT NULL REFERENCES usuario_contenido(id),
    tmdb_episode_id         INTEGER,          -- id de episodio en TMDB, si está disponible
    temporada               INTEGER NOT NULL,
    episodio                INTEGER NOT NULL,
    episodio_absoluto       INTEGER,          -- numeración absoluta MAL/Jikan (anime), NULL si no aplica
    fecha_visto             TEXT NOT NULL DEFAULT (datetime('now')),  -- timestamp completo, no solo fecha
    origen                  TEXT NOT NULL CHECK (origen IN ('APP', 'IMPORTADO')),

    UNIQUE (usuario_contenido_id, temporada, episodio)
);

CREATE INDEX idx_episodios_vistos_usuario_contenido ON episodios_vistos (usuario_contenido_id);

CREATE TABLE tv_metadata_cache (
    contenido_id    INTEGER PRIMARY KEY REFERENCES contenido(id),
    total_temporadas INTEGER NOT NULL,
    temporadas_json TEXT NOT NULL
);

CREATE TABLE historial_recomendaciones (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    contenido_id    INTEGER NOT NULL UNIQUE REFERENCES contenido(id),
    fecha           TEXT NOT NULL DEFAULT (datetime('now')),
    accion          TEXT NOT NULL CHECK (
                        accion IN ('SIGUIENTE', 'PARA_DESPUES', 'EMPEZAR_EN_PROGRESO', 'YA_LA_VI', 'RECOMENDADA_HOY', 'AJUSTE_PROGRESO')
                    ),
    detalle         TEXT,
    veces_mostrada  INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE catalogo_offline (
    contenido_id    INTEGER PRIMARY KEY REFERENCES contenido(id),
    fecha_agregado  TEXT NOT NULL DEFAULT (datetime('now'))
);
```
