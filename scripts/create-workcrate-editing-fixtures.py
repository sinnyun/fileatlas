"""Small linked-image fixtures for the same-process editing scenario (Pillow)."""
from pathlib import Path
import json
import sys
from PIL import Image, ImageDraw

workspace = Path(__file__).resolve().parent.parent
profile = (workspace / (sys.argv[1] if len(sys.argv) > 1 else
           ".local/validation/profiles/workcrate-editing-final")).resolve()
if not profile.is_relative_to(workspace / ".local/validation/profiles"):
    raise SystemExit("Use an isolated profile under .local/validation/profiles")
fixtures = profile / "fixtures"
fixtures.mkdir(parents=True, exist_ok=True)
for name, size, color in [("portrait.png", (300, 900), "#c6d9ec"),
                          ("wide.png", (1200, 300), "#b7ddc5")]:
    image = Image.new("RGB", size, color)
    draw = ImageDraw.Draw(image)
    for x in range(0, size[0], 30):
        draw.line((x, 0, x, size[1]), fill="#839aa4", width=2)
    image.save(fixtures / name)
if not (profile / "settings.json").exists():
    (profile / "settings.json").write_text(json.dumps({"color_mode": "light", "mcp_access_level": "full"}), encoding="utf-8")
