import random
from dataclasses import dataclass
from datetime import datetime
from typing import Optional

from app.providers.tmdb import TMDBProvider
from app.providers.jikan import JikanProvider
from typing import Optional
from .models import (
    Contenido, UsuarioContenido, ProgresoSerie, EpisodioVisto,
    ESTADO_PARA_DESPUES as PENDIENTE,
    ESTADO_EN_PROGRESO as EN_PROGRESO,
    ESTADO_PAUSADA as PAUSADA,
    ESTADO_ABANDONADA as ABANDONADA,
    ESTADO_TERMINADA as TERMINADA
)
from . import repository

@dataclass
class HistorialRecomendacionItem:
    contenido_id: int
    titulo: str
    tipo: str
    fecha: str
    accion: str
    veces_mostrada: int

@dataclass
class PeliculaVista:
    contenido_id: int
    titulo: str
    fecha_finalizacion: str

@dataclass
class SerieVista:
    usuario_contenido_id: int
    contenido_id: int
    titulo: str
    episodios: list[EpisodioVisto]  # ya ordenados de más reciente a más antiguo

@dataclass
class ResultadoBusqueda:
    contenido_id: int
    titulo: str
    tipo: str
    estado_actual: str | None       # None si nunca se agregó a la biblioteca
    usuario_contenido_id: int | None
    ultima_aparicion: str | None    # fecha de la última vez que apareció en HOY, si aplica

@dataclass
class SerieEnProgreso:
    usuario_contenido_id: int
    contenido_id: int
    titulo: str
    tipo: str
    es_anime: bool
    temporada_actual: int
    episodio_actual: int
    estado: str

@dataclass
class ElementoBiblioteca:
    usuario_contenido: UsuarioContenido
    contenido: Contenido

MAX_SERIES_EN_PROGRESO = 4

class LimiteSeriesEnProgresoAlcanzado(Exception):
    """Se lanza al intentar poner una quinta serie en EN_PROGRESO."""
    pass



TRANSICIONES_VALIDAS = {
    PENDIENTE:   {EN_PROGRESO, TERMINADA},   # TERMINADA directo = "Ya la vi" en películas
    EN_PROGRESO: {PAUSADA, ABANDONADA, TERMINADA},
    PAUSADA:     {EN_PROGRESO, ABANDONADA},
    ABANDONADA:  {EN_PROGRESO},              # permite reanudar manualmente algo abandonado
    TERMINADA:   set(),                      # estado final, no se puede salir automáticamente
}

class TransicionEstadoInvalida(Exception):
    """Se lanza cuando se intenta un cambio de estado no permitido por TRANSICIONES_VALIDAS."""
    pass

def cambiar_estado(usuario_contenido_id: int, nuevo_estado: str) -> UsuarioContenido:
    """
    Cambia el estado de un usuario_contenido, validando que la transición sea legal
    según TRANSICIONES_VALIDAS. Actualiza automáticamente fecha_inicio (primera vez
    que entra a en_progreso) y fecha_finalizacion (al llegar a terminada).

    Lanza TransicionEstadoInvalida si la transición no está permitida.
    Lanza ValueError si no existe el usuario_contenido_id.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    estado_actual = usuario_contenido.estado
    
    if nuevo_estado not in TRANSICIONES_VALIDAS.get(estado_actual, set()):
        raise TransicionEstadoInvalida(
            f"Transición no permitida de '{estado_actual}' a '{nuevo_estado}'."
        )
        
    if nuevo_estado == EN_PROGRESO and usuario_contenido.fecha_inicio is None:
        usuario_contenido.fecha_inicio = datetime.now().isoformat()
        
    if nuevo_estado == TERMINADA:
        usuario_contenido.fecha_finalizacion = datetime.now().isoformat()
        
    usuario_contenido.estado = nuevo_estado
    repository.actualizar_usuario_contenido(usuario_contenido)
    
    return usuario_contenido

def marcar_vista_pelicula(usuario_contenido_id: int) -> UsuarioContenido:
    """
    Marca una película como vista ("Ya la vi").
    Transiciona PENDIENTE -> TERMINADA usando cambiar_estado() (que ya valida
    que la transición sea legal) y registra la acción en historial_recomendaciones.

    Lanza ValueError si el usuario_contenido no existe o si el contenido asociado
    no es de tipo MOVIE (las series usan un flujo distinto, ver Fase 3/4).
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "MOVIE":
        raise ValueError("marcar_vista_pelicula es solo para películas")
        
    usuario_contenido_actualizado = cambiar_estado(usuario_contenido_id, TERMINADA)
    repository.upsert_historial_recomendacion(contenido.id, "YA_LA_VI")
    
    return usuario_contenido_actualizado

