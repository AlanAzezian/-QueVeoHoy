import os

html_content = """<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Presentación QuéVeoHoy</title>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.3.1/reveal.js"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.3.1/reset.css">
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.3.1/reveal.css">
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.3.1/theme/black.css">
    <style>
        :root {
            --r-background-color: #121217;
            --r-main-color: #F3F4F6;
            --r-heading-color: #6D28D9;
            --r-link-color: #6D28D9;
            --r-link-color-hover: #8B5CF6;
            --r-selection-background-color: #6D28D9;
        }
        .reveal h1, .reveal h2, .reveal h3, .reveal h4, .reveal h5, .reveal h6 {
            color: var(--r-heading-color);
            text-transform: none;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            font-weight: bold;
        }
        .reveal .slide-background {
            background-color: var(--r-background-color);
        }
        .reveal, .reveal p, .reveal li {
            color: var(--r-main-color);
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
        }
        .reveal .controls, .reveal .progress {
            color: var(--r-heading-color);
        }
        .accent {
            color: #6D28D9;
        }
        .box {
            background: rgba(255, 255, 255, 0.05);
            border: 2px solid #6D28D9;
            border-radius: 10px;
            padding: 20px;
            margin-top: 20px;
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
        }
    </style>
</head>
<body>
    <div class="reveal">
        <div class="slides">
            <section>
                <h1>QuéVeoHoy</h1>
                <h3>Estado Actual del Proyecto</h3>
                <div class="box">
                    <p><strong>Propósito:</strong> Gestor y recomendador personal de películas, series y anime.</p>
                    <p><strong>Stack:</strong> Python / CustomTkinter / SQLite</p>
                    <p><strong>APIs:</strong> TMDB y Jikan</p>
                </div>
            </section>
            
            <section>
                <h2>2. Pantalla Principal ("Hoy")</h2>
                <ul>
                    <li><strong>Tarjeta de recomendación:</strong> Destaca el contenido ideal para ver hoy.</li>
                    <li><strong>Póster centrado:</strong> Interfaz atractiva y visual.</li>
                    <li><strong>Título:</strong> Ajuste multilínea automático para nombres largos.</li>
                    <li><strong>Acciones:</strong> Adaptadas dinámicamente por tipo de contenido.</li>
                </ul>
            </section>
            
            <section>
                <h2>3. Gestión de Series e Importación</h2>
                <ul>
                    <li><strong>Modal Centrado:</strong> Selector intuitivo de temporadas y capítulos reales.</li>
                    <li><strong>Información Precisa:</strong> Datos extraídos de las APIs en tiempo real.</li>
                    <li><strong>Botón Directo:</strong> Opción "Ya la terminé" para flujo rápido y sin fricción.</li>
                </ul>
            </section>
            
            <section>
                <h2>4. Pestaña "En Progreso"</h2>
                <ul>
                    <li><strong>Seguimiento Episódico:</strong> Control exacto de lo que estás viendo.</li>
                    <li><strong>Tabla Simplificada:</strong> Vista clara y sin elementos redundantes.</li>
                    <li><strong>Reanudación Inmediata:</strong> Enlace directo a la vista de "Hoy" para continuar viendo.</li>
                </ul>
            </section>
            
            <section>
                <h2>5. Pestaña "Biblioteca"</h2>
                <p>Catálogo unificado de todo tu contenido.</p>
                <div class="box">
                    <ul>
                        <li><span class="accent">Pendientes</span></li>
                        <li><span class="accent">Terminados / Vistos</span></li>
                        <li><span class="accent">Abandonados</span></li>
                    </ul>
                </div>
            </section>
            
            <section>
                <h2>6. Doble Filtro Combinado</h2>
                <ul>
                    <li><strong>Selectores Independientes:</strong> Filtra por Tipo (Película, Serie, Anime) y por Estado.</li>
                    <li><strong>Búsqueda en Tiempo Real:</strong> Resultados instantáneos mientras escribes.</li>
                    <li><strong>Fluidez:</strong> La interfaz responde sin bloqueos al filtrar listas grandes.</li>
                </ul>
            </section>
            
            <section>
                <h2>7. Pestaña "Historial"</h2>
                <ul>
                    <li><strong>Auditoría Cronológica:</strong> Registro de tus últimas interacciones.</li>
                    <li><strong>Formato Limpio:</strong> Sin horas ni IDs técnicos que ensucien la vista.</li>
                    <li><strong>Layout Estable:</strong> Zona inferior anclada para evitar saltos visuales.</li>
                </ul>
            </section>
            
            <section>
                <h2>8. Arquitectura Multi-API</h2>
                <div class="box">
                    <p><strong>Normalización de Anime:</strong></p>
                    <ul>
                        <li>Unificación de esquemas entre Jikan (Anime) y TMDB (Películas/Series).</li>
                        <li>Implementación de bandera interna <code>es_anime</code> para enrutamiento.</li>
                        <li>Interfaz unificada a pesar de usar fuentes de datos distintas.</li>
                    </ul>
                </div>
            </section>
            
            <section>
                <h2>9. Rendimiento y Concurrencia</h2>
                <ul>
                    <li><strong>Buffer de Precarga:</strong> Tiempos de respuesta menores a 100ms.</li>
                    <li><strong>Debounce:</strong> Optimización de peticiones en campos de búsqueda.</li>
                    <li><strong>Request ID Token:</strong> Protección contra condiciones de carrera al cambiar de pestaña rápidamente.</li>
                </ul>
            </section>
            
            <section>
                <h2>10. Estado Actual y Roadmap</h2>
                <ul>
                    <li><strong>Hitos Alcanzados:</strong> Sistema base estable, interfaz CustomTkinter, integración TMDB/Jikan.</li>
                    <li><span class="accent">Próximos Pasos:</span></li>
                    <ul>
                        <li>Integración con Letterboxd.</li>
                        <li>Calendario semanal de emisiones.</li>
                        <li>Animación de ruleta para recomendaciones aleatorias.</li>
                    </ul>
                </ul>
            </section>
        </div>
    </div>
    <script>
        Reveal.initialize({
            hash: true,
            transition: 'slide',
            controls: true,
            progress: true,
            center: true,
        });
    </script>
</body>
</html>
"""

with open("presentacion.html", "w", encoding="utf-8") as f:
    f.write(html_content)

print("Archivo presentacion.html generado con éxito.")
