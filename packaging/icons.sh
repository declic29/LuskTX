#!/usr/bin/env bash
# Regenerate every app icon from the Lusk Texte logo, assets/app-icon/wordcraft-1024.png (the
# master, 1024 x 1024 with a transparent background: replace this file to change the icon, then
# run this script). It sits outside hicolor/ because Linux icon themes stop at 512.
#
# Needs: python3 with Pillow (pip install Pillow), and cargo for the Windows .ico. The outputs are
# committed, so builds and packaging never need these tools.
#
#   packaging/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/assets/app-icon"
ID="bzh.lusk.texte"
MASTER="$DIR/wordcraft-1024.png"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

[ -f "$MASTER" ] || { echo "error: $MASTER not found" >&2; exit 1; }
python3 -c "import PIL" 2>/dev/null || { echo "error: Pillow not found (pip install Pillow)" >&2; exit 1; }

# Windows and Linux use the logo full bleed. macOS icons sit on Apple's grid: an 824/1024 body
# with a transparent margin. The .icns is written by Pillow, so iconutil (macOS only) isn't needed.
python3 - "$MASTER" "$DIR" "$ID" "$TMP" <<'EOF'
import os, sys
from PIL import Image

master, out, app_id, tmp = sys.argv[1:5]
src = Image.open(master).convert("RGBA")
if src.size != (1024, 1024):
    sys.exit(f"error: {master} must be 1024 x 1024 (is {src.size[0]} x {src.size[1]})")

def sized(im, s):
    return im if im.size == (s, s) else im.resize((s, s), Image.Resampling.LANCZOS)

# Apple grid: the logo scaled to 824/1024, centred on a transparent 1024 canvas.
mac = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
mac.alpha_composite(sized(src, 824), (100, 100))

# Runtime window/Dock icon on macOS (embedded by apps/wordcraft); other platforms use hicolor 256.
sized(mac, 512).save(os.path.join(out, "wordcraft-macos-512.png"), optimize=True)

# Linux hicolor theme.
for s in (16, 24, 32, 48, 64, 128, 256, 512):
    d = os.path.join(out, "hicolor", f"{s}x{s}", "apps")
    os.makedirs(d, exist_ok=True)
    sized(src, s).save(os.path.join(d, f"{app_id}.png"), optimize=True)

# Windows .ico sizes, packed by xtask below.
for s in (16, 20, 24, 32, 40, 48, 64, 128, 256):
    sized(src, s).save(os.path.join(tmp, f"ico-{s}.png"))

# macOS .icns.
mac.save(os.path.join(out, "wordcraft.icns"), format="ICNS",
         append_images=[sized(mac, s) for s in (16, 32, 64, 128, 256, 512)])
EOF

ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/wordcraft.ico" "${ICO_PNGS[@]}")
echo "icons written to $DIR"