def marcar_serie_terminada(usuario_contenido_id: int) -> tuple[ProgresoSerie, UsuarioContenido]:
    """
    Marca una serie como terminada ("Ya la terminé").
    Avanza la serie hasta la última temporada y último capítulo, y transiciona a TERMINADA.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("marcar_serie_terminada es solo para series")
        
    # Obtener metadatos
    meta = repository.obtener_tv_metadata(contenido.id)
    if not meta:
        if contenido.es_anime and contenido.mal_id:
            provider = JikanProvider()
            temporadas = provider.obtener_temporadas(contenido.mal_id)
        else:
            provider = TMDBProvider()
            temporadas = provider.obtener_temporadas(contenido.tmdb_id)
        import json
        repository.upsert_tv_metadata(contenido.id, len(temporadas), json.dumps(temporadas))
        meta = repository.obtener_tv_metadata(contenido.id)
        
    import json
    temporadas_dict = json.loads(meta['temporadas_json']) if meta else {}
    
    # Calcular última temporada y episodio
    if not temporadas_dict:
        ultima_temporada = 1
        ultimo_episodio = 1
    else:
        ultima_temporada = max(int(k) for k in temporadas_dict.keys())
        ultimo_episodio = temporadas_dict[str(ultima_temporada)]
        if ultimo_episodio == 0:
            ultimo_episodio = 1  # Si es 0 (emisión), marcamos al menos el cap 1 de la temporada
            
    # Marcar último episodio como visto
    episodio_visto = EpisodioVisto(
        id=None,
        usuario_contenido_id=usuario_contenido_id,
        tmdb_episode_id=None,
        temporada=ultima_temporada,
        episodio=ultimo_episodio,
        episodio_absoluto=None,
        fecha_visto=datetime.now().isoformat(),
        origen="IMPORTADO"
    )
    repository.insertar_episodio_visto(episodio_visto)
    
    # Cambiar estado a TERMINADA
    usuario_contenido_actualizado = cambiar_estado(usuario_contenido_id, TERMINADA)
    repository.upsert_historial_recomendacion(contenido.id, "YA_LA_VI")
    
    progreso = ProgresoSerie(
        id=None,
        usuario_contenido_id=usuario_contenido_id,
        temporada_actual=ultima_temporada,
        episodio_actual=ultimo_episodio,
        episodio_absoluto_actual=None,
        actualizado_en=datetime.now().isoformat()
    )
    
    return progreso, usuario_contenido_actualizado

def guardar_para_despues(usuario_contenido_id: int) -> UsuarioContenido:
    """
    Guarda un contenido en "Para después". No cambia el estado (ya nace como
    PENDIENTE al crearse el usuario_contenido), pero deja constancia en
    historial_recomendaciones de que el usuario eligió esta acción.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    repository.upsert_historial_recomendacion(usuario_contenido.contenido_id, "PARA_DESPUES")
    
    return usuario_contenido

def registrar_siguiente(contenido_id: int) -> None:
    """
    Registra que el usuario tocó "Siguiente" para un contenido mostrado en HOY.
    A diferencia de las otras dos funciones, no requiere que exista un
    usuario_contenido — el usuario puede saltear algo que todavía no agregó
    a su biblioteca.
    """
    repository.upsert_historial_recomendacion(contenido_id, "SIGUIENTE")

