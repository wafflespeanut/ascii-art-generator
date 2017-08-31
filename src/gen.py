from collections import Counter
from PIL import Image, ImageFilter, ImageFont, ImageDraw, ImageEnhance, ImageOps

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


# Modified from https://github.com/ajalt/pyasciigen/blob/master/asciigen.py
class Sketch(object):
    def __init__(self):
        font = ImageFont.load_default()

        def char_density(c, font=font):
            image = Image.new('1', font.getsize(c), color=255)
            draw = ImageDraw.Draw(image)
            draw.text((0, 0), c, font=font)
            return Counter(image.getdata())[0]      # count black pixels

        self.char_width, self.char_height = font.getsize('X')
        # sort the characters according to the pixel density of their render
        chars = string.letters + string.digits + string.punctuation + ' '
        self.chars = sorted(chars, key=char_density, reverse=True)

    # Generate the sketch from an Image object.
    def generate_basic_sketch(self, image, min_level, max_level, gamma):
        blur_filter = ImageFilter.GaussianBlur(radius=8)
        foreground = image.filter(blur_filter)          # apply gaussian blur
        foreground = ImageOps.invert(foreground)        # invert colors
        image = Image.blend(foreground, image, 0.5)     # blend with 50% opacity (should show the outlines)
        leveller = Level(min_level, max_level, gamma)   # clamp color levels
        data = [leveller.level_values(data) for data in image.getdata()]
        image.putdata(data)
        return image

    # Generate ASCII line by line from an Image object.
    def generate_ascii(self, image, min_level, max_level, gamma,
                       required_width=None, brightness=None, contrast=None):
        if image.format.lower() == 'png':   # destroy transparency for PNG
            bg = Image.new('RGB', image.size, (255, 255, 255))
            bg.paste(image)
            image = bg

        scale = 1
        width, height = image.size
        if required_width is None:
            required_width = min(width, 500)
        if required_width < width:
            scale = float(required_width) / width
            width = required_width

        if contrast is not None:
            image = ImageEnhance.Contrast(image).enhance(contrast)
        if brightness is not None:
            image = ImageEnhance.Brightness(image).enhance(brightness)

        # resize the image based on character size and aspect ratio
        height = int(height * scale * self.char_width / float(self.char_height))
        image = image.resize((width, height), Image.ANTIALIAS)
        image = self.generate_basic_sketch(image, min_level, max_level, gamma)
        pixels = image.convert('L').load()

        for y in xrange(height):
            yield ''.join(self.chars[int(pixels[x, y] / 255. * (len(self.chars) - 1) + 0.5)] \
                          for x in xrange(width))
