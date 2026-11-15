"""Render every Rig brand asset from one definition of the mark.

The mark is the thing Rig actually does: a grid of keys where the lit ones spell
an R, with the light running across them the way an effect would.

    python3 assets/make_brand.py        # needs Pillow; run from the repo root

Writes assets/logo.svg, assets/icon-*.png, assets/Rig.icns (via iconutil),
assets/banner-dark.png, assets/banner-light.png and assets/social-preview.png.
"""
import os
import shutil
import subprocess

from PIL import Image, ImageDraw, ImageFilter, ImageFont

FONT = "web/fonts/Archivo.ttf"

# The R, on a 5 x 5 key grid.
GLYPH = [
    "####.",
    "#...#",
    "####.",
    "#..#.",
    "#...#",
]

# Geometry in a 100-unit box: 5 keys of 16 with 5 between them.
CELL, GAP, RADIUS = 16, 5, 4

# The light: brightest at the top left, settling into Rig green.
HOT = (150, 255, 120)
GREEN = (68, 214, 44)
DEEP = (38, 160, 24)

OFF_DARK = (44, 47, 49)
OFF_LIGHT = (214, 219, 216)

BG_DARK = (12, 13, 14)
BG_LIGHT = (246, 248, 246)
INK_DARK = (240, 246, 242)
INK_LIGHT = (18, 20, 21)


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def lit_colour(row, col):
    """A diagonal sweep, so the mark reads as a running effect, not a flat fill."""
    t = (row + col) / 8
    return lerp(HOT, GREEN, t / 0.5) if t < 0.5 else lerp(GREEN, DEEP, (t - 0.5) / 0.5)


def cells():
    for row, line in enumerate(GLYPH):
        for col, ch in enumerate(line):
            yield row, col, ch == "#"


def hex_(c):
    return "#%02x%02x%02x" % c


# --------------------------------------------------------------------- svg


def svg():
    rects = []
    for row, col, lit in cells():
        x, y = col * (CELL + GAP), row * (CELL + GAP)
        attrs = f'x="{x}" y="{y}" width="{CELL}" height="{CELL}" rx="{RADIUS}"'
        if lit:
            rects.append(f'  <rect {attrs} fill="{hex_(lit_colour(row, col))}"/>')
        else:
            rects.append(f'  <rect {attrs} class="off"/>')
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" role="img" aria-label="Rig">\n'
        "  <style>\n"
        f"    .off {{ fill: {hex_(OFF_DARK)}; }}\n"
        f"    @media (prefers-color-scheme: light) {{ .off {{ fill: {hex_(OFF_LIGHT)}; }} }}\n"
        "  </style>\n" + "\n".join(rects) + "\n</svg>\n"
    )


# ------------------------------------------------------------------ raster


def gradient(size, top, bottom):
    image = Image.new("RGB", size, top)
    draw = ImageDraw.Draw(image)
    for y in range(size[1]):
        draw.line([(0, y), (size[0], y)], fill=lerp(top, bottom, y / max(1, size[1] - 1)))
    return image


def mark(px, off, glow=True):
    """The mark at px x px on a transparent canvas, with light bleeding from lit keys."""
    pad = round(px * 0.25)  # room for the bloom
    full = px + 2 * pad
    s = px / 100
    keys = Image.new("RGBA", (full, full), (0, 0, 0, 0))
    light = Image.new("RGBA", (full, full), (0, 0, 0, 0))
    kd, ld = ImageDraw.Draw(keys), ImageDraw.Draw(light)
    for row, col, is_lit in cells():
        x0 = pad + col * (CELL + GAP) * s
        y0 = pad + row * (CELL + GAP) * s
        box = [x0, y0, x0 + CELL * s - 1, y0 + CELL * s - 1]
        r = max(1, RADIUS * s)
        if is_lit:
            colour = lit_colour(row, col)
            kd.rounded_rectangle(box, radius=r, fill=(*colour, 255))
            ld.rounded_rectangle(box, radius=r, fill=(*colour, 255))
        else:
            kd.rounded_rectangle(box, radius=r, fill=(*off, 255))
    out = Image.new("RGBA", (full, full), (0, 0, 0, 0))
    if glow and px >= 48:
        bloom = light.filter(ImageFilter.GaussianBlur(px * 0.07))
        bloom.putalpha(bloom.getchannel("A").point(lambda a: int(a * 0.55)))
        out = Image.alpha_composite(out, bloom)
    out = Image.alpha_composite(out, keys)
    return out.crop((0, 0, full, full)), pad


def icon(size):
    """macOS app icon. Drawn large and scaled down, so tiny sizes stay crisp."""
    if size < 512:
        big = 1024 if size > 32 else 512
        return _icon(big, small=size <= 32).resize((size, size), Image.LANCZOS)
    return _icon(size, small=False)


