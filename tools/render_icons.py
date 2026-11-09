"""Icon suggestions for the new client, drawn from the original GUI's `GFX_GUI_AO_LOGO` (161x285, steel on a green key).

The logo's silhouette (every pixel that is not the green key) becomes a smooth high-resolution mask; the faithful variant keeps
the logo's own steel shading, the others colour the silhouette. Writes 1024px PNGs, 256px previews, a .ico per variant and a
contact sheet with the small sizes.

    python render_icons2.py <GFX_GUI_AO_LOGO.png> <out dir>
"""
import os, sys
from PIL import Image, ImageChops, ImageDraw, ImageFilter

src, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
S = 1024
logo = Image.open(src).convert("RGBA")

# The silhouette: not the green key (the texture's alpha is opaque everywhere).
sil = Image.new("L", logo.size, 0)
px = logo.load()
for y in range(logo.height):
    for x in range(logo.width):
        r, g, b, a = px[x, y]
        key = g > 180 and r < 90 and b < 90
        sil.putpixel((x, y), 0 if key else 255)
# (the key bleeds a little into the edge pixels: pull the edge in by half a pixel of green mix)
sil = sil.filter(ImageFilter.GaussianBlur(0.6)).point(lambda v: 255 if v >= 150 else 0)

HEIGHT = 0.80  # the emblem's height as a part of the icon


