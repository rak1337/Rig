"""Render the Rig icon and the README banner."""
from PIL import Image, ImageDraw, ImageFilter, ImageFont

GREEN = (68, 214, 44)
FONT = "web/fonts/Archivo.ttf"


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def gradient(size, top, bottom):
    image = Image.new("RGB", size, top)
    draw = ImageDraw.Draw(image)
    for y in range(size[1]):
        draw.line([(0, y), (size[0], y)], fill=lerp(top, bottom, y / size[1]))
    return image


def font(size, weight=None):
    face = ImageFont.truetype(FONT, size)
    if weight:
        try:
            face.set_variation_by_name(weight)
        except Exception:
            pass
    return face


def icon(size):
    s = size / 1024
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    body = gradient((size, size), (26, 28, 30), (9, 10, 11))
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=int(226 * s), fill=255)
    canvas.paste(body, (0, 0), mask)

    # keycap
    cap = [int(232 * s), int(258 * s), int(792 * s), int(690 * s)]
    cap_mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(cap_mask).rounded_rectangle(cap, radius=int(78 * s), fill=255)
    canvas.paste(gradient((size, size), (54, 60, 64), (24, 27, 29)), (0, 0), cap_mask)
    ImageDraw.Draw(canvas).rounded_rectangle(
        cap, radius=int(78 * s), outline=(74, 82, 86, 255), width=max(1, int(5 * s))
    )

    # the legend is the light source: bloom, then the crisp glyph
    glyph = "R"
    legend = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(legend).text(
        (size / 2, int(470 * s)), glyph, font=font(int(280 * s), "Bold"),
        fill=(*GREEN, 255), anchor="mm",
    )
    bloom = legend.filter(ImageFilter.GaussianBlur(int(26 * s)))
    canvas = Image.alpha_composite(canvas, bloom)
    canvas = Image.alpha_composite(canvas, legend)

    return canvas


def banner(width=1280, height=340):
    canvas = Image.new("RGBA", (width, height), (12, 13, 14, 255))

    glow = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse([40, 30, 470, height - 30], fill=(*GREEN, 40))
    canvas = Image.alpha_composite(canvas, glow.filter(ImageFilter.GaussianBlur(72)))

    canvas.alpha_composite(icon(198), (84, (height - 198) // 2))

    draw = ImageDraw.Draw(canvas)
    draw.text((330, 88), "RIG", font=font(112, "ExtraBold"), fill=(240, 246, 242, 255))
    draw.text((336, 214), "keyboard control for macOS", font=font(31, "Medium"),
              fill=(138, 148, 143, 255))

    rule = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    rd = ImageDraw.Draw(rule)
    rd.rectangle([336, 288, width - 96, 289], fill=(255, 255, 255, 18))
    rd.rectangle([336, 286, 452, 289], fill=(*GREEN, 255))
    canvas = Image.alpha_composite(canvas, rule)

    return canvas


for s in [1024, 512, 256, 128, 64, 32, 16]:
    icon(s).save(f"assets/icon-{s}.png")
banner().save("assets/banner.png")
print("wrote icons and assets/banner.png")