def _icon(size, small):
    s = size / 1024
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    inset = round(100 * s)  # Apple's grid leaves a margin round the body
    body_box = [inset, inset, size - inset - 1, size - inset - 1]
    radius = round(185 * s)

    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(body_box, radius=radius, fill=255)
    canvas.paste(gradient((size, size), (30, 33, 35), (10, 11, 12)), (0, 0), mask)
    if not small:
        ImageDraw.Draw(canvas).rounded_rectangle(
            body_box, radius=radius, outline=(62, 67, 70, 255), width=max(1, round(4 * s))
        )

    # Small sizes drop the margin and glow so the keys stay legible.
    if small:
        canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        ImageDraw.Draw(canvas).rounded_rectangle(
            [0, 0, size - 1, size - 1], radius=round(size * 0.22), fill=(*BG_DARK, 255)
        )
        px = round(size * 0.78)
    else:
        px = round(size * 0.46)
    m, pad = mark(px, OFF_DARK, glow=not small)
    offset = (size - px) // 2 - pad
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    layer.alpha_composite(m, (max(0, offset), max(0, offset)), (max(0, -offset), max(0, -offset)))
    return Image.alpha_composite(canvas, layer)


def font(size, weight):
    face = ImageFont.truetype(FONT, size)
    try:
        face.set_variation_by_name(weight)
    except Exception:
        pass
    return face


def lockup(width, height, dark, scale=2, tagline=True):
    """Mark + wordmark + tagline, for the README hero and the social card."""
    w, h = width * scale, height * scale
    bg, ink = (BG_DARK, INK_DARK) if dark else (BG_LIGHT, INK_LIGHT)
    sub = (138, 148, 143) if dark else (96, 104, 100)
    off = OFF_DARK if dark else OFF_LIGHT
    canvas = Image.new("RGBA", (w, h), (*bg, 255))

    px = round(h * 0.46)
    glyph_x = round(w * 0.075)
    glyph_y = (h - px) // 2

    if dark:
        halo = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        ImageDraw.Draw(halo).ellipse(
            [glyph_x - px * 0.5, glyph_y - px * 0.35, glyph_x + px * 1.5, glyph_y + px * 1.35],
            fill=(*GREEN, 34),
        )
        canvas = Image.alpha_composite(canvas, halo.filter(ImageFilter.GaussianBlur(px * 0.35)))

    m, pad = mark(px, off, glow=dark)
    canvas.alpha_composite(m, (glyph_x - pad, glyph_y - pad))

    draw = ImageDraw.Draw(canvas)
    text_x = glyph_x + px + round(px * 0.32)
    title = font(round(px * 0.62), "ExtraBold")
    draw.text((text_x, glyph_y - round(px * 0.04)), "RIG", font=title, fill=(*ink, 255), anchor="la")
    if tagline:
        draw.text(
            (text_x + round(px * 0.02), glyph_y + round(px * 0.70)),
            "keyboard control for macOS",
            font=font(round(px * 0.17), "Medium"),
            fill=(*sub, 255),
        )
        rule_y = glyph_y + px - round(px * 0.03)
        rule = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        rd = ImageDraw.Draw(rule)
        rd.rectangle([text_x, rule_y, w - glyph_x, rule_y + scale - 1],
                     fill=(*(ink if dark else (0, 0, 0)), 26))
        rd.rectangle([text_x, rule_y - scale, text_x + round(px * 0.55), rule_y + scale - 1],
                     fill=(*GREEN, 255))
        canvas = Image.alpha_composite(canvas, rule)
    return canvas.convert("RGB")


def icns():
    if not shutil.which("iconutil"):
        print("iconutil not found; skipped Rig.icns")
        return
    iconset = "assets/Rig.iconset"
    os.makedirs(iconset, exist_ok=True)
    for base in [16, 32, 128, 256, 512]:
        icon(base).save(f"{iconset}/icon_{base}x{base}.png")
        icon(base * 2).save(f"{iconset}/icon_{base}x{base}@2x.png")
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", "assets/Rig.icns"], check=True)
    shutil.rmtree(iconset)
    print("assets/Rig.icns")


if __name__ == "__main__":
    with open("assets/logo.svg", "w") as fh:
        fh.write(svg())
    print("assets/logo.svg")
    for size in [1024, 512, 256, 128, 64, 32, 16]:
        icon(size).save(f"assets/icon-{size}.png")
    print("assets/icon-{16..1024}.png")
    icns()
    lockup(1280, 340, dark=True).save("assets/banner-dark.png", optimize=True)
    lockup(1280, 340, dark=False).save("assets/banner-light.png", optimize=True)
    lockup(1280, 640, dark=True, scale=1).save("assets/social-preview.png", optimize=True)
    print("assets/banner-dark.png, assets/banner-light.png, assets/social-preview.png")
