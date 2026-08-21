import urllib.request
import urllib.parse
import urllib.error
import json
import random
from typing import List
from datetime import datetime

from app.models import Contenido, TIPO_SERIE
from app.providers.base import ContentProvider

BASE_URL = "https://api.jikan.moe/v4"

class JikanProvider(ContentProvider):
    def _make_request(self, endpoint: str, params: dict = None) -> dict:
        if params is None:
            params = {}
            
        query_string = urllib.parse.urlencode(params)
        url = f"{BASE_URL}{endpoint}?{query_string}"
        
        try:
            req = urllib.request.Request(url, headers={'Accept': 'application/json'})
            with urllib.request.urlopen(req, timeout=10) as response:
                return json.loads(response.read().decode('utf-8'))
        except Exception as e:
            print(f"Jikan Error en {endpoint}: {e}")
            return {}

    def _parse_results(self, results: list) -> List[Contenido]:
        parsed = []
        for item in results:
            titulo = item.get("title")
            titulo_original = item.get("title_japanese")
            
            # Fecha de estreno
            date_str = None
            aired = item.get("aired", {})
            if aired and "from" in aired and aired["from"]:
                date_str = aired["from"][:10]  # YYYY-MM-DD
            
            # Poster
            poster_url = None
            images = item.get("images", {}).get("jpg", {})
            if "large_image_url" in images:
                poster_url = images["large_image_url"]
            elif "image_url" in images:
                poster_url = images["image_url"]
                
            contenido = Contenido(
                id=None,
                tmdb_id=None, # Mapear cruzado en el futuro si se necesita
                mal_id=item.get("mal_id"),
                tipo=TIPO_SERIE,
                es_anime=True,
                titulo=titulo or "Sin Título",
                titulo_original=titulo_original,
                poster_url=poster_url,
                sinopsis=item.get("synopsis", ""),
                fecha_estreno=date_str,
                creado_en=datetime.now().isoformat()
            )
            parsed.append(contenido)
        return parsed

    def obtener_tendencias_peliculas(self) -> List[Contenido]:
        return []

    def obtener_tendencias_series(self) -> List[Contenido]:
        return []

    def obtener_tendencias_anime(self) -> List[Contenido]:
        # Limitamos a las primeras 100 páginas de popularidad (top 2500 animes)
        page = random.randint(1, 100)
        data = self._make_request("/anime", {
            "order_by": "popularity",
            "sort": "asc",
            "sfw": "true",
            "page": str(page)
        })
        return self._parse_results(data.get("data", []))

    def obtener_detalle_completo(self, content_id: int, tipo: str) -> dict:
        # A futuro si necesitamos detalle (Jikan da mucha info ya en /anime)
        # La sinopsis y género suelen venir en el mismo GET
        return {}

    def obtener_temporadas(self, mal_id: int) -> dict:
        # Para anime consideramos 1 sola temporada con N episodios
        data = self._make_request(f"/anime/{mal_id}")
        if not data or "data" not in data:
            return {}
            
        anime_data = data["data"]
        episodes = anime_data.get("episodes")
        if episodes:
            return {"1": episodes}
        return {"1": 0}  # En emisión, no se sabe el límite

    def buscar_contenido(self, query: str) -> List[Contenido]:
        data = self._make_request("/anime", {"q": query})
        return self._parse_results(data.get("data", []))