def place(img, resample=Image.LANCZOS):
    """`img` (the logo's size) scaled so the emblem is HEIGHT of the frame, centred."""
    scale = S * HEIGHT / logo.height
    size = (round(logo.width * scale), round(logo.height * scale))
    big = img.resize(size, resample)
    frame = Image.new(img.mode, (S, S), 0 if img.mode == "L" else (0, 0, 0, 0))
    frame.paste(big, ((S - size[0]) // 2, (S - size[1]) // 2))
    return frame


MASK = place(sil).filter(ImageFilter.GaussianBlur(2.5)).point(lambda v: 255 if v >= 128 else 0).filter(ImageFilter.GaussianBlur(1.0))

# The logo's own shading, with the key removed and the edge taken from the mask.
steel = logo.copy()
steel.putalpha(sil)
STEEL = place(steel)
STEEL.putalpha(MASK)


def rounded_square(color, radius=S * 0.22, inset=S * 0.04):
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(im).rounded_rectangle([inset, inset, S - inset, S - inset], radius=radius, fill=color)
    return im


def vertical_gradient(top, bottom):
    g = Image.new("RGBA", (S, S))
    d = ImageDraw.Draw(g)
    for y in range(S):
        t = y / (S - 1)
        d.line([(0, y), (S, y)], fill=tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(4)))
    return g


def paint(base, mask, fill):
    layer = fill if isinstance(fill, Image.Image) else Image.new("RGBA", (S, S), fill)
    base.paste(layer, (0, 0), mask)
    return base


def shadow(mask, offset=(0, 14), blur=18, alpha=150):
    sh = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    sh.paste((0, 0, 0, alpha), (0, 0), mask)
    sh = sh.filter(ImageFilter.GaussianBlur(blur))
    moved = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    moved.paste(sh, offset)
    return moved


def glow(mask, color, blur=50):
    g = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    g.paste(color, (0, 0), mask)
    return g.filter(ImageFilter.GaussianBlur(blur))


def inset(mask, width):
    """The mask pulled in by `width` pixels."""
    return mask.filter(ImageFilter.MinFilter(width * 2 + 1))


def outline(mask, width):
    return ImageChops.subtract(mask.filter(ImageFilter.MaxFilter(width * 2 + 1)), mask)


def bevel(mask, width=16):
    """A light rim along the mask's upper-left edges and a dark one along the lower-right (a soft emboss)."""
    inner = inset(mask, width)
    rim = ImageChops.subtract(mask, inner)
    up = Image.new("L", (S, S), 0)
    up.paste(rim, (-width // 2, -width // 2))
    down = Image.new("L", (S, S), 0)
    down.paste(rim, (width // 2, width // 2))
    light = ImageChops.multiply(rim, up).filter(ImageFilter.GaussianBlur(3))
    dark = ImageChops.multiply(rim, down).filter(ImageFilter.GaussianBlur(3))
    return light, dark


variants = {}

# A. Steel: the original logo's own shading (lifted a little) on a slate square, a shadow under it.
a = rounded_square((58, 63, 72, 255))
sheen = vertical_gradient((255, 255, 255, 28), (0, 0, 0, 60))
sheen.putalpha(ImageChops.multiply(sheen.getchannel("A"), a.getchannel("A")))
a = Image.alpha_composite(a, sheen)
a = Image.alpha_composite(a, shadow(MASK))
from PIL import ImageEnhance
lifted = ImageEnhance.Brightness(STEEL).enhance(1.35)
lifted.putalpha(STEEL.getchannel("A"))
a = Image.alpha_composite(a, lifted)
variants["A_steel"] = a

# B. Teal: the silhouette in notum teal on deep navy, a soft emboss.
b = rounded_square((13, 27, 40, 255))
paint(b, MASK, vertical_gradient((0, 196, 200, 255), (0, 132, 140, 255)))
light, dark = bevel(MASK)
paint(b, light, (190, 255, 250, 255))
paint(b, dark, (0, 70, 80, 255))
variants["B_teal"] = b

# C. Ember: a sunset gradient with a warm glow, for the desert of Rubi-Ka.
c = rounded_square((26, 17, 16, 255))
c = Image.alpha_composite(c, glow(MASK, (255, 120, 40, 110)))
paint(c, MASK, vertical_gradient((255, 214, 120, 255), (214, 64, 28, 255)))
light, dark = bevel(MASK, 12)
paint(c, light, (255, 245, 210, 255))
variants["C_ember"] = c

# D. Outline: a thin light line on near-black, with a faint fill.
d = rounded_square((18, 18, 22, 255))
paint(d, MASK, (255, 255, 255, 18))
paint(d, outline(inset(MASK, 6), 7), (205, 212, 220, 255))
variants["D_outline"] = d

# E. Glyph: the silhouette alone, white with a shadow, on transparency (title bars, dark docks).
e = Image.new("RGBA", (S, S), (0, 0, 0, 0))
e = Image.alpha_composite(e, shadow(MASK, offset=(0, 10), blur=22, alpha=180))
paint(e, MASK, vertical_gradient((255, 255, 255, 255), (214, 220, 228, 255)))
variants["E_glyph"] = e

# F. Prism: a blue-to-violet gradient with a bright rim, a nod to the new renderer.
f = rounded_square((15, 17, 30, 255))
f = Image.alpha_composite(f, glow(MASK, (90, 120, 255, 70)))
paint(f, MASK, vertical_gradient((96, 170, 255, 255), (156, 72, 230, 255)))
light, dark = bevel(MASK, 14)
paint(f, light, (225, 240, 255, 255))
variants["F_prism"] = f

# G. Steel on dark, no square: the logo as a Windows-style free-standing icon.
g = Image.new("RGBA", (S, S), (0, 0, 0, 0))
g = Image.alpha_composite(g, shadow(MASK, offset=(0, 16), blur=26, alpha=200))
g = Image.alpha_composite(g, STEEL)
variants["G_steel_free"] = g

for name, im in variants.items():
    im.save(os.path.join(out, f"{name}.png"))
    im.resize((256, 256), Image.LANCZOS).save(os.path.join(out, f"{name}_256.png"))
    im.save(os.path.join(out, f"{name}.ico"), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

names = list(variants)
cell, gap = 200, 24
sheet = Image.new("RGBA", (gap + len(names) * (cell + gap), 24 + 192 + 20 + 64 + 20 + 32 + 20 + 16 + 24 + 100), (120, 120, 124, 255))
draw = ImageDraw.Draw(sheet)
sheet.paste(Image.new("RGBA", (sheet.width, 100), (236, 236, 236, 255)), (0, sheet.height - 100))
for i, name in enumerate(names):
    x = gap + i * (cell + gap)
    y = 24
    sheet.alpha_composite(variants[name].resize((192, 192), Image.LANCZOS), (x + 4, y))
    y += 192 + 20
    for size in (64, 32, 16):
        sheet.alpha_composite(variants[name].resize((size, size), Image.LANCZOS), (x + 4, y))
        y += size + 20
    sheet.alpha_composite(variants[name].resize((48, 48), Image.LANCZOS), (x + 4, sheet.height - 100 + 26))
    sheet.alpha_composite(variants[name].resize((24, 24), Image.LANCZOS), (x + 70, sheet.height - 100 + 38))
    draw.text((x + 4, 4), name, fill=(255, 255, 255, 255))
sheet.convert("RGB").save(os.path.join(out, "contact_sheet.png"))
print("wrote", len(variants), "variants to", out)
