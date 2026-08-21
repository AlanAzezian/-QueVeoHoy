from app.providers.tmdb import TMDBProvider

def test():
    provider = TMDBProvider()
    
    print("Testing TMDB Trending Movies...")
    movies = provider.obtener_tendencias_peliculas()
    print(f"Got {len(movies)} movies. First: {movies[0].titulo if movies else 'None'}")

    print("Testing TMDB Trending TV...")
    tv = provider.obtener_tendencias_series()
    print(f"Got {len(tv)} tv shows. First: {tv[0].titulo if tv else 'None'}")

    print("Testing TMDB Anime...")
    anime = provider.obtener_tendencias_anime()
    print(f"Got {len(anime)} anime. First: {anime[0].titulo if anime else 'None'}")

if __name__ == '__main__':
    test()
