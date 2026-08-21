from rembg import remove
from PIL import Image
import os

def procesar():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    raw_path = os.path.join(base_dir, "logo_crudo.png")
    final_logo_path = os.path.join(base_dir, "logo.png")
    ico_path = os.path.join(base_dir, "icon.ico")

    print("Removiendo fondo...")
    img = Image.open(raw_path)
    transparent = remove(img)

    print("Recortando y ajustando padding...")
    bbox = transparent.getbbox()
    if bbox:
        cropped = transparent.crop(bbox)
        width, height = cropped.size
        max_dim = max(width, height)
        # 5% padding total (2.5% per side) to make it massive
        new_size = int(max_dim * 1.05) 
        
        final_img = Image.new("RGBA", (new_size, new_size), (0, 0, 0, 0))
        offset = ((new_size - width) // 2, (new_size - height) // 2)
        final_img.paste(cropped, offset)
        
        final_img.save(final_logo_path, format="PNG")
        
        icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        final_img.save(ico_path, format="ICO", sizes=icon_sizes)
        print("Nuevo logo e icono generados con éxito.")
    else:
        print("Error: Bounding box vacio.")

if __name__ == "__main__":
    procesar()
