from typing import Protocol, List
from app.models import Contenido

class ContentProvider(Protocol):
    def obtener_tendencias_peliculas(self) -> List[Contenido]:
        """Devuelve una lista de películas en tendencia."""
        ...

    def obtener_tendencias_series(self) -> List[Contenido]:
        """Devuelve una lista de series en tendencia."""
        ...

    def obtener_tendencias_anime(self) -> List[Contenido]:
        """Devuelve una lista de animes recomendados/en tendencia."""
        ...

    def obtener_detalle_completo(self, content_id: int, tipo: str) -> dict:
        """Devuelve metadata adicional, como generos o sinopsis ampliada."""
        ...
