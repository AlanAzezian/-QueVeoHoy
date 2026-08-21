import os
from PIL import Image

def generate_ico():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    logo_path = os.path.join(base_dir, "logo.png")
    ico_path = os.path.join(base_dir, "icon.ico")
    
    if not os.path.exists(logo_path):
        print(f"Error: No se encontró el archivo {logo_path}")
        return

    # Abrir la imagen base (se recomienda que sea PNG cuadrada y de alta resolución, ej 512x512)
    img = Image.open(logo_path)
    
    # Resoluciones estándar recomendadas para Windows (.ico)
    icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    
    # Guardar como .ico especificando las resoluciones
    img.save(ico_path, format="ICO", sizes=icon_sizes)
    print(f"Éxito: Se generó correctamente el archivo {ico_path} con múltiples resoluciones.")

if __name__ == "__main__":
    generate_ico()
