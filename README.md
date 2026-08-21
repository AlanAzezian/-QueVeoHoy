# 🎬 QuéVeoHoy

**QuéVeoHoy** es una aplicación de escritorio moderna diseñada para solucionar la indecisión al momento de consumir contenido multimedia. Actúa como un recomendador diario inteligente y gestor integral de películas, series y anime, integrando catálogos de bases de datos globales en una interfaz limpia, oscura y reactiva.

---

## ✨ Características Principales

* **🎲 Recomendación Diaria Persistente:** Sugiere un contenido destacado por día (película, serie o anime). La recomendación persiste entre reinicios y avanza bajo demanda del usuario (`Siguiente`, `Dejar para después`, `Ya la vi`).
* **📺 Gestión Episódica Inteligente:** Selector modal dinámico de temporada y capítulo para series y animes, permitiendo pausar, reanudar o completar producciones directamente.
* **📚 Biblioteca Centralizada:** Catálogo personal con soporte para múltiples estados (*Para después*, *Terminados / Vistos*, *Pausadas*, *Abandonadas*).
* **🔍 Doble Filtrado y Búsqueda en Vivo:** Filtrado simultáneo por tipo y estado, complementado con un buscador por texto en tiempo real.
* **📊 Panel de Métricas:** Contador en tiempo real con desglose detallado de contenido completado: total global, películas, series y animes.
* **📜 Historial Cronológico:** Registro de auditoría con las interacciones del usuario, fechas y eventos asociados.
* **🌐 Búsqueda Externa:** Módulo de exploración directa para incorporar contenido de TMDB y Jikan a la colección local.

---

## 🛠️ Stack Tecnológico

* **Lenguaje:** Python 3.10+
* **Interfaz Gráfica:** CustomTkinter
* **Base de Datos:** SQLite 3 (con soporte para categorización `es_anime`)
* **APIs:** TMDB API y Jikan API (MyAnimeList)
* **Librerías:** Pillow (PIL), Requests

---

## 🚀 Instalación y Puesta en Marcha

1. **Clonar el repositorio:**
   ```bash
   git clone https://github.com/TU_USUARIO/QueVeoHoy.git
   cd QueVeoHoy
   ```

2. **Crear y activar entorno virtual:**
   ```bash
   python -m venv venv
   venv\Scripts\activate
   ```

3. **Instalar dependencias:**
   ```bash
   pip install -r requirements.txt
   ```

4. **Configurar API Key:**
   Crear un archivo `.env` en la raíz con una clave propia de TMDB:
   ```env
   TMDB_API_KEY=tu_api_key_aqui
   ```

5. **Ejecutar la aplicación:**
   ```bash
   python main.py
   ```

## 📂 Arquitectura del Proyecto

```text
QueVeoHoy/
│
├── app/
│   ├── api/             # Conexión con TMDB y Jikan
│   ├── database.py      # Conexión SQLite y esquemas
│   ├── repository.py    # Consultas SQL, inserciones y métricas
│   ├── recommendation.py# Motor de recomendación y persistencia
│   └── ui.py            # Vistas, componentes y modales (CustomTkinter)
│
├── assets/              # Recursos gráficos e iconos
├── .gitignore           # Exclusión de archivos locales/temporales
├── requirements.txt     # Dependencias
└── README.md            # Documentación
```
