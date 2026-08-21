from PIL import Image
import os

def crop_and_resize():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    transparent_path = os.path.join(base_dir, "logo_transparent.png")
    final_logo_path = os.path.join(base_dir, "logo.png")
    ico_path = os.path.join(base_dir, "icon.ico")

    if not os.path.exists(transparent_path):
        print(f"Error: No se encontró {transparent_path}")
        return

    # Abrir la imagen transparente (generada por rembg)
    img = Image.open(transparent_path).convert("RGBA")
    
    # Obtener el bounding box (el área que realmente tiene pixeles no transparentes)
    bbox = img.getbbox()
    
    if bbox:
        # Recortar los bordes transparentes
        img_cropped = img.crop(bbox)
        
        # Calcular el nuevo tamaño para agregar un 10% de padding general
        width, height = img_cropped.size
        max_dim = max(width, height)
        # Añadir aprox 5% de margen a cada lado (ocupar casi el 90%)
        new_size = int(max_dim * 1.1) 
        
        # Crear un nuevo lienzo 100% transparente
        new_img = Image.new("RGBA", (new_size, new_size), (0, 0, 0, 0))
        
        # Pegar la imagen recortada en el centro
        offset = ((new_size - width) // 2, (new_size - height) // 2)
        new_img.paste(img_cropped, offset)
        
        # Sobreescribir el logo.png principal y el transparente para consistencia
        new_img.save(final_logo_path, format="PNG")
        new_img.save(transparent_path, format="PNG")
        
        # Regenerar el .ico
        icon_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
        new_img.save(ico_path, format="ICO", sizes=icon_sizes)
        print("Logo recortado, reescalado (ocupando 90% del lienzo) y exportado con transparencia Alpha 0 correcta.")
    else:
        print("No se encontró bounding box válido.")

if __name__ == "__main__":
    crop_and_resize()