def empezar_serie(usuario_contenido_id: int) -> ProgresoSerie:
    """
    Inicia una serie nueva ("Empezar a ver"). Transiciona PENDIENTE -> EN_PROGRESO
    y crea el registro inicial de progreso en Temporada 1, Episodio 1.

    Lanza ValueError si el usuario_contenido no existe o si el contenido asociado
    no es de tipo TV.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("empezar_serie es solo para series")
        
    _verificar_limite_series_en_progreso()
    cambiar_estado(usuario_contenido_id, EN_PROGRESO)
    
    progreso = ProgresoSerie(
        id=None,
        usuario_contenido_id=usuario_contenido_id,
        temporada_actual=1,
        episodio_actual=1,
        episodio_absoluto_actual=1 if contenido.es_anime else None,
        actualizado_en=None
    )
    repository.upsert_progreso_serie(progreso)
    repository.upsert_historial_recomendacion(contenido.id, "EMPEZAR_EN_PROGRESO")
    
    return progreso

def importar_progreso_serie(
    usuario_contenido_id: int,
    temporada_actual: int,
    episodio_actual: int,
    marcar_anteriores: bool = False,
    episodio_absoluto_actual: int | None = None,
) -> ProgresoSerie:
    """
    Registra el progreso de una serie que el usuario ya venía viendo antes de
    instalar QuéVeoHoy ("Ya la estoy viendo"). Transiciona PENDIENTE -> EN_PROGRESO
    y fija temporada_actual/episodio_actual directamente (sin pasar por T1E1).

    Si marcar_anteriores es True, consulta el caché de temporadas para generar
    e insertar todos los episodios anteriores como vistos con origen 'IMPORTADO'.

    Lanza ValueError si el usuario_contenido no existe o si el contenido no es TV.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("importar_progreso_serie es solo para series")
        
    _verificar_limite_series_en_progreso()
    cambiar_estado(usuario_contenido_id, EN_PROGRESO)
    
    if marcar_anteriores:
        meta = repository.obtener_tv_metadata(contenido.id)
        if not meta:
            if contenido.es_anime and contenido.mal_id:
                provider = JikanProvider()
                temporadas = provider.obtener_temporadas(contenido.mal_id)
            else:
                provider = TMDBProvider()
                temporadas = provider.obtener_temporadas(contenido.tmdb_id)
            import json
            repository.upsert_tv_metadata(contenido.id, len(temporadas), json.dumps(temporadas))
            meta = repository.obtener_tv_metadata(contenido.id)
            
        if meta:
            import json
            temporadas_dict = json.loads(meta['temporadas_json'])
            ep_abs = 1
            for t in range(1, temporada_actual + 1):
                t_str = str(t)
                max_ep = temporadas_dict.get(t_str, 0)
                if max_ep == 0 and t < temporada_actual:
                    continue # No data for this season
                
                limit_ep = episodio_actual - 1 if t == temporada_actual else max_ep
                for ep in range(1, limit_ep + 1):
                    episodio_visto = EpisodioVisto(
                        id=None,
                        usuario_contenido_id=usuario_contenido_id,
                        tmdb_episode_id=None,
                        temporada=t,
                        episodio=ep,
                        episodio_absoluto=ep_abs if contenido.es_anime else None,
                        fecha_visto=None,
                        origen="IMPORTADO"
                    )
                    repository.insertar_episodio_visto(episodio_visto)
                    ep_abs += 1
            
    progreso = ProgresoSerie(
        id=None,
        usuario_contenido_id=usuario_contenido_id,
        temporada_actual=temporada_actual,
        episodio_actual=episodio_actual,
        episodio_absoluto_actual=episodio_absoluto_actual,
        actualizado_en=None
    )
    repository.upsert_progreso_serie(progreso)
    repository.upsert_historial_recomendacion(contenido.id, "EMPEZAR_EN_PROGRESO")
    
    return progreso

