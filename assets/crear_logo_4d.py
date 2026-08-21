import os
from PIL import Image, ImageDraw

def generar_logo_4d():
    size = 512
    scale = 2
    sw, sh = size * scale, size * scale
    
    canvas = Image.new("RGBA", (sw, sh), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    
    # 1. Contenedor redondeado exterior
    pad = 32
    draw.rounded_rectangle([pad, pad, sw - pad, sh - pad], radius=220, fill=(24, 16, 40, 255), outline=(236, 72, 153, 255), width=24)
    
    # 2. Marco interior de Pantalla (Centrado geométrico)
    draw.rounded_rectangle([150, 150, sw - 150, sh - 220], radius=85, fill=(18, 10, 34, 255), outline=(168, 85, 247, 255), width=26)
    
    # 3. Botón Play perfectamente centrado vertical y horizontalmente en la pantalla
    play_pts = [(410, 310), (630, 440), (410, 570)]
    draw.polygon(play_pts, fill=(56, 189, 248, 255))
    
    # 4. Pochoclos en esquina inferior derecha
    bucket_pts = [(620, 570), (655, 840), (815, 840), (850, 570)]
    draw.polygon(bucket_pts, fill=(236, 72, 153, 255))
    draw.ellipse([640, 500, 715, 580], fill=(254, 240, 138, 255))
    draw.ellipse([695, 460, 785, 560], fill=(255, 255, 255, 255))
    draw.ellipse([760, 500, 835, 580], fill=(254, 240, 138, 255))
    
    final_img = canvas.resize((size, size), Image.Resampling.LANCZOS)
    
    # Se corrige la ruta para que guarde en la misma carpeta 'assets' sin anidar
    assets_dir = os.path.dirname(os.path.abspath(__file__))
    os.makedirs(assets_dir, exist_ok=True)
    
    png_path = os.path.join(assets_dir, "logo.png")
    ico_path = os.path.join(assets_dir, "icon.ico")
    
    final_img.save(png_path, "PNG")
    sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
    final_img.save(ico_path, format="ICO", sizes=sizes)
    print(f"Icono transparente final 4D guardado en {ico_path}")

if __name__ == "__main__":
    generar_logo_4d()
