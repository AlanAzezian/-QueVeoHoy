import os
import math
from PIL import Image, ImageDraw

def create_app_icon():
    size = (512, 512)
    # Lienzo 100% transparente
    img = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # 1. Base circular estilizada (tipo Steam / OBS)
    # Borde exterior con brillo neón violeta
    draw.ellipse([16, 16, 496, 496], fill=(18, 18, 26, 255), outline=(139, 92, 246, 255), width=18)
    # Anillo interior fino cyan
    draw.ellipse([40, 40, 472, 472], outline=(56, 189, 248, 160), width=6)
    
    # 2. Botón Play central (Cyan Neón)
    play_points = [(190, 150), (370, 256), (190, 362)]
    draw.polygon(play_points, fill=(56, 189, 248, 255), outline=(255, 255, 255, 220))
    
    # 3. Pochoclos / Palomitas en la esquina inferior derecha (Magenta/Púrpura)
    # Balde/Contenedor
    bucket_points = [(330, 340), (430, 340), (410, 440), (350, 440)]
    draw.polygon(bucket_points, fill=(236, 72, 153, 255), outline=(255, 255, 255, 200), width=4)
    
    # Pochoclos individuales
    draw.ellipse([335, 305, 375, 345], fill=(255, 255, 255, 255), outline=(236, 72, 153, 255), width=4)
    draw.ellipse([365, 290, 405, 335], fill=(253, 224, 71, 255), outline=(236, 72, 153, 255), width=4)
    draw.ellipse([395, 310, 435, 345], fill=(255, 255, 255, 255), outline=(236, 72, 153, 255), width=4)
    
    assets_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)))
    png_path = os.path.join(assets_dir, "logo.png")
    ico_path = os.path.join(assets_dir, "icon.ico")
    
    # Guardar PNG en alta
    img.save(png_path, "PNG")
    
    # Generar ICO con todas las capas escaladas con antialiasing
    icon_sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
    img.save(ico_path, format='ICO', sizes=icon_sizes)
    print("Icono generado con éxito en:", ico_path)

if __name__ == "__main__":
    create_app_icon()