def vi_este_capitulo(
    usuario_contenido_id: int,
    proxima_temporada: int | None = None,
    proximo_episodio: int | None = None,
    proximo_episodio_absoluto: int | None = None,
    es_fin_de_serie: bool = False,
) -> tuple[ProgresoSerie, UsuarioContenido]:
    """
    Marca como visto el episodio actual (el que está guardado en progreso_serie)
    y avanza el progreso.

    - Si es_fin_de_serie es False: proxima_temporada y proximo_episodio son
      obligatorios (quien llama ya sabe, por metadata de TMDB, cuál es el
      siguiente episodio — incluyendo el caso de pasar de temporada).
      proximo_episodio_absoluto es opcional, solo aplica si el contenido es anime.

    - Si es_fin_de_serie es True: proxima_temporada y proximo_episodio deben
      venir en None (no hay "próximo"). Además de guardar el episodio actual
      como visto, transiciona el usuario_contenido a TERMINADA.

    Lanza ValueError si:
    - el usuario_contenido no existe, o el contenido no es de tipo TV
    - no hay un progreso_serie guardado (la serie nunca se empezó)
    - es_fin_de_serie es False pero falta proxima_temporada o proximo_episodio
    - es_fin_de_serie es True pero se pasó proxima_temporada o proximo_episodio
      igual (evita bugs de la capa que llama)
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("vi_este_capitulo es solo para series")
        
    progreso_serie = repository.obtener_progreso_serie(usuario_contenido_id)
    if not progreso_serie:
        raise ValueError("La serie no tiene progreso registrado, usá empezar_serie o importar_progreso_serie primero")
        
    if not es_fin_de_serie and (proxima_temporada is None or proximo_episodio is None):
        raise ValueError("Si no es fin de serie, proxima_temporada y proximo_episodio son obligatorios")
        
    if es_fin_de_serie and (proxima_temporada is not None or proximo_episodio is not None):
        raise ValueError("Si es fin de serie, proxima_temporada y proximo_episodio deben ser None")
        
    episodio_visto = EpisodioVisto(
        id=None,
        usuario_contenido_id=usuario_contenido_id,
        tmdb_episode_id=None,
        temporada=progreso_serie.temporada_actual,
        episodio=progreso_serie.episodio_actual,
        episodio_absoluto=progreso_serie.episodio_absoluto_actual,
        fecha_visto=None,
        origen="APP"
    )
    repository.insertar_episodio_visto(episodio_visto)
    
    if es_fin_de_serie:
        usuario_contenido_actualizado = cambiar_estado(usuario_contenido_id, TERMINADA)
        progreso_serie_actualizado = progreso_serie
    else:
        progreso_serie.temporada_actual = proxima_temporada
        progreso_serie.episodio_actual = proximo_episodio
        if proximo_episodio_absoluto is not None:
            progreso_serie.episodio_absoluto_actual = proximo_episodio_absoluto
        repository.upsert_progreso_serie(progreso_serie)
        
        usuario_contenido_actualizado = usuario_contenido
        progreso_serie_actualizado = progreso_serie
        
    return progreso_serie_actualizado, usuario_contenido_actualizado

def obtener_historial_recomendaciones(limit: int = 50) -> list[HistorialRecomendacionItem]:
    """
    Devuelve el historial de recomendaciones (todo lo que apareció en HOY alguna
    vez), de más reciente a más antiguo, con el título y tipo de cada contenido
    ya resueltos para que la UI no tenga que hacer consultas adicionales.
    """
    historial = repository.obtener_historial_recomendaciones(limit)
    res = []
    for h in historial:
        contenido = repository.obtener_contenido_por_id(h.contenido_id)
        if contenido:
            res.append(HistorialRecomendacionItem(
                contenido_id=h.contenido_id,
                titulo=contenido.titulo,
                tipo=contenido.tipo,
                fecha=h.fecha,
                accion=h.accion,
                veces_mostrada=h.veces_mostrada
            ))
    return res

def obtener_peliculas_vistas() -> list[PeliculaVista]:
    """Devuelve las películas marcadas como vistas, de más reciente a más antigua."""
    peliculas_vistas = repository.obtener_peliculas_vistas()
    res = []
    for pv in peliculas_vistas:
        contenido = repository.obtener_contenido_por_id(pv.contenido_id)
        if contenido:
            res.append(PeliculaVista(
                contenido_id=pv.contenido_id,
                titulo=contenido.titulo,
                fecha_finalizacion=pv.fecha_finalizacion
            ))
    return res

def obtener_series_vistas() -> list[SerieVista]:
    """
    Devuelve, para cada serie con al menos un episodio visto, el título y la
    lista completa de episodios vistos (de más reciente a más antiguo).
    """
    ids = repository.obtener_usuario_contenido_ids_con_episodios_vistos()
    res = []
    for uid in ids:
        usuario_contenido = repository.obtener_usuario_contenido_por_id(uid)
        if not usuario_contenido:
            continue
        contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
        if not contenido:
            continue
        episodios = repository.obtener_episodios_vistos_por_usuario_contenido(uid)
        res.append(SerieVista(
            usuario_contenido_id=uid,
            contenido_id=contenido.id,
            titulo=contenido.titulo,
            episodios=episodios
        ))
    return res

def buscar_en_historial(query: str) -> list[ResultadoBusqueda]:
    """
    Busca contenido por título (recomendado alguna vez o agregado a la biblioteca)
    y devuelve, para cada resultado, su estado actual (si existe) y la última vez
    que apareció en HOY, para que la UI pueda ofrecer las acciones correspondientes.
    """
    contenidos = repository.buscar_contenido_por_titulo(query)
    res = []
    for c in contenidos:
        usuario_contenido = repository.obtener_usuario_contenido_por_contenido_id(c.id)
        historial = repository.obtener_historial_recomendacion_por_contenido_id(c.id)
        
        res.append(ResultadoBusqueda(
            contenido_id=c.id,
            titulo=c.titulo,
            tipo=c.tipo,
            estado_actual=usuario_contenido.estado if usuario_contenido else None,
            usuario_contenido_id=usuario_contenido.id if usuario_contenido else None,
            ultima_aparicion=historial.fecha if historial else None
        ))
    return res

def _verificar_limite_series_en_progreso() -> None:
    """
    Lanza LimiteSeriesEnProgresoAlcanzado si ya hay MAX_SERIES_EN_PROGRESO series
    en estado EN_PROGRESO. Uso interno — se llama antes de cualquier transición
    que lleve una serie a EN_PROGRESO (empezar, importar progreso o reanudar).
    """
    en_progreso = repository.obtener_usuario_contenido_en_progreso_tv()
    if len(en_progreso) >= MAX_SERIES_EN_PROGRESO:
        raise LimiteSeriesEnProgresoAlcanzado(f"Ya tenés {MAX_SERIES_EN_PROGRESO} series en progreso.")

def obtener_series_en_progreso() -> list[SerieEnProgreso]:
    """
    Devuelve las series actualmente en EN_PROGRESO, con su temporada/episodio
    actual. Útil tanto para el apartado "En progreso" de la UI como para mostrar
    la lista al usuario cuando alcanza el límite (sección 9 del diseño:
    "Series en progreso: 1. Friends 2. Chernobyl...").
    """
    en_progreso = repository.obtener_usuario_contenido_en_progreso_tv()
    res = []
    for uc in en_progreso:
        contenido = repository.obtener_contenido_por_id(uc.contenido_id)
        progreso = repository.obtener_progreso_serie(uc.id)
        if contenido and progreso:
            res.append(SerieEnProgreso(
                usuario_contenido_id=uc.id,
                contenido_id=contenido.id,
                titulo=contenido.titulo,
                tipo=contenido.tipo,
                es_anime=contenido.es_anime,
                temporada_actual=progreso.temporada_actual,
                episodio_actual=progreso.episodio_actual,
                estado=uc.estado
            ))
    return res

def obtener_series_activas() -> list[SerieEnProgreso]:
    """
    Devuelve las series EN_PROGRESO y PAUSADAS. Útil para la pestaña En Progreso de la UI.
    """
    activas = repository.obtener_usuario_contenido_en_progreso_y_pausadas_tv()
    res = []
    for uc in activas:
        contenido = repository.obtener_contenido_por_id(uc.contenido_id)
        progreso = repository.obtener_progreso_serie(uc.id)
        if contenido and progreso:
            res.append(SerieEnProgreso(
                usuario_contenido_id=uc.id,
                contenido_id=contenido.id,
                titulo=contenido.titulo,
                tipo=contenido.tipo,
                es_anime=contenido.es_anime,
                temporada_actual=progreso.temporada_actual,
                episodio_actual=progreso.episodio_actual,
                estado=uc.estado
            ))
    return res

def pausar_serie(usuario_contenido_id: int) -> UsuarioContenido:
    """
    Pausa una serie en progreso ("Quiero seguir viéndola eventualmente").
    Transiciona EN_PROGRESO -> PAUSADA. Mientras esté pausada, no cuenta contra
    el límite de MAX_SERIES_EN_PROGRESO.
    Lanza ValueError si el contenido no es de tipo TV.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("pausar_serie es solo para series")
        
    return cambiar_estado(usuario_contenido_id, PAUSADA)

