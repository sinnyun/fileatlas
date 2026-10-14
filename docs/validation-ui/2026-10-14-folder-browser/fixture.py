# Regenerate only the isolated directory-browser validation fixture.
from pathlib import Path
import base64
root = Path(__file__).resolve().parents[3] / "target/mcp-folder-browser-fixture/library"
for directory in [".hidden", "01-Photos/nested", "02-Empty", "03-Many"]:
    (root / directory).mkdir(parents=True, exist_ok=True)
(root / "cover.png").write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAADwAAAAoCAIAAAAt2Q6oAAAASklEQVR4nO3OQQ3AIAAAMcDcfKEJgfOwx5ElrYLOZ5/xN+t24AvpinRFuiJdka5IV6Qr0hXpinRFuiJdka5IV6Qr0hXpinRFuvICnfsBzYOK5YYAAAAASUVORK5CYII="))
(root / "01-Photos/photo.png").write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAADAAAAAgCAIAAADbtmxLAAAAPklEQVR4nO3OMQHAIADAMEDXlKATgfv39GRHoiDz7Gf8ybod+BIqQkWoCBWhIlSEilARKkJFqAgVoSJUhMoLsrsBigzv+JUAAAAASUVORK5CYII="))
(root / "notes.txt").write_text("root note")
(root / ".hidden/secret.bin").write_bytes(b"hidden")
(root / "01-Photos/nested/deep.txt").write_text("deep only")
for i in range(245):
    (root / "03-Many" / f"file-{i:03}.txt").write_text("cache fixture")
