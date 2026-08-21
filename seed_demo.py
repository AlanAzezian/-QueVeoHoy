from app.database import init_db
from app.recommendation import agregar_nuevo_contenido
from app.models import TIPO_SERIE, TIPO_PELICULA

def run_seed():
    init_db()
    
    print("Limpiando base de datos y cargando demo...")
    import sqlite3
    from pathlib import Path
    db_file = Path.home() / '.queveohoy.db'
    conn = sqlite3.connect(str(db_file))
    conn.execute("DELETE FROM historial")
    conn.execute("DELETE FROM estado_contenido")
    conn.execute("DELETE FROM contenido")
    conn.commit()
    conn.close()
    
    print("Agregando Breaking Bad (Serie)...")
    agregar_nuevo_contenido(
        tipo=TIPO_SERIE,
        titulo="Breaking Bad",
        genero="Drama",
        anio=2008,
        temporadas_info={"1": 7, "2": 13, "3": 13, "4": 13, "5": 16},
        notas="Clásico imprescindible"
    )
    
    print("Agregando The Matrix (Película)...")
    agregar_nuevo_contenido(
        tipo=TIPO_PELICULA,
        titulo="The Matrix",
        genero="Ciencia Ficción",
        anio=1999,
        temporadas_info={},
        notas="Tomar la pastilla roja"
    )
    
    print("Agregando The Office (Serie)...")
    agregar_nuevo_contenido(
        tipo=TIPO_SERIE,
        titulo="The Office",
        genero="Comedia",
        anio=2005,
        temporadas_info={"1": 6, "2": 22},
        notas="Para reírse un rato"
    )

    print("¡Datos de prueba cargados! Ejecuta main.py para probar la app.")

if __name__ == '__main__':
    run_seed()
