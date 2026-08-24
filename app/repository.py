from typing import Optional
from .database import get_connection
from .models import (
    Contenido, UsuarioContenido, ProgresoSerie,
    EpisodioVisto, HistorialRecomendacion
)

def insertar_contenido(contenido: Contenido) -> int:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO contenido (
            tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original,
            poster_url, sinopsis, fecha_estreno
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    ''', (
        contenido.tmdb_id, contenido.mal_id, contenido.tipo,
        1 if contenido.es_anime else 0, contenido.titulo, contenido.titulo_original,
        contenido.poster_url, contenido.sinopsis, contenido.fecha_estreno
    ))
    row_id = cursor.lastrowid
    conn.commit()
    conn.close()
    return row_id

def obtener_contenido_por_tmdb_id(tmdb_id: int, tipo: str) -> Optional[Contenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original,
               poster_url, sinopsis, fecha_estreno, creado_en
        FROM contenido
        WHERE tmdb_id = ? AND tipo = ?
    ''', (tmdb_id, tipo))
    row = cursor.fetchone()
    conn.close()

    if row:
        return Contenido(
            id=row['id'],
            tmdb_id=row['tmdb_id'],
            mal_id=row['mal_id'],
            tipo=row['tipo'],
            es_anime=bool(row['es_anime']),
            titulo=row['titulo'],
            titulo_original=row['titulo_original'],
            poster_url=row['poster_url'],
            sinopsis=row['sinopsis'],
            fecha_estreno=row['fecha_estreno'],
            creado_en=row['creado_en']
        )
    return None

def obtener_contenido_por_mal_id(mal_id: int) -> Optional[Contenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original,
               poster_url, sinopsis, fecha_estreno, creado_en
        FROM contenido
        WHERE mal_id = ?
    ''', (mal_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return Contenido(
            id=row['id'],
            tmdb_id=row['tmdb_id'],
            mal_id=row['mal_id'],
            tipo=row['tipo'],
            es_anime=bool(row['es_anime']),
            titulo=row['titulo'],
            titulo_original=row['titulo_original'],
            poster_url=row['poster_url'],
            sinopsis=row['sinopsis'],
            fecha_estreno=row['fecha_estreno'],
            creado_en=row['creado_en']
        )
    return None

def insertar_usuario_contenido(usuario_contenido: UsuarioContenido) -> int:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO usuario_contenido (
            contenido_id, estado, fecha_inicio, fecha_finalizacion
        ) VALUES (?, ?, ?, ?)
    ''', (
        usuario_contenido.contenido_id, usuario_contenido.estado,
        usuario_contenido.fecha_inicio, usuario_contenido.fecha_finalizacion
    ))
    row_id = cursor.lastrowid
    conn.commit()
    conn.close()
    return row_id

def actualizar_estado_usuario_contenido(usuario_contenido_id: int, nuevo_estado: str) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        UPDATE usuario_contenido
        SET estado = ?
        WHERE id = ?
    ''', (nuevo_estado, usuario_contenido_id))
    conn.commit()
    conn.close()

def insertar_episodio_visto(episodio: EpisodioVisto) -> int:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO episodios_vistos (
            usuario_contenido_id, tmdb_episode_id, temporada, episodio,
            episodio_absoluto, origen
        ) VALUES (?, ?, ?, ?, ?, ?)
    ''', (
        episodio.usuario_contenido_id, episodio.tmdb_episode_id,
        episodio.temporada, episodio.episodio,
        episodio.episodio_absoluto, episodio.origen
    ))
    row_id = cursor.lastrowid
    conn.commit()
    conn.close()
    return row_id

def obtener_ultimo_episodio_visto(usuario_contenido_id: int) -> Optional[EpisodioVisto]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, usuario_contenido_id, tmdb_episode_id, temporada, episodio,
               episodio_absoluto, fecha_visto, origen
        FROM episodios_vistos
        WHERE usuario_contenido_id = ?
        ORDER BY fecha_visto DESC, temporada DESC, episodio DESC
        LIMIT 1
    ''', (usuario_contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return EpisodioVisto(
            id=row['id'],
            usuario_contenido_id=row['usuario_contenido_id'],
            tmdb_episode_id=row['tmdb_episode_id'],
            temporada=row['temporada'],
            episodio=row['episodio'],
            episodio_absoluto=row['episodio_absoluto'],
            fecha_visto=row['fecha_visto'],
            origen=row['origen']
        )
    return None

def upsert_progreso_serie(progreso: ProgresoSerie) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO progreso_serie (
            usuario_contenido_id, temporada_actual, episodio_actual,
            episodio_absoluto_actual, actualizado_en
        ) VALUES (?, ?, ?, ?, datetime('now'))
        ON CONFLICT(usuario_contenido_id) DO UPDATE SET
            temporada_actual = excluded.temporada_actual,
            episodio_actual = excluded.episodio_actual,
            episodio_absoluto_actual = excluded.episodio_absoluto_actual,
            actualizado_en = datetime('now')
    ''', (
        progreso.usuario_contenido_id, progreso.temporada_actual,
        progreso.episodio_actual, progreso.episodio_absoluto_actual
    ))
    conn.commit()
    conn.close()

