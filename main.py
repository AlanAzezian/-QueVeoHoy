from app.database import init_db
from app.ui import run

def main():
    # Inicializar la base de datos
    init_db()
    # Iniciar la aplicación Tkinter
    run()

if __name__ == "__main__":
    main()
