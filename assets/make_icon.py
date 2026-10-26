"""Render the Rig app icon at every size macOS wants."""
from PIL import Image, ImageDraw, ImageFilter, ImageFont

SIZE = 1024
BG = (16, 16, 16)
GREEN = (68, 214, 44)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def vertical_gradient(size, top, bottom):
    image = Image.new("RGB", size, top)
    draw = ImageDraw.Draw(image)
    for y in range(size[1]):
        draw.line([(0, y), (size[0], y)], fill=lerp(top, bottom, y / size[1]))
    return image


def render(size):
    scale = size / SIZE
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    # Squircle body
    body = vertical_gradient((size, size), (28, 30, 32), (10, 11, 12))
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=int(224 * scale), fill=255
    )
    canvas.paste(body, (0, 0), mask)

    draw = ImageDraw.Draw(canvas)
    draw.rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=int(224 * scale),
        outline=(58, 62, 66, 255), width=max(1, int(6 * scale)),
    )

    # Keycap, sitting a little high so the light under it has room
    cap = [int(232 * scale), int(272 * scale), int(792 * scale), int(668 * scale)]
    radius = int(74 * scale)
    cap_mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(cap_mask).rounded_rectangle(cap, radius=radius, fill=255)

    face = vertical_gradient((size, size), (66, 73, 78), (28, 32, 35))
    canvas.paste(face, (0, 0), cap_mask)
    draw.rounded_rectangle(
        cap, radius=radius, outline=(84, 92, 97, 255), width=max(1, int(5 * scale))
    )
    # Light escaping from under the cap: a wide bloom plus a hot core
    bloom = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(bloom).rounded_rectangle(
        [int(262 * scale), int(658 * scale), int(762 * scale), int(694 * scale)],
        radius=int(18 * scale), fill=(*GREEN, 255),
    )
    bloom = bloom.filter(ImageFilter.GaussianBlur(int(44 * scale)))
    canvas = Image.alpha_composite(canvas, bloom)

    core = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(core).rounded_rectangle(
        [int(316 * scale), int(664 * scale), int(708 * scale), int(682 * scale)],
        radius=int(9 * scale), fill=(150, 255, 118, 255),
    )
    core = core.filter(ImageFilter.GaussianBlur(int(9 * scale)))
    canvas = Image.alpha_composite(canvas, core)

    # Legend
    legend = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    font = ImageFont.truetype("web/fonts/Archivo.ttf", int(268 * scale))
    draw = ImageDraw.Draw(legend)
    draw.text((size / 2, int(474 * scale)), "R", font=font, fill=(240, 246, 242, 255), anchor="mm")
    canvas = Image.alpha_composite(canvas, legend)

    return canvas


sizes = [1024, 512, 256, 128, 64, 32, 16]
for s in sizes:
    render(s).save(f"assets/icon-{s}.png")
    print(f"assets/icon-{s}.png")
