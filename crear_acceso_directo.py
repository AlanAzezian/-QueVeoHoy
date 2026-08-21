import os
import sys

def create_shortcut():
    try:
        import win32com.client
    except ImportError:
        print("Error: El modulo 'pywin32' no esta instalado. Instalalo con 'pip install pywin32'.")
        return

    desktop = os.path.join(os.environ['USERPROFILE'], 'Desktop')
    path = os.path.join(desktop, 'QuéVeoHoy.lnk')
    
    # Usar pythonw.exe para evitar que se abra la consola negra de Windows
    python_dir = os.path.dirname(sys.executable)
    target = os.path.join(python_dir, "pythonw.exe")
    if not os.path.exists(target):
        # Fallback a python.exe si no existe pythonw.exe
        target = sys.executable
        
    base_dir = os.path.dirname(os.path.abspath(__file__))
    main_script = os.path.join(base_dir, "main.py")
    icon_path = os.path.join(base_dir, "assets", "icon.ico")
    
    try:
        shell = win32com.client.Dispatch("WScript.Shell")
        shortcut = shell.CreateShortCut(path)
        shortcut.Targetpath = target
        shortcut.Arguments = f'"{main_script}"'
        shortcut.WorkingDirectory = base_dir
        shortcut.IconLocation = icon_path
        shortcut.save()
        print(f"Éxito: Acceso directo creado en {path}")
    except Exception as e:
        print(f"Error al crear el acceso directo: {e}")

if __name__ == "__main__":
    create_shortcut()
