# Relevamiento del Estado Actual del Proyecto: QuéVeoHoy

Este documento detalla el estado actual del proyecto "QuéVeoHoy", basado en el código fuente, la estructura de la base de datos y el historial de cambios y decisiones tomadas en el workspace.

## 1. RESUMEN GENERAL DEL PROYECTO

"QuéVeoHoy" es una aplicación de escritorio diseñada para gestionar y hacer seguimiento de películas, series y anime. Su diferencial principal es un motor de recomendación automatizado que, diariamente, prioriza continuar series en progreso o sugiere contenido nuevo basado en tendencias, ayudando a combatir la "parálisis por análisis" al elegir qué ver.

**Estado general:** Funcional y en desarrollo activo. Cuenta con una arquitectura sólida separada en capas, integración con APIs externas (TMDB y Jikan), persistencia local en SQLite y una interfaz gráfica moderna construida con CustomTkinter.

## 2. ARQUITECTURA ACTUAL

El proyecto sigue una arquitectura en capas con separación de responsabilidades:

*   **`app/models.py`**: Define las entidades de dominio usando `dataclasses` de Python (`Contenido`, `UsuarioContenido`, `ProgresoSerie`, `EpisodioVisto`, `HistorialRecomendacion`). También almacena las constantes globales (ej. `TIPO_SERIE`, `ESTADO_EN_PROGRESO`).
*   **`app/database.py`**: Gestiona la conexión a SQLite e inicializa el esquema de la base de datos.
*   **`app/repository.py`**: Capa de persistencia (DAO). Contiene todas las consultas SQL (`INSERT`, `SELECT`, `UPDATE`). Convierte las tuplas de la base de datos en instancias de `models.py` aislando al resto del sistema de la lógica SQL.
*   **`app/recommendation.py`**: Contiene el core de la **lógica de negocio**. Implementa:
    *   Máquinas de estado (validación de transiciones legales entre estados de contenido).
    *   Reglas de negocio estrictas (límite máximo de series en progreso).
    *   El algoritmo de `recomendacion_de_hoy()` que decide qué mostrar en la pantalla principal.
    *   Funciones de alto nivel (`marcar_vista_pelicula`, `empezar_serie`, `guardar_para_despues`) que orquestan las llamadas al repositorio.
*   **`app/providers/`**: Módulos de integración con APIs externas. Implementan una interfaz común (heredando de `ContentProvider` en `base.py`).
    *   `tmdb.py`: Cliente para TheMovieDB (películas y series occidentales/generales).
    *   `jikan.py`: Cliente para Jikan/MyAnimeList (específico para anime).
*   **`app/ui.py`**: La interfaz gráfica desarrollada con CustomTkinter. Maneja los eventos de usuario, el renderizado de ventanas/pestañas, la carga y caché de imágenes (pósters), y delega la lógica a `recommendation.py`.
*   **`app/config.py`**: Gestión de variables de entorno y configuración (ej. API Keys).
*   **`main.py`**: Punto de entrada de la aplicación. Inicializa la base de datos y lanza la UI.

## 3. FUNCIONALIDADES IMPLEMENTADAS (Lista Exhaustiva)

Lo que la aplicación **ya es capaz de hacer**:

### Motor de Recomendación (`recomendacion_de_hoy`)
*   **Priorización de series en progreso:** Si el usuario tiene alguna serie en estado `en_progreso`, la aplicación *siempre* recomienda el próximo capítulo de la primera serie de la lista antes que contenido nuevo.
*   **Descubrimiento dinámico:** Si no hay series pendientes de ver hoy, obtiene aleatoriamente tendencias populares combinando 3 fuentes: Películas (TMDB), Series (TMDB) y Anime (Jikan).
*   **Filtro anti-repetición:** Evita recomendar contenido que el usuario ya tenga marcado como visto (`terminada`), `abandonada`, `pausada` o `en_progreso`.
*   **Sesión estable:** La recomendación del día se ancla en memoria para que no cambie erráticamente al navegar entre pestañas.

