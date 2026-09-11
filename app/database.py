import sqlite3
from pathlib import Path

DB_FILE = Path.home() / '.queveohoy.db'

def get_connection() -> sqlite3.Connection:
    conn = sqlite3.connect(str(DB_FILE), timeout=10.0)
    conn.execute("PRAGMA journal_mode=WAL;")
    conn.execute("PRAGMA foreign_keys = ON;")
    conn.row_factory = sqlite3.Row
    return conn

def init_db():
    conn = get_connection()
    cursor = conn.cursor()

    cursor.executescript('''
-- =========================================================
-- 1. CONTENIDO — datos provenientes de TMDB / Jikan
-- =========================================================
CREATE TABLE IF NOT EXISTS contenido (
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

CREATE INDEX IF NOT EXISTS idx_contenido_mal_id ON contenido (mal_id);


-- =========================================================
-- 2. USUARIO_CONTENIDO — relación del usuario con ese contenido
-- =========================================================
CREATE TABLE IF NOT EXISTS usuario_contenido (
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

CREATE INDEX IF NOT EXISTS idx_usuario_contenido_estado ON usuario_contenido (estado);


-- =========================================================
-- 3. PROGRESO_SERIE — CACHÉ, solo para tipo TV
--    Se regenera siempre a partir de episodios_vistos.
-- =========================================================
CREATE TABLE IF NOT EXISTS progreso_serie (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_contenido_id    INTEGER NOT NULL UNIQUE REFERENCES usuario_contenido(id),
    temporada_actual        INTEGER NOT NULL DEFAULT 1,
    episodio_actual         INTEGER NOT NULL DEFAULT 1,
    episodio_absoluto_actual INTEGER,      -- numeración absoluta (relevante para anime vía Jikan)
    actualizado_en          TEXT NOT NULL DEFAULT (datetime('now'))
);


-- =========================================================
-- 4. EPISODIOS_VISTOS — FUENTE ÚNICA DE VERDAD
-- =========================================================
CREATE TABLE IF NOT EXISTS episodios_vistos (
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

CREATE INDEX IF NOT EXISTS idx_episodios_vistos_usuario_contenido ON episodios_vistos (usuario_contenido_id);


-- =========================================================
-- 5. HISTORIAL_RECOMENDACIONES — deduplicado, 1 fila por contenido
-- =========================================================
CREATE TABLE IF NOT EXISTS historial_recomendaciones (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    contenido_id    INTEGER NOT NULL UNIQUE REFERENCES contenido(id),
    fecha           TEXT NOT NULL DEFAULT (datetime('now')),
    accion          TEXT NOT NULL CHECK (
                        accion IN ('SIGUIENTE', 'PARA_DESPUES', 'EMPEZAR_EN_PROGRESO', 'YA_LA_VI', 'RECOMENDADA_HOY', 'AJUSTE_PROGRESO')
                    ),
    detalle         TEXT,
    veces_mostrada  INTEGER NOT NULL DEFAULT 1
);
-- =========================================================
-- 6. TV_METADATA_CACHE — caché de temporadas de TMDB/Jikan
-- =========================================================
CREATE TABLE IF NOT EXISTS tv_metadata_cache (
    contenido_id    INTEGER PRIMARY KEY REFERENCES contenido(id),
    total_temporadas INTEGER NOT NULL,
    temporadas_json TEXT NOT NULL
);

-- =========================================================
-- 7. CATALOGO_OFFLINE — pool rotativo offline (max ~150 ítems)
-- =========================================================
CREATE TABLE IF NOT EXISTS catalogo_offline (
    contenido_id    INTEGER PRIMARY KEY REFERENCES contenido(id),
    fecha_agregado  TEXT NOT NULL DEFAULT (datetime('now'))
);
    ''')

    # Saneamiento de datos de Anime
    cursor.execute('''
        UPDATE contenido 
        SET es_anime = 1 
        WHERE mal_id IS NOT NULL 
           OR LOWER(titulo) LIKE '%kimetsu%' 
           OR LOWER(titulo) LIKE '%chainsaw%' 
           OR LOWER(titulo) LIKE '%jujutsu%' 
           OR LOWER(titulo) LIKE '%animatrix%' 
           OR LOWER(titulo) LIKE '%naruto%' 
           OR LOWER(titulo) LIKE '%hero academia%';
    ''')

    # Migración de historial_recomendaciones
    try:
        # Check if detalle column exists, if it fails, it means we need to migrate
        cursor.execute("SELECT detalle FROM historial_recomendaciones LIMIT 1")
    except sqlite3.OperationalError:
        print("Realizando migración de historial_recomendaciones...")
        cursor.executescript('''
            PRAGMA foreign_keys=off;
            
            ALTER TABLE historial_recomendaciones RENAME TO historial_recomendaciones_old;
            
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
            
            INSERT INTO historial_recomendaciones (id, contenido_id, fecha, accion, veces_mostrada, detalle)
            SELECT id, contenido_id, fecha, accion, veces_mostrada, NULL
            FROM historial_recomendaciones_old;
            
            DROP TABLE historial_recomendaciones_old;
            
            PRAGMA foreign_keys=on;
        ''')

    conn.commit()
    conn.close()

if __name__ == '__main__':
    init_db()
