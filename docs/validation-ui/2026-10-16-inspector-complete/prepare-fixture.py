"""Prepare isolated file/EXIF fixtures; preserve the synthetic JPEG raster bytes."""
from pathlib import Path
import shutil
import struct
import sys

workspace = Path(__file__).resolve().parents[3]
profile = (workspace / (sys.argv[1] if len(sys.argv) > 1 else "target/mcp-inspector-complete-20261016")).resolve()
if not profile.is_relative_to(workspace / "target"):
    raise SystemExit("Fixture profile must stay inside workspace/target")
root = profile / "fixtures" / "2026-秋日旅行"
root.mkdir(parents=True, exist_ok=True)
assets = Path(__file__).parent / "fixtures"
sources = [assets / name for name in ["history-v1.jpg", "history-v0.jpg", "history-branch.jpg"]]
for i in range(18):
    shutil.copy2(sources[i % 3], root / f"DSC{8812+i:05}.jpg")
for j, name in enumerate(["日本", "欧洲", "城市", "自然", "美食"]):
    folder = root / name
    folder.mkdir(exist_ok=True)
    for i in range(3):
        shutil.copy2(sources[(i+j) % 3], folder / f"{name}-{i+1:02}.jpg")

def ascii_tag(tag, text):
    data = text.encode("ascii") + b"\0"
    return tag, 2, len(data), data

def ifd(offset, entries):
    entries = sorted(entries)
    tail = bytearray()
    out = bytearray(struct.pack("<H", len(entries)))
    base = offset + 2 + 12 * len(entries) + 4
    for tag, kind, count, data in entries:
        if len(data) <= 4:
            value = data.ljust(4, b"\0")
        else:
            value = struct.pack("<I", base + len(tail))
            tail += data
            tail += b"\0" * (len(tail) % 2)
        out += struct.pack("<HHI", tag, kind, count) + value
    return bytes(out + struct.pack("<I", 0) + tail)

primary = [ascii_tag(0x010F, "Canon"), ascii_tag(0x0110, "Canon EOS R6"),
           ascii_tag(0x0132, "2026:10:14 14:23:36"), (0x8769, 4, 1, struct.pack("<I", 0))]
sub_offset = 8 + len(ifd(8, primary))
primary[-1] = (0x8769, 4, 1, struct.pack("<I", sub_offset))
secondary = [(0x829A, 5, 1, struct.pack("<II", 1, 200)),
             (0x829D, 5, 1, struct.pack("<II", 56, 10)),
             (0x8827, 3, 1, struct.pack("<H", 100)),
             ascii_tag(0x9003, "2026:10:14 14:23:36"),
             (0x920A, 5, 1, struct.pack("<II", 35, 1)),
             ascii_tag(0xA434, "RF24-105mm F4 L IS USM")]
exif = b"Exif\0\0II" + struct.pack("<HI", 42, 8) + ifd(8, primary) + ifd(sub_offset, secondary)
photo = root / "DSC08812.jpg"
raw = photo.read_bytes()
assert raw[:2] == b"\xff\xd8"
photo.write_bytes(raw[:2] + b"\xff\xe1" + struct.pack(">H", len(exif)+2) + exif + raw[2:])
print(root)