### Lógica de Negocio y Estados
*   **Máquina de estados estricta:** Un contenido puede estar en `pendiente`, `en_progreso`, `pausada`, `abandonada` o `terminada`. Se validan las transiciones (ej. no se puede ir de `terminada` a `en_progreso`).
*   **Límite de burnout:** Bloqueo explícito (`LimiteSeriesEnProgresoAlcanzado`) que impide tener más de 4 series en estado `en_progreso` simultáneamente.
*   **Acciones de contenido:**
    *   **"Ya la vi" (Películas):** Transiciona directo a `terminada`.
    *   **"Capítulo Visto" (Series):** Inicia la serie (`en_progreso`) o incrementa el `episodio_actual`.
    *   **"Dejar para después":** Agrega el contenido a la biblioteca con estado `pendiente` pero no lo muestra en "Hoy".
    *   **"Siguiente":** Descarta la recomendación. Deja registro en `historial_recomendaciones` (acción `SIGUIENTE`) para castigar su prioridad futura sin agregarlo a la biblioteca del usuario.

### Integración de Datos (Providers)
*   Integración con el endpoint `/discover/movie` y `/discover/tv` de TMDB, parseando título, póster (`image.tmdb.org`), sinopsis, y fecha de estreno.
*   Integración con el endpoint `/anime` de JikanV4 (ordenado por popularidad) para capturar metadata específica de anime.

### Interfaz Gráfica (CustomTkinter)
*   **Aspecto Visual:** Tema oscuro (`#0D0D12` fondo), paleta de acentos violetas (`#7F4FE0`, `#A78BFA`), fuente "Helvetica".
*   **Estructura:** Ventana principal fija configurada manualmente (`WINDOW_WIDTH = 800`, `WINDOW_HEIGHT = 880`) centrada en pantalla.
*   **Pestaña "Hoy":** Layout estilo tarjeta ("card-based"). Incluye póster con sombra emulada (`#2B1A4A`), título, metadatos, sinopsis con text-wrap estático (`wraplength=520`) y 3 botones estilo "pill" anclados abajo.
*   **Pestaña "En progreso":** Un componente `Treeview` que lista las series activas y muestra el progreso actual (ej. `T1 C1`).
*   **Pestaña "Para después":** Un `Treeview` que muestra la biblioteca inactiva (`pendiente`, `pausada`, `abandonada`). Incluye un botón para mover el ítem seleccionado a `en_progreso` (validando el límite de 4 series).
*   **Pestaña "Historial":** Un `Treeview` para auditar qué interacciones se tuvieron con las recomendaciones ("SIGUIENTE", "PARA_DESPUES", "EMPEZAR_EN_PROGRESO", "YA_LA_VI").
*   **Caché de Imágenes:** Las imágenes descargadas por `urllib` se transforman en objetos `CTkImage` y se guardan en un diccionario de caché para evitar recargas constantes en la UI.

## 4. HISTORIAL DE CAMBIOS RELEVANTES

A lo largo del proyecto se tomaron decisiones de diseño clave:

*   **Migración a CustomTkinter:** Se abandonó `tkinter` estándar (que lucía viejo) para adoptar un "Dark Mode" premium. Esto requirió reescribir los frames y labels a `CTkFrame` y `CTkLabel`.
*   **Control Manual de Geometría:** Se decidió usar valores absolutos (`WINDOW_WIDTH = 800`) y empaquetados específicos en lugar de redimensionamiento fluido. **Razón:** El comportamiento responsivo de Tkinter deformaba la tarjeta principal, descentraba botones y cortaba la sinopsis. La medida manual garantiza la estética de "app nativa premium".
*   **Refactor de la Persistencia de Recomendaciones:** *Cambio revertido/modificado*. Originalmente, apenas una recomendación se mostraba en pantalla, se creaba un registro `UsuarioContenido` en la base de datos como `pendiente`. **Razón del cambio:** Esto ensuciaba la base de datos de los usuarios con contenido que solo miraron de pasada. Se refactorizó para que el contenido solo pase a `usuario_contenido` si el usuario interactúa (Guardar para después, Visto). La acción de "Siguiente" ahora solo impacta en la tabla de `historial_recomendaciones`.
*   **Separación de TMDB y Jikan:** En los inicios se planeaba usar TMDB para anime, pero la metadata era inconsistente. Se separó la lógica usando abstracciones (`ContentProvider`) para permitir que el anime provenga de Jikan/MyAnimeList, manejando `tmdb_id` y `mal_id` como identificadores mutuamente excluyentes o complementarios.