def upsert_historial_recomendacion(contenido_id: int, accion: str, detalle: str = None) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO historial_recomendaciones (
            contenido_id, fecha, accion, veces_mostrada, detalle
        ) VALUES (?, datetime('now'), ?, 1, ?)
        ON CONFLICT(contenido_id) DO UPDATE SET
            fecha = excluded.fecha,
            accion = excluded.accion,
            veces_mostrada = veces_mostrada + 1,
            detalle = excluded.detalle
    ''', (contenido_id, accion, detalle))
    conn.commit()
    conn.close()

def obtener_usuario_contenido_por_id(usuario_contenido_id: int) -> Optional[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, contenido_id, estado, fecha_agregado, fecha_inicio, fecha_finalizacion
        FROM usuario_contenido
        WHERE id = ?
    ''', (usuario_contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        )
    return None

def actualizar_usuario_contenido(usuario_contenido: UsuarioContenido) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        UPDATE usuario_contenido
        SET estado = ?,
            fecha_inicio = ?,
            fecha_finalizacion = ?
        WHERE id = ?
    ''', (
        usuario_contenido.estado,
        usuario_contenido.fecha_inicio,
        usuario_contenido.fecha_finalizacion,
        usuario_contenido.id
    ))
    conn.commit()
    conn.close()

def obtener_contenido_por_id(contenido_id: int) -> Optional[Contenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original,
               poster_url, sinopsis, fecha_estreno, creado_en
        FROM contenido
        WHERE id = ?
    ''', (contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return Contenido(
            id=row['id'],
            tmdb_id=row['tmdb_id'],
            mal_id=row['mal_id'],
            tipo=row['tipo'],
            es_anime=bool(row['es_anime']),
            titulo=row['titulo'],
            titulo_original=row['titulo_original'],
            poster_url=row['poster_url'],
            sinopsis=row['sinopsis'],
            fecha_estreno=row['fecha_estreno'],
            creado_en=row['creado_en']
        )
    return None

def obtener_progreso_serie(usuario_contenido_id: int) -> Optional[ProgresoSerie]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, usuario_contenido_id, temporada_actual, episodio_actual,
               episodio_absoluto_actual, actualizado_en
        FROM progreso_serie
        WHERE usuario_contenido_id = ?
    ''', (usuario_contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return ProgresoSerie(
            id=row['id'],
            usuario_contenido_id=row['usuario_contenido_id'],
            temporada_actual=row['temporada_actual'],
            episodio_actual=row['episodio_actual'],
            episodio_absoluto_actual=row['episodio_absoluto_actual'],
            actualizado_en=row['actualizado_en']
        )
    return None

def obtener_historial_recomendaciones(limit: int = 50) -> list[HistorialRecomendacion]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, contenido_id, fecha, accion, veces_mostrada
        FROM historial_recomendaciones
        WHERE accion != 'RECOMENDADA_HOY'
        ORDER BY fecha DESC
        LIMIT ?
    ''', (limit,))
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(HistorialRecomendacion(
            id=row['id'],
            contenido_id=row['contenido_id'],
            fecha=row['fecha'],
            accion=row['accion'],
            veces_mostrada=row['veces_mostrada']
        ))
    return res

def obtener_recomendacion_activa_hoy() -> Optional[Contenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT c.id, c.tmdb_id, c.mal_id, c.tipo, c.es_anime, c.titulo, c.titulo_original,
               c.poster_url, c.sinopsis, c.fecha_estreno, c.creado_en
        FROM historial_recomendaciones hr
        JOIN contenido c ON hr.contenido_id = c.id
        WHERE hr.accion = 'RECOMENDADA_HOY'
          AND date(hr.fecha) = date('now', 'localtime')
        ORDER BY hr.fecha DESC
        LIMIT 1
    ''')
    row = cursor.fetchone()
    conn.close()

    if row:
        return Contenido(
            id=row['id'],
            tmdb_id=row['tmdb_id'],
            mal_id=row['mal_id'],
            tipo=row['tipo'],
            es_anime=bool(row['es_anime']),
            titulo=row['titulo'],
            titulo_original=row['titulo_original'],
            poster_url=row['poster_url'],
            sinopsis=row['sinopsis'],
            fecha_estreno=row['fecha_estreno'],
            creado_en=row['creado_en']
        )
    return None

def obtener_peliculas_vistas() -> list[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT uc.id, uc.contenido_id, uc.estado, uc.fecha_agregado, uc.fecha_inicio, uc.fecha_finalizacion
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE c.tipo = 'MOVIE' AND uc.estado = 'terminada'
        ORDER BY uc.fecha_finalizacion DESC
    ''')
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        ))
    return res

