"""Synthetic 3840x2160 JPEG for the same-process WorkCrate benchmark (Pillow)."""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

target = Path(sys.argv[1] if len(sys.argv) > 1 else
              ".local/validation/profiles/workcrate-performance-before/fixtures/large.jpg")
target.parent.mkdir(parents=True, exist_ok=True)
image = Image.new("RGB", (3840, 2160), "white")
draw = ImageDraw.Draw(image)
for y in range(0, 2160, 16):
    for x in range(0, 3840, 16):
        draw.rectangle((x, y, x + 14, y + 14),
                       fill=((x // 16 * 17) % 256, (y // 16 * 19) % 256,
                             ((x + y) // 16 * 7) % 256))
image.save(target, quality=95)
