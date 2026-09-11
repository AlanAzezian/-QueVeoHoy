import urllib.request
import urllib.parse
import urllib.error
import json
import random
from typing import List
from datetime import datetime

from app.config import TMDB_API_KEY
from app.models import Contenido, TIPO_PELICULA, TIPO_SERIE
from app.providers.base import ContentProvider

BASE_URL = "https://api.themoviedb.org/3"

class TMDBProvider(ContentProvider):
    def _make_request(self, endpoint: str, params: dict = None) -> dict:
        if params is None:
            params = {}
        params['api_key'] = TMDB_API_KEY
        params['language'] = 'es-ES'
        
        query_string = urllib.parse.urlencode(params)
        url = f"{BASE_URL}{endpoint}?{query_string}"
        
        try:
            req = urllib.request.Request(url, headers={'Accept': 'application/json'})
            with urllib.request.urlopen(req, timeout=3.0) as response:
                return json.loads(response.read().decode('utf-8'))
        except Exception as e:
            print(f"TMDB Error en {endpoint}: {e}")
            return {}

    def _parse_results(self, results: list, tipo: str, force_es_anime: bool = False) -> List[Contenido]:
        parsed = []
        for item in results:
            titulo = item.get("title") if tipo == TIPO_PELICULA else item.get("name")
            titulo_original = item.get("original_title") if tipo == TIPO_PELICULA else item.get("original_name")
            date_str = item.get("release_date") if tipo == TIPO_PELICULA else item.get("first_air_date")
            
            poster_path = item.get("poster_path")
            poster_url = f"https://image.tmdb.org/t/p/w342{poster_path}" if poster_path else None
            
            # TMDB Anime Detection
            genre_ids = item.get("genre_ids", [])
            original_language = item.get("original_language", "")
            origin_country = item.get("origin_country", [])
            
            is_tmdb_anime = (16 in genre_ids) and (original_language == "ja" or "JP" in origin_country)
            es_anime = force_es_anime or is_tmdb_anime
            
            contenido = Contenido(
                id=None,  # DB id, not assigned yet
                tmdb_id=item.get("id"),
                mal_id=None,
                tipo=tipo,
                es_anime=es_anime,
                titulo=titulo or "Sin Título",
                titulo_original=titulo_original,
                poster_url=poster_url,
                sinopsis=item.get("overview", ""),
                fecha_estreno=date_str,
                creado_en=datetime.now().isoformat()
            )
            parsed.append(contenido)
        return parsed

    def obtener_tendencias_peliculas(self) -> List[Contenido]:
        page = random.randint(1, 500)
        data = self._make_request("/discover/movie", {
            "sort_by": "popularity.desc",
            "vote_count.gte": "100",
            "vote_average.gte": "5.0",
            "page": str(page)
        })
        return self._parse_results(data.get("results", []), TIPO_PELICULA)

    def obtener_tendencias_series(self) -> List[Contenido]:
        page = random.randint(1, 500)
        data = self._make_request("/discover/tv", {
            "sort_by": "popularity.desc",
            "vote_count.gte": "100",
            "vote_average.gte": "5.0",
            "page": str(page)
        })
        return self._parse_results(data.get("results", []), TIPO_SERIE)

    def obtener_tendencias_anime(self, tipo: str = "tv") -> List[Contenido]:
        page = random.randint(1, 5)
        endpoint = "/discover/tv" if tipo == "tv" else "/discover/movie"
        data = self._make_request(endpoint, {
            "with_genres": "16",
            "with_origin_country": "JP",
            "sort_by": "popularity.desc",
            "page": str(page)
        })
        return self._parse_results(data.get("results", []), TIPO_SERIE if tipo == "tv" else TIPO_PELICULA, force_es_anime=True)

    def obtener_temporadas(self, tv_id: int) -> dict:
        data = self._make_request(f"/tv/{tv_id}")
        temporadas = {}
        
        if not data:
            return temporadas
            
        for season in data.get("seasons", []):
            sn = season.get("season_number")
            if sn is not None and sn > 0:
                temporadas[str(sn)] = season.get("episode_count", 0)
                
        return temporadas

    def obtener_detalle_completo(self, tmdb_id: int, tipo: str) -> dict:
        endpoint = f"/movie/{tmdb_id}" if tipo == TIPO_PELICULA else f"/tv/{tmdb_id}"
        data = self._make_request(endpoint)
        if not data:
            return {}
            
        genres = data.get("genres", [])
        genero_str = genres[0]["name"] if genres else "Desconocido"
        
        return {
            "genero": genero_str,
            "sinopsis": data.get("overview", "")
        }

    def buscar_contenido(self, query: str) -> List[Contenido]:
        data = self._make_request("/search/multi", {"query": query})
        results = data.get("results", [])
        
        parsed = []
        for item in results:
            media_type = item.get("media_type")
            if media_type not in ["movie", "tv"]:
                continue
                
            tipo = TIPO_PELICULA if media_type == "movie" else TIPO_SERIE
            titulo = item.get("title") if tipo == TIPO_PELICULA else item.get("name")
            titulo_original = item.get("original_title") if tipo == TIPO_PELICULA else item.get("original_name")
            date_str = item.get("release_date") if tipo == TIPO_PELICULA else item.get("first_air_date")
            
            poster_path = item.get("poster_path")
            poster_url = f"https://image.tmdb.org/t/p/w342{poster_path}" if poster_path else None
            
            contenido = Contenido(
                id=None,
                tmdb_id=item.get("id"),
                mal_id=None,
                tipo=tipo,
                es_anime=False,
                titulo=titulo or "Sin Título",
                titulo_original=titulo_original,
                poster_url=poster_url,
                sinopsis=item.get("overview", ""),
                fecha_estreno=date_str,
                creado_en=datetime.now().isoformat()
            )
            parsed.append(contenido)
            
        return parsed