def obtener_usuario_contenido_ids_con_episodios_vistos() -> list[int]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT DISTINCT usuario_contenido_id
        FROM episodios_vistos
    ''')
    rows = cursor.fetchall()
    conn.close()
    return [row['usuario_contenido_id'] for row in rows]

def obtener_episodios_vistos_por_usuario_contenido(usuario_contenido_id: int) -> list[EpisodioVisto]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, usuario_contenido_id, tmdb_episode_id, temporada, episodio,
               episodio_absoluto, fecha_visto, origen
        FROM episodios_vistos
        WHERE usuario_contenido_id = ?
        ORDER BY fecha_visto DESC, temporada DESC, episodio DESC
    ''', (usuario_contenido_id,))
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(EpisodioVisto(
            id=row['id'],
            usuario_contenido_id=row['usuario_contenido_id'],
            tmdb_episode_id=row['tmdb_episode_id'],
            temporada=row['temporada'],
            episodio=row['episodio'],
            episodio_absoluto=row['episodio_absoluto'],
            fecha_visto=row['fecha_visto'],
            origen=row['origen']
        ))
    return res

def buscar_contenido_por_titulo(query: str) -> list[Contenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, tmdb_id, mal_id, tipo, es_anime, titulo, titulo_original,
               poster_url, sinopsis, fecha_estreno, creado_en
        FROM contenido
        WHERE titulo LIKE '%' || ? || '%' COLLATE NOCASE
    ''', (query,))
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(Contenido(
            id=row['id'],
            tmdb_id=row['tmdb_id'],
            mal_id=row['mal_id'],
            tipo=row['tipo'],
            es_anime=bool(row['es_anime']),
            titulo=row['titulo'],
            titulo_original=row['titulo_original'],
            poster_url=row['poster_url'],
            sinopsis=row['sinopsis'],
            fecha_estreno=row['fecha_estreno'],
            creado_en=row['creado_en']
        ))
    return res

def obtener_usuario_contenido_por_contenido_id(contenido_id: int) -> Optional[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, contenido_id, estado, fecha_agregado, fecha_inicio, fecha_finalizacion
        FROM usuario_contenido
        WHERE contenido_id = ?
    ''', (contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        )
    return None

def obtener_historial_recomendacion_por_contenido_id(contenido_id: int) -> Optional[HistorialRecomendacion]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT id, contenido_id, fecha, accion, veces_mostrada
        FROM historial_recomendaciones
        WHERE contenido_id = ?
    ''', (contenido_id,))
    row = cursor.fetchone()
    conn.close()

    if row:
        return HistorialRecomendacion(
            id=row['id'],
            contenido_id=row['contenido_id'],
            fecha=row['fecha'],
            accion=row['accion'],
            veces_mostrada=row['veces_mostrada']
        )
    return None

def obtener_usuario_contenido_en_progreso_tv() -> list[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT uc.id, uc.contenido_id, uc.estado, uc.fecha_agregado, uc.fecha_inicio, uc.fecha_finalizacion
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE uc.estado = 'en_progreso' AND c.tipo = 'TV'
    ''')
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        ))
    return res

def obtener_usuario_contenido_en_progreso_y_pausadas_tv() -> list[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT uc.id, uc.contenido_id, uc.estado, uc.fecha_agregado, uc.fecha_inicio, uc.fecha_finalizacion
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE uc.estado IN ('en_progreso', 'pausada') AND c.tipo = 'TV'
    ''')
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        ))
    return res

def obtener_biblioteca_inactiva() -> list[UsuarioContenido]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT uc.id, uc.contenido_id, uc.estado, uc.fecha_agregado, uc.fecha_inicio, uc.fecha_finalizacion
        FROM usuario_contenido uc
        LEFT JOIN contenido c ON uc.contenido_id = c.id
        WHERE uc.estado IN ('pendiente', 'abandonada', 'terminada', 'vista')
        ORDER BY uc.fecha_agregado DESC
    ''')
    rows = cursor.fetchall()
    conn.close()

    res = []
    for row in rows:
        res.append(UsuarioContenido(
            id=row['id'],
            contenido_id=row['contenido_id'],
            estado=row['estado'],
            fecha_agregado=row['fecha_agregado'],
            fecha_inicio=row['fecha_inicio'],
            fecha_finalizacion=row['fecha_finalizacion']
        ))
    return res

