#!/usr/bin/env python3
"""Draw the Windows installer's two bitmaps from the Lusk Texte logo and the Lusk fox.

    python3 packaging/windows/images/generate.py

Writes, next to this script:
- dialog.bmp (493 x 312): the welcome and finish pages. WiX writes their text from x = 173 px,
  so the artwork stays in the 164 px band on the left.
- banner.bmp (493 x 58): the strip at the top of the other pages. Their title and description
  take the left 400 px, so the artwork sits on the right.

Sources: assets/app-icon/wordcraft-1024.png (the logo) and assets/avatar/lusk-renard.png (the fox),
with the caption in Inter (assets/fonts). Needs Pillow. The bitmaps are committed, so packaging
never runs this script.
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
LOGO = os.path.join(ROOT, "assets", "app-icon", "wordcraft-1024.png")
FOX = os.path.join(ROOT, "assets", "avatar", "lusk-renard.png")
FONT = os.path.join(ROOT, "assets", "fonts", "Inter-SemiBold.ttf")

BRAND = (0x1D, 0x9C, 0x9B)  # Lusk Texte's colour (crates/ui-egui/src/theme.rs)
INK = (0x13, 0x6E, 0x6D)  # its darker ink for text on light backgrounds
TOP = (0xF1, 0xFA, 0xF9)
BOTTOM = (0xD6, 0xEE, 0xEC)
WHITE = (255, 255, 255)

SCALE = 4  # draw large, then reduce: smooth edges in the 24-bit bitmaps


def sized(path, px):
    im = Image.open(path).convert("RGBA")
    return im.resize((px, px), Image.Resampling.LANCZOS)


def gradient(w, h, top, bottom):
    band = Image.new("RGB", (1, h))
    for y in range(h):
        t = y / max(h - 1, 1)
        band.putpixel((0, y), tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)))
    return band.resize((w, h))


def dialog():
    s = SCALE
    w, h, panel = 493 * s, 312 * s, 164 * s
    im = Image.new("RGBA", (w, h), WHITE + (255,))
    im.paste(gradient(panel, h, TOP, BOTTOM), (0, 0))
    ImageDraw.Draw(im).rectangle([panel, 0, panel + 2 * s - 1, h], fill=BRAND)
    logo = sized(LOGO, 132 * s)
    im.alpha_composite(logo, ((panel - logo.width) // 2, 22 * s))
    fox = sized(FOX, 64 * s)
    fy = 190 * s
    im.alpha_composite(fox, ((panel - fox.width) // 2, fy))
    draw = ImageDraw.Draw(im)
    font = ImageFont.truetype(FONT, 11 * s)
    caption = "association Lusk"
    box = draw.textbbox((0, 0), caption, font=font)
    draw.text(((panel - (box[2] - box[0])) // 2, fy + fox.height + 8 * s), caption, font=font, fill=INK)
    return im.convert("RGB").resize((493, 312), Image.Resampling.LANCZOS)


def banner():
    s = SCALE
    w, h = 493 * s, 58 * s
    im = Image.new("RGBA", (w, h), WHITE + (255,))
    # A soft fade from white into the pale brand tint behind the artwork.
    fade_from = 380 * s
    for x in range(fade_from, w):
        t = min((x - fade_from) / (40 * s), 1.0)
        c = tuple(round(a + (b - a) * t) for a, b in zip(WHITE, TOP))
        ImageDraw.Draw(im).line([(x, 0), (x, h)], fill=c)
    fox = sized(FOX, 38 * s)
    im.alpha_composite(fox, (404 * s, (h - fox.height) // 2))
    logo = sized(LOGO, 52 * s)
    im.alpha_composite(logo, (w - logo.width - 6 * s, (h - logo.height) // 2))
    return im.convert("RGB").resize((493, 58), Image.Resampling.LANCZOS)


def main():
    for path in (LOGO, FOX, FONT):
        if not os.path.isfile(path):
            sys.exit(f"error: {path} not found")
    dialog().save(os.path.join(HERE, "dialog.bmp"))
    banner().save(os.path.join(HERE, "banner.bmp"))
    print(f"installer bitmaps written to {HERE}")


if __name__ == "__main__":
    main()
