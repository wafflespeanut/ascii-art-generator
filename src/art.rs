use crate::utils;
use image::{DynamicImage, FilterType, GenericImageView, ImageError, RgbImage};

use std::cmp;

const BLEND_RATIO: f32 = 0.5;

const DEFAULT_MIN_LEVEL: u8 = 78;
const DEFAULT_MAX_LEVEL: u8 = 125;
const DEFAULT_GAMMA: f32 = 0.78;
const MAX_WIDTH: u32 = 500;

/* Constants below are obtained using Python. See https://github.com/wafflespeanut/ascii-art-generator/blob/0b519b00b43eadb8500db30c304b2b87ad7eb159/src/gen.py#L27-L39 */

// Char width and height based on system fonts.
const DEFAULT_CHAR_WIDTH: f32 = 6.0;
const DEFAULT_CHAR_HEIGHT: f32 = 11.0;
// Characters sorted based on the pixel density of their render.
const CHARS: &[char] = &[
    'H', '$', 'd', 'g', 'q', '0', 'p', 'R', '8', 'b', 'h', 'k', 'B', 'D', 'N', 'Q', 'U', '5', '6',
    '9', '@', 'A', 'K', 'y', 'E', 'G', 'O', 'Z', '2', '4', '#', 'a', 'f', 'u', 'M', 'P', 'S', '3',
    '%', 'l', 't', 'x', 'W', 'X', 'Y', '1', '&', 'j', 'n', 's', 'z', 'C', '7', 'e', 'i', 'm', 'o',
    'w', 'F', 'L', 'T', 'V', '[', ']', 'r', 'J', 'c', 'I', '{', '}', 'v', '(', ')', '?', '!', '<',
    '>', '*', '+', '/', '=', '\\', '^', '|', '"', ';', '_', '~', '-', '\'', ',', ':', '`', '.',
    ' ',
];

/// This project - the whole deal.
pub struct AsciiArtGenerator {
    pub min_level: u8,
    pub max_level: u8,
    pub gamma: f32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    img: DynamicImage,
    ar: f32,
}

impl AsciiArtGenerator {
    /// Creates an instance from the given buffer.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes)?;
        let (w, h) = (img.width(), img.height());
        let clamped_width = cmp::min(w, MAX_WIDTH);

        let mut gen = AsciiArtGenerator {
            min_level: DEFAULT_MIN_LEVEL,
            max_level: DEFAULT_MAX_LEVEL,
            gamma: DEFAULT_GAMMA,

            img,
            width: w,
            height: h,
            ar: w as f32 / h as f32,
        };

        if clamped_width < w {
            gen.set_width(MAX_WIDTH);
        }

        Ok(gen)
    }

    /// Sets the width of the final image and returns the new height.
    ///
    /// **NOTE:**
    /// - No-op if the width is greater than the actual width of the image.
    /// - This also affects the height to maintain aspect ratio.
    /// - This only stores the dimensions - scaling is done while generating the art.
    /// - The image will be resized once again to match character widths and heights,
    /// but will be closer to this value.
    pub fn set_width(&mut self, width: u32) -> u32 {
        if width >= self.img.width() {
            return self.height;
        }

        self.width = width;
        self.height = (width as f32 / self.ar) as u32;
        self.height
    }

    /// Sets the height of the final image and returns the new width.
    ///
    /// **NOTE:**
    /// - No-op if the height is greater than the actual height of the image.
    /// - This also affects the width to maintain aspect ratio.
    /// - This only stores the dimensions - scaling is done while generating the art.
    /// - The height of the image will probably change later to fit the character
    /// widths and heights.
    pub fn set_height(&mut self, height: u32) -> u32 {
        let actual = self.img.height();
        if height >= actual {
            return self.width;
        }

        self.height = height;
        self.width = (height as f32 * self.ar) as u32;
        self.width
    }

    /// Let the artwork begin!
    ///
    /// See https://blog.waffles.space/2017/02/28/ascii-sketch/ for how it works.
    pub fn generate(&self) -> impl Iterator<Item = String> {
        let width = self.width;
        let height = (self.height as f32 * DEFAULT_CHAR_WIDTH / DEFAULT_CHAR_HEIGHT) as u32;
        let img = self.img.resize_exact(width, height, FilterType::Lanczos3);

        let mut foreground = img.blur(8.0);
        foreground.invert();

        let mut actual_buf = img.to_rgb();
        let fg_buf = foreground.to_rgb();
        self.blend_and_adjust_levels(&mut actual_buf, &fg_buf);

        let detailed = DynamicImage::ImageRgb8(actual_buf);
        let final_img = DynamicImage::ImageLuma8(detailed.to_luma());

        let multiplier = (CHARS.len() - 1) as f32;
        (0..height).map(move |y| {
            (0..width)
                .map(|x| {
                    let p = final_img.get_pixel(x, y).data[0] as f32 / 255.0;
                    CHARS[(p * multiplier + 0.5) as usize]
                })
                .collect()
        })
    }

    fn blend_and_adjust_levels(&self, actual_buf: &mut RgbImage, fg_buf: &RgbImage) {
        let (min, max, inv_gamma) = (
            self.min_level as f32 / 255.0,
            self.max_level as f32 / 255.0,
            1.0 / self.gamma,
        );

        actual_buf
            .pixels_mut()
            .zip(fg_buf.pixels())
            .for_each(|(p1, p2)| {
                let r = blend_pixel(p1[0], p2[0], BLEND_RATIO);
                let g = blend_pixel(p1[1], p2[1], BLEND_RATIO);
                let b = blend_pixel(p1[2], p2[2], BLEND_RATIO);

                let (h, s, mut v) = utils::convert_rgb_to_hsv((r, g, b));
                if v <= min {
                    v = 0.0;
                } else if v >= max {
                    v = 1.0;
                } else {
                    v = ((v - min) / (max - min)).powf(inv_gamma);
                }

                let (r, g, b) = utils::convert_hsv_to_rgb((h, s, v));
                p1.data = [
                    (r * 255.0).round() as u8,
                    (g * 255.0).round() as u8,
                    (b * 255.0).round() as u8,
                ];
            });
    }
}

/// Blends a pixel value using the given ratio and returns the normalized value in [0, 1]
#[inline]
const fn blend_pixel(p1: u8, p2: u8, ratio: f32) -> f32 {
    (p1 as f32 * (1.0 - ratio) + p2 as f32 * ratio) / 255.0
}