def reanudar_serie(usuario_contenido_id: int) -> UsuarioContenido:
    """
    Reanuda una serie pausada. Transiciona PAUSADA -> EN_PROGRESO, validando
    primero el límite de series en progreso.
    Lanza ValueError si el contenido no es de tipo TV.
    Lanza LimiteSeriesEnProgresoAlcanzado si ya hay 4 series en progreso.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    contenido = repository.obtener_contenido_por_id(usuario_contenido.contenido_id)
    if not contenido or contenido.tipo != "TV":
        raise ValueError("reanudar_serie es solo para series")
        
    _verificar_limite_series_en_progreso()
    return cambiar_estado(usuario_contenido_id, EN_PROGRESO)

def abandonar(usuario_contenido_id: int) -> UsuarioContenido:
    """
    Abandona un contenido en progreso o pausado ("No quiero seguir viendo esto").
    Transiciona EN_PROGRESO o PAUSADA -> ABANDONADA (ambas ya son transiciones
    válidas según TRANSICIONES_VALIDAS de la Fase 1). El registro NO se borra:
    queda marcado como abandonado en la base. La exclusión de recomendaciones
    futuras para contenido abandonado es responsabilidad de la capa de
    integración con TMDB, no de esta función.
    """
    usuario_contenido = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not usuario_contenido:
        raise ValueError(f"No se encontró el UsuarioContenido con ID {usuario_contenido_id}")
        
    return cambiar_estado(usuario_contenido_id, ABANDONADA)

def agregar_a_biblioteca(contenido_id: int) -> UsuarioContenido:
    """
    Crea un UsuarioContenido en PENDIENTE si no existe para el contenido dado.
    """
    uc = repository.obtener_usuario_contenido_por_contenido_id(contenido_id)
    if not uc:
        uc_new = UsuarioContenido(
            id=None,
            contenido_id=contenido_id,
            estado=PENDIENTE,
            fecha_agregado=datetime.now().isoformat(),
            fecha_inicio=None,
            fecha_finalizacion=None
        )
        new_id = repository.insertar_usuario_contenido(uc_new)
        uc_new.id = new_id
        return uc_new
    return uc

def obtener_biblioteca_inactiva() -> list[ElementoBiblioteca]:
    """
    Retorna todo el contenido que está pendiente, pausado o abandonado.
    """
    inactivos = repository.obtener_biblioteca_inactiva()
    res = []
    for uc in inactivos:
        c = repository.obtener_contenido_por_id(uc.contenido_id)
        if c:
            res.append(ElementoBiblioteca(usuario_contenido=uc, contenido=c))
    return res

def recomendacion_de_hoy(ignorar_id: int = None, es_prefetch: bool = False, forzar_aleatoria: bool = False) -> Optional[Contenido]:
    """
    Motor de recomendación diario.
    1. Si hay series en progreso (y no se forzó aleatoria), devuelve la primera (a menos que sea prefetch).
    2. Si no, pide tendencias al Provider (TMDB), filtra las ya vistas/descartadas/abandonadas,
       y elige una al azar.
    """
    if not forzar_aleatoria and not es_prefetch:
        activa = repository.obtener_recomendacion_activa_hoy()
        if activa and activa.id != ignorar_id:
            return activa

    if not forzar_aleatoria:
        en_progreso = obtener_series_en_progreso()
        if en_progreso:
            if es_prefetch:
                return None
            c = repository.obtener_contenido_por_id(en_progreso[0].contenido_id)
            if c:
                return c
            
    import datetime
    today_str = datetime.date.today().isoformat()
    historial_reciente = repository.obtener_historial_recomendaciones(limit=50)
    
    # Check if we already generated a recommendation today and it's still valid
    for h in historial_reciente:
        if h.fecha and h.fecha.startswith(today_str) and h.accion == "RECOMENDADA_HOY":
            if ignorar_id and h.contenido_id == ignorar_id:
                continue
            c_db = repository.obtener_contenido_por_id(h.contenido_id)
            if c_db:
                uc = repository.obtener_usuario_contenido_por_contenido_id(c_db.id)
                if uc and uc.estado != PENDIENTE:
                    continue
                if es_prefetch:
                    return c_db
                return c_db
            
    tmdb_provider = TMDBProvider()
    jikan_provider = JikanProvider()
    bolsa = []
    
    # 10 de cada uno
    peliculas = tmdb_provider.obtener_tendencias_peliculas()[:10]
    series = tmdb_provider.obtener_tendencias_series()[:10]
    animes = jikan_provider.obtener_tendencias_anime()[:10]
    
    bolsa.extend(peliculas)
    bolsa.extend(series)
    bolsa.extend(animes)
    
    candidatos_validos = []
    for cont in bolsa:
        if cont.mal_id is not None:
            c_db = repository.obtener_contenido_por_mal_id(cont.mal_id)
        else:
            c_db = repository.obtener_contenido_por_tmdb_id(cont.tmdb_id, cont.tipo)
            
        if c_db:
            uc = repository.obtener_usuario_contenido_por_contenido_id(c_db.id)
            hist = repository.obtener_historial_recomendacion_por_contenido_id(c_db.id)
            
            if uc and uc.estado in (TERMINADA, ABANDONADA, PAUSADA, EN_PROGRESO):
                continue
                
            if hist and hist.accion == "SIGUIENTE":
                continue
                
        candidatos_validos.append(cont)
        
    if not candidatos_validos:
        inactivos = obtener_biblioteca_inactiva()
        pendientes = [elem for elem in inactivos if elem.usuario_contenido.estado == PENDIENTE]
        if pendientes:
            rec = pendientes[0].contenido
            if not es_prefetch:
                repository.upsert_historial_recomendacion(rec.id, "RECOMENDADA_HOY")
            return rec
        return None
        
    elegido = random.choice(candidatos_validos)
    
    if elegido.mal_id is not None:
        c_db = repository.obtener_contenido_por_mal_id(elegido.mal_id)
    else:
        c_db = repository.obtener_contenido_por_tmdb_id(elegido.tmdb_id, elegido.tipo)
        
    if not c_db:
        new_id = repository.insertar_contenido(elegido)
        elegido.id = new_id
    else:
        elegido = c_db
        
    if not es_prefetch:
        repository.upsert_historial_recomendacion(elegido.id, "RECOMENDADA_HOY")
        
    return elegido

def avanzar_progreso_serie(usuario_contenido_id: int) -> tuple[ProgresoSerie, UsuarioContenido, bool]:
    """
    Avanza el progreso de la serie consultando la caché de temporadas.
    Llama a vi_este_capitulo resolviendo automáticamente si avanza al próximo
    episodio, a la próxima temporada, o si la serie finaliza.
    Retorna (progreso, usuario_contenido, es_fin_de_serie).
    """
    uc = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not uc:
        raise ValueError("Usuario contenido no encontrado")
        
    c = repository.obtener_contenido_por_id(uc.contenido_id)
    prog = repository.obtener_progreso_serie(uc.id)
    if not c or not prog:
        raise ValueError("Contenido o progreso no encontrado")

    meta = repository.obtener_tv_metadata(c.id)
    import json
    if not meta:
        if c.es_anime and c.mal_id:
            provider = JikanProvider()
            temporadas = provider.obtener_temporadas(c.mal_id)
        else:
            provider = TMDBProvider()
            temporadas = provider.obtener_temporadas(c.tmdb_id)
        repository.upsert_tv_metadata(c.id, len(temporadas), json.dumps(temporadas))
        meta = repository.obtener_tv_metadata(c.id)

    temporadas_dict = json.loads(meta['temporadas_json'])
    
    temp_actual_str = str(prog.temporada_actual)
    ep_actual = prog.episodio_actual
    ep_abs = prog.episodio_absoluto_actual + 1 if prog.episodio_absoluto_actual else None
    
    total_ep_en_temp = temporadas_dict.get(temp_actual_str, 0)
    
    if total_ep_en_temp > 0 and ep_actual >= total_ep_en_temp:
        siguiente_temp = prog.temporada_actual + 1
        siguiente_temp_str = str(siguiente_temp)
        if siguiente_temp_str in temporadas_dict:
            p, u = vi_este_capitulo(uc.id, siguiente_temp, 1, ep_abs, False)
            return p, u, False
        else:
            p, u = vi_este_capitulo(uc.id, None, None, None, True)
            return p, u, True
    else:
        p, u = vi_este_capitulo(uc.id, prog.temporada_actual, ep_actual + 1, ep_abs, False)
        return p, u, False

def corregir_progreso_serie(usuario_contenido_id: int, nueva_temporada: int, nuevo_episodio: int, nuevo_episodio_absoluto: int | None = None) -> ProgresoSerie:
    """
    Corrige el progreso modificando episodios_vistos (eliminando posteriores o rellenando faltantes).
    Luego recalcula progreso_serie.
    """
    uc = repository.obtener_usuario_contenido_por_id(usuario_contenido_id)
    if not uc:
        raise ValueError("Usuario contenido no encontrado")
        
    prog = repository.obtener_progreso_serie(uc.id)
    if not prog:
        raise ValueError("No hay progreso que corregir")
        
    c = repository.obtener_contenido_por_id(uc.contenido_id)

    # 1. Borrar todos los episodios vistos posteriores al nuevo progreso
    repository.eliminar_episodios_vistos_posteriores(uc.id, nueva_temporada, nuevo_episodio, nuevo_episodio_absoluto)

    # 2. Revisar qué episodios faltan antes del nuevo progreso y rellenarlos
    vistos = repository.obtener_episodios_vistos_por_usuario_contenido(uc.id)
    vistos_set = set((v.temporada, v.episodio) for v in vistos)

    meta = repository.obtener_tv_metadata(c.id)
    if not meta:
        if c.es_anime and c.mal_id:
            provider = JikanProvider()
            temporadas = provider.obtener_temporadas(c.mal_id)
        else:
            provider = TMDBProvider()
            temporadas = provider.obtener_temporadas(c.tmdb_id)
        import json
        repository.upsert_tv_metadata(c.id, len(temporadas), json.dumps(temporadas))
        meta = repository.obtener_tv_metadata(c.id)
        
    if meta:
        import json
        temporadas_dict = json.loads(meta['temporadas_json'])
        ep_abs = 1
        for t in range(1, nueva_temporada + 1):
            t_str = str(t)
            max_ep = temporadas_dict.get(t_str, 0)
            if max_ep == 0 and t < nueva_temporada:
                continue
            
            limit_ep = nuevo_episodio - 1 if t == nueva_temporada else max_ep
            for ep in range(1, limit_ep + 1):
                if (t, ep) not in vistos_set:
                    episodio_visto = EpisodioVisto(
                        id=None,
                        usuario_contenido_id=uc.id,
                        tmdb_episode_id=None,
                        temporada=t,
                        episodio=ep,
                        episodio_absoluto=ep_abs if c.es_anime else None,
                        fecha_visto=None,
                        origen="IMPORTADO"
                    )
                    repository.insertar_episodio_visto(episodio_visto)
                ep_abs += 1
                
    # 3. Recalcular progreso_serie
    prog.temporada_actual = nueva_temporada
    prog.episodio_actual = nuevo_episodio
    prog.episodio_absoluto_actual = nuevo_episodio_absoluto
    repository.upsert_progreso_serie(prog)
    
    # 4. Ajustar estado si era terminada y la corrigió hacia atrás
    if uc.estado == TERMINADA:
        cambiar_estado(uc.id, EN_PROGRESO)
        
    return prog

def eliminar_de_biblioteca(usuario_contenido_ids: list[int]) -> None:
    """Elimina permanentemente de la biblioteca los IDs provistos."""
    for uc_id in usuario_contenido_ids:
        repository.eliminar_usuario_contenido(uc_id)

def buscar_online(query: str) -> list[Contenido]:
    """Busca en TMDB y Jikan simultáneamente."""
    tmdb_provider = TMDBProvider()
    jikan_provider = JikanProvider()
    
    resultados_tmdb = tmdb_provider.buscar_contenido(query)
    resultados_jikan = jikan_provider.buscar_contenido(query)
    
    return resultados_tmdb + resultados_jikan