## 5. ESTADO DE LA BASE DE DATOS

La base de datos es SQLite (`.queveohoy.db` en el home del usuario) y consta de 5 tablas principales:

1.  **`contenido`**: Caché local de la data de las APIs. Campos clave: `tmdb_id`, `mal_id`, `tipo` ('MOVIE' o 'TV'), `es_anime`, `titulo`, `poster_url`, `sinopsis`.
2.  **`usuario_contenido`**: Relación 1-1 entre el usuario y un contenido. Campos clave: `estado` (varchar restringido), `fecha_agregado`, `fecha_inicio`, `fecha_finalizacion`.
3.  **`progreso_serie`**: Tabla de estado rápido para TV. Guarda `temporada_actual` y `episodio_actual`.
4.  **`episodios_vistos`**: Fuente única de verdad. Guarda cada episodio individual visto (temporada, episodio, timestamp, origen: 'APP' o 'IMPORTADO').
5.  **`historial_recomendaciones`**: Registro de métricas y rechazos. Guarda qué `accion` se tomó con una recomendación y `veces_mostrada`.

## 6. PROBLEMAS CONOCIDOS / PENDIENTES (Issues)

*   **Lógica de avance de episodios ciega en UI:** En `ui.py` (`on_marcar_visto`), cuando el usuario marca un capítulo visto, el código hace `proximo_episodio=progreso.episodio_actual + 1`. Esto suma de a 1 infinitamente. La aplicación **no consulta a TMDB cuántos episodios tiene una temporada**, por lo que nunca hace el salto automático a "Temporada 2, Episodio 1" ni detecta el fin de serie.
*   **Ausencia de controles de estado manuales en UI:** La regla de negocio de "Pausar" o "Abandonar" una serie está programada en `recommendation.py` (`pausar_serie`, `abandonar`), pero no hay botones en la interfaz para ejecutar estas acciones. Si un usuario llega a 4 series "En progreso" y quiere iniciar una 5ta, no tiene forma gráfica de pausar una de las anteriores para liberar un cupo.
*   **Demora en inicialización/red de UI:** La obtención de recomendaciones (`refresh_hoy`) hace llamadas HTTP síncronas usando `urllib.request`. Si TMDB tarda en responder, la interfaz gráfica se congela hasta que se resuelva el request (no hay hilos/async explícito).

## 7. LO QUE FALTA O ESTÁ PLANEADO PERO NO IMPLEMENTADO

*   **Búsqueda manual:** El usuario debe esperar a que el motor recomiende algo. Falta un buscador libre ("Buscar serie/película") para agregar contenido proactivamente a la biblioteca.
*   **Importación de progreso en la UI ("Ya la estoy viendo"):** La función `importar_progreso_serie` permite decirle al sistema "estoy en la T3 E4", pero la interfaz gráfica aún no tiene un formulario que permita ingresar estos datos.
*   **Gestión de `tmdb_episode_id` y `episodio_absoluto`:** Las tablas están preparadas para manejar la numeración absoluta típica del anime (donde no hay temporadas, sino episodios del 1 al 1000) y IDs puntuales de episodios de TMDB, pero no se está usando activamente en la persistencia.
*   **Soporte de multilenguaje o preferencias de filtrado:** No hay settings para decir "No me recomiendes anime" o "Solo busco películas hoy". La ruleta está hardcodeada (10 de TMDB Movie, 10 de TMDB TV, 10 de Jikan) y se hace un `random.choice`.
