from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "assets"
OUT.mkdir(parents=True, exist_ok=True)

font_candidates = [
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
]
font_path = next((Path(p) for p in font_candidates if Path(p).exists()), None)
if font_path is None:
    raise SystemExit("No monospace font found")

font = ImageFont.truetype(str(font_path), 26)
small = ImageFont.truetype(str(font_path), 21)

lines = [
    ("$ cargo run --bin x86-console", "prompt"),
    ("x86 native console v0.1.0", "title"),
    ("No browser or WebAssembly runtime is used.", "muted"),
    ("x86> load state arch_state-v3.bin.zst", "command"),
    ("saved state loaded", "ok"),
    ("x86> load bootloader seabios.bin", "command"),
    ("bootloader loaded", "ok"),
    ("x86> info", "command"),
    ("status: Created", "text"),
    ("ram: 134217728 bytes", "text"),
    ("saved-state: version 6, compressed=true, 66 buffers", "text"),
    ("x86> checksum state", "command"),
    ("state: 7763d30ac1a94cd3272ba5a8f8cc3e19...", "hash"),
    ("x86> quit", "command"),
]

colors = {
    "background": (15, 23, 42),
    "panel": (30, 41, 59),
    "border": (51, 65, 85),
    "prompt": (148, 163, 184),
    "title": (226, 232, 240),
    "muted": (148, 163, 184),
    "command": (125, 211, 252),
    "ok": (134, 239, 172),
    "text": (203, 213, 225),
    "hash": (196, 181, 253),
}

width, height = 1680, 900
image = Image.new("RGB", (width, height), colors["background"])
draw = ImageDraw.Draw(image)
draw.rounded_rectangle((36, 36, width - 36, height - 36), radius=22, fill=colors["panel"], outline=colors["border"], width=2)
# Window chrome.
draw.ellipse((70, 65, 88, 83), fill=(248, 113, 113))
draw.ellipse((100, 65, 118, 83), fill=(250, 204, 21))
draw.ellipse((130, 65, 148, 83), fill=(74, 222, 128))
draw.text((184, 58), "x86-console · native terminal", font=small, fill=colors["muted"])

x, y = 76, 128
for text, kind in lines:
    draw.text((x, y), text, font=font, fill=colors[kind])
    y += 49

image.save(OUT / "console-en.png")
# Keep a localized copy name so translated docs can link naturally while showing identical deterministic output.
image.save(OUT / "console-ru.png")
image.save(OUT / "console-uk.png")
