from collections import Counter
from PIL import Image, ImageFilter, ImageFont, ImageDraw, ImageEnhance, ImageOps

import argparse
import cgi
import colorsys
import string

# Level object from https://stackoverflow.com/a/3125421/2313792
class Level(object):
    def __init__(self, min_val, max_val, gamma):
        self.min_value, self.max_value = min_val / 255.0, max_val / 255.0
        self.interval = self.max_value - self.min_value
        self.inv_gamma = 1.0 / gamma

    def level_values(self, band_values):
        h, s, v = colorsys.rgb_to_hsv(*(i / 255.0 for i in band_values))
        if v <= self.min_value:
            v = 0.0
        elif v >= self.max_value:
            v = 1.0
        else:
            v = ((v - self.min_value) / self.interval) ** self.inv_gamma
        return tuple(int(255 * i) for i in colorsys.hsv_to_rgb(h, s, v))


def generate_basic_sketch(image, min_level, max_level, gamma):
    blur_filter = ImageFilter.GaussianBlur(radius=8)
    foreground = image.filter(blur_filter)          # apply gaussian blur
    foreground = ImageOps.invert(foreground)        # invert colors
    image = Image.blend(foreground, image, 0.5)     # blend with 50% opacity (should show the outlines)
    leveller = Level(min_level, max_level, gamma)   # clamp color levels
    data = [leveller.level_values(data) for data in image.getdata()]
    image.putdata(data)
    return image

# Modified from https://github.com/ajalt/pyasciigen/blob/master/asciigen.py
def generate_art(path, min_level, max_level, gamma, given_width=None,
                 brightness=None, contrast=None, html=False):
    font = ImageFont.load_default()
    char_width, char_height = font.getsize('X')

    def char_density(c, font=font):
        image = Image.new('1', font.getsize(c), color=255)
        draw = ImageDraw.Draw(image)
        draw.text((0, 0), c, font=font)
        return Counter(image.getdata())[0]      # count black pixels

    # sort the characters according to the pixel density of their render
    chars = sorted(string.letters + string.digits + string.punctuation + ' ',
                   key=char_density, reverse=True)

    image = Image.open(path)
    if image.format.lower() == 'png':
        bg = Image.new('RGB', image.size, (255, 255, 255))
        bg.paste(image)
        image = bg

    scale = 1
    width, height = image.size
    if given_width is None:
        given_width = min(width, 500)
    if given_width < width:
        scale = float(given_width) / width
        width = given_width

    if contrast is not None:
        image = ImageEnhance.Contrast(image).enhance(contrast)
    if brightness is not None:
        image = ImageEnhance.Brightness(image).enhance(brightness)

    # resize the image based on character size and aspect ratio
    height = int(height * scale * char_width / float(char_height))
    image = image.resize((width, height), Image.ANTIALIAS)
    # generate basic sketch to extract the necessary details
    image = generate_basic_sketch(image, min_level, max_level, gamma)
    pixels = image.convert('L').load()

    lines = []
    for y in xrange(height):
        s = ''.join(chars[int(pixels[x, y] / 255. * (len(chars) - 1) + 0.5)] for x in xrange(width))
        lines.append(cgi.escape(s) if html else s)

    return lines       # return HTML-escaped lines of chars.
