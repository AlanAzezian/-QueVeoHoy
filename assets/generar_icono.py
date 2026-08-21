from rembg import remove
from PIL import Image
import os

def remove_bg_and_generate_ico():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    logo_path = os.path.join(base_dir, "logo.png")
    transparent_path = os.path.join(base_dir, "logo_transparent.png")
    ico_path = os.path.join(base_dir, "icon.ico")

    print(f"Buscando logo en: {logo_path}")
    if not os.path.exists(logo_path):
        print("Error: No se encontro el logo.")
        return

    print("Removiendo fondo usando IA (rembg)...")
    input_img = Image.open(logo_path)
    output_img = remove(input_img)
    output_img.save(transparent_path, format="PNG")
    print(f"Logo transparente guardado en: {transparent_path}")

    print("Regenerando .ico con fondo transparente...")
    icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    output_img.save(ico_path, format="ICO", sizes=icon_sizes)
    print(f"Icono guardado en: {ico_path}")
    print("Completado.")

if __name__ == "__main__":
    remove_bg_and_generate_ico()
