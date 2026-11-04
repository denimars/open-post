"""Builds a 1024x1024 app icon master from the diya PROJECT wordmark.

The source is only 160x64, so the plane mark is isolated, its alpha mask is
upscaled with LANCZOS and then contrast-sharpened to keep the edges crisp.
"""
from PIL import Image, ImageDraw, ImageFilter
import sys

SRC, OUT = sys.argv[1], sys.argv[2]
SIZE = 1024

src = Image.open(SRC).convert("RGBA")
mark = src.crop((0, 0, 54, 64))          # the paper-plane glyph, without the text
mark = mark.crop(mark.getbbox())

# Navy taken from the densest pixels of the logo itself.
px = [p for p in mark.getdata() if p[3] > 200]
navy = tuple(sum(c[i] for c in px) // len(px) for i in range(3))

# Upscale the mask, then sharpen the ramp so the 16x blow-up does not look soft.
inner = int(SIZE * 0.72)
w, h = mark.size
scale = inner / max(w, h)
target = (max(1, round(w * scale)), max(1, round(h * scale)))
alpha = mark.split()[3].resize(target, Image.LANCZOS)
alpha = alpha.point(lambda v: 0 if v < 100 else (255 if v > 165 else int((v - 100) * 255 / 65)))
alpha = alpha.filter(ImageFilter.GaussianBlur(0.4))

glyph = Image.new("RGBA", target, navy + (0,))
glyph.putalpha(alpha)

# White rounded square so the navy mark stays visible on dark taskbars too.
icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
plate = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
ImageDraw.Draw(plate).rounded_rectangle(
    (0, 0, SIZE - 1, SIZE - 1), radius=int(SIZE * 0.22), fill=(255, 255, 255, 255)
)
icon.alpha_composite(plate)
icon.alpha_composite(glyph, ((SIZE - target[0]) // 2, (SIZE - target[1]) // 2))
icon.save(OUT)
print(f"navy={navy} mark={mark.size} -> {target} saved {OUT}")