def obtener_tv_metadata(contenido_id: int) -> Optional[dict]:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        SELECT total_temporadas, temporadas_json
        FROM tv_metadata_cache
        WHERE contenido_id = ?
    ''', (contenido_id,))
    row = cursor.fetchone()
    conn.close()
    if row:
        return {
            'total_temporadas': row['total_temporadas'],
            'temporadas_json': row['temporadas_json']
        }
    return None

def upsert_tv_metadata(contenido_id: int, total_temporadas: int, temporadas_json: str) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        INSERT INTO tv_metadata_cache (contenido_id, total_temporadas, temporadas_json)
        VALUES (?, ?, ?)
        ON CONFLICT(contenido_id) DO UPDATE SET
            total_temporadas = excluded.total_temporadas,
            temporadas_json = excluded.temporadas_json
    ''', (contenido_id, total_temporadas, temporadas_json))
    conn.commit()
    conn.close()

def eliminar_usuario_contenido(usuario_contenido_id: int) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('DELETE FROM episodios_vistos WHERE usuario_contenido_id = ?', (usuario_contenido_id,))
    cursor.execute('DELETE FROM progreso_serie WHERE usuario_contenido_id = ?', (usuario_contenido_id,))
    cursor.execute('DELETE FROM usuario_contenido WHERE id = ?', (usuario_contenido_id,))
    conn.commit()
    conn.close()

def eliminar_episodios_vistos_posteriores(usuario_contenido_id: int, temporada: int, episodio: int, episodio_absoluto: Optional[int] = None) -> None:
    conn = get_connection()
    cursor = conn.cursor()
    cursor.execute('''
        DELETE FROM episodios_vistos
        WHERE usuario_contenido_id = ? AND (
            temporada > ? OR 
            (temporada = ? AND episodio > ?)
        )
    ''', (usuario_contenido_id, temporada, temporada, episodio))
    if episodio_absoluto is not None:
        cursor.execute('''
            DELETE FROM episodios_vistos
            WHERE usuario_contenido_id = ? AND episodio_absoluto > ?
        ''', (usuario_contenido_id, episodio_absoluto))
    conn.commit()
    conn.close()

def obtener_metricas_biblioteca() -> dict:
    """Retorna desglose de contenidos completados discriminando animes"""
    conn = get_connection()
    cursor = conn.cursor()
    
    cursor.execute('''
        SELECT COUNT(*) as count
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE LOWER(uc.estado) IN ('terminada', 'terminado', 'vista', 'visto')
          AND UPPER(c.tipo) = 'MOVIE' 
          AND (c.es_anime = 0 OR c.es_anime IS NULL)
    ''')
    pelis = cursor.fetchone()['count']
    
    cursor.execute('''
        SELECT COUNT(*) as count
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE LOWER(uc.estado) IN ('terminada', 'terminado', 'vista', 'visto')
          AND UPPER(c.tipo) = 'TV' 
          AND (c.es_anime = 0 OR c.es_anime IS NULL)
    ''')
    series = cursor.fetchone()['count']
    
    cursor.execute('''
        SELECT COUNT(*) as count
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE LOWER(uc.estado) IN ('terminada', 'terminado', 'vista', 'visto')
          AND UPPER(c.tipo) = 'TV'
          AND (c.es_anime = 1 OR c.mal_id IS NOT NULL)
    ''')
    anime_series = cursor.fetchone()['count']
    
    cursor.execute('''
        SELECT COUNT(*) as count
        FROM usuario_contenido uc
        JOIN contenido c ON uc.contenido_id = c.id
        WHERE LOWER(uc.estado) IN ('terminada', 'terminado', 'vista', 'visto')
          AND UPPER(c.tipo) = 'MOVIE'
          AND (c.es_anime = 1 OR c.mal_id IS NOT NULL)
    ''')
    anime_pelis = cursor.fetchone()['count']
    
    conn.close()
    return {
        "pelis": pelis,
        "series": series,
        "anime_series": anime_series,
        "anime_pelis": anime_pelis,
        "total": pelis + series + anime_series + anime_pelis
    }

def obtener_estado_en_biblioteca(item: Contenido) -> Optional[str]:
    """Retorna el estado del contenido en la biblioteca local, o None si no existe."""
    c_db = None
    if item.mal_id is not None:
        c_db = obtener_contenido_por_mal_id(item.mal_id)
    elif item.tmdb_id is not None:
        c_db = obtener_contenido_por_tmdb_id(item.tmdb_id, item.tipo)
    else:
        conn = get_connection()
        cursor = conn.cursor()
        cursor.execute('''
            SELECT id FROM contenido 
            WHERE titulo = ? AND tipo = ? COLLATE NOCASE
        ''', (item.titulo, item.tipo))
        row = cursor.fetchone()
        conn.close()
        if row:
            c_db = obtener_contenido_por_id(row['id'])
            
    if c_db:
        uc = obtener_usuario_contenido_por_contenido_id(c_db.id)
        if uc:
            return uc.estado
    return None


