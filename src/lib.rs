#![feature(const_fn)]

mod utils;

use image::{DynamicImage, FilterType, GenericImageView};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use js_sys::Uint8Array;

use std::cmp;
use std::rc::Rc;

const BLEND_RATIO: f32 = 0.5;
const DEFAULT_MIN_LEVEL: u8 = 78;
const DEFAULT_MAX_LEVEL: u8 = 125;
const DEFAULT_GAMMA: f32 = 0.78;
const MAX_WIDTH: u32 = 500;

/* Parameters below are obtained using Python. See https://github.com/wafflespeanut/ascii-art-generator/blob/0b519b00b43eadb8500db30c304b2b87ad7eb159/src/gen.py#L27-L39 */

// Char width and height based on system fonts.
const DEFAULT_CHAR_WIDTH: f32 = 6.0;
const DEFAULT_CHAR_HEIGHT: f32 = 11.0;
// Characters sorted based on the pixel density of their render.
const CHARS: &[char] = &['H', '$', 'd', 'g', 'q', '0', 'p', 'R', '8', 'b', 'h', 'k', 'B', 'D', 'N', 'Q', 'U', '5', '6', '9', '@', 'A', 'K', 'y', 'E', 'G', 'O', 'Z', '2', '4', '#', 'a', 'f', 'u', 'M', 'P', 'S', '3', '%', 'l', 't', 'x', 'W', 'X', 'Y', '1', '&', 'j', 'n', 's', 'z', 'C', '7', 'e', 'i', 'm', 'o', 'w', 'F', 'L', 'T', 'V', '[', ']', 'r', 'J', 'c', 'I', '{', '}', 'v', '(', ')', '?', '!', '<', '>', '*', '+', '/', '=', '\\', '^', '|', '"', ';', '_', '~', '-', '\'', ',', ':', '`', '.', ' '];

#[inline]
const fn blend_pixel(p1: u8, p2: u8, ratio: f32) -> f32 {
    (p1 as f32 * (1.0 - ratio) + p2 as f32 * ratio) / 255.0
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn js_log(s: &str);
}

macro_rules! console_log {
    ($($arg:tt)*) => (js_log(&std::fmt::format(format_args!($($arg)*))))
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let window = web_sys::window().expect("getting window");
    let document = window.document().map(Rc::new).expect("getting document");
    // let body = document.body().expect("getting body");

    let art_elem = document.get_element_by_id("art-box")
        .expect("pre element?")
        .dyn_into::<web_sys::HtmlPreElement>()?;

    let input = document.get_element_by_id("file-thingy")
        .expect("file input element?")
        .dyn_into::<web_sys::HtmlInputElement>()
        .map(Rc::new)?;
    input.set_value("");

    let reader = web_sys::FileReader::new().map(Rc::new)?;
    {
        let d = document.clone();
        let r = reader.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let value = r.result().expect("reading complete but no result?");
            let buffer = Uint8Array::new(&value);
            let len = buffer.length();
            console_log!("Bytes read: {}", len);

            let mut bytes = vec![0; len as usize];
            buffer.copy_to(&mut bytes);
            convert_and_add_ascii_to_element(&d, bytes, &art_elem).expect("error creating ascii art");
        }) as Box<dyn FnMut(_)>);

        reader.set_onload(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    {
        let i = input.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let file = match i.files().and_then(|l| l.get(0)) {
                Some(f) => f.slice().expect("failed to get blob"),
                None => {
                    console_log!("change event triggered for no files?");
                    return
                },
            };

            reader.read_as_array_buffer(&file).expect("failed to read file");
        }) as Box<dyn FnMut(_)>);

        input.set_onchange(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    Ok(())
}

fn convert_and_add_ascii_to_element(document: &web_sys::Document, bytes: Vec<u8>,
                                    art_element: &web_sys::HtmlPreElement)
                                    -> Result<(), JsValue>
{
    let img = image::load_from_memory(&bytes).expect("failed to load image");
    let (mut width, mut height) = (img.width(), img.height());
    console_log!("Loaded {} x {} image.", width, height);

    let clamped_width = cmp::min(width, MAX_WIDTH);
    let mut scale = 1.0;
    if clamped_width < width {
        scale = clamped_width as f32 / width as f32;
        width = clamped_width;
    }

    height = (height as f32 * scale * DEFAULT_CHAR_WIDTH / DEFAULT_CHAR_HEIGHT) as u32;
    let img = img.resize_exact(width, height, FilterType::Lanczos3);

    let mut foreground = img.blur(8.0);
    foreground.invert();

    let mut actual_buf = img.to_rgb();
    let fg_buf = foreground.to_rgb();
    let (min, max, gamma) = (DEFAULT_MIN_LEVEL, DEFAULT_MAX_LEVEL, DEFAULT_GAMMA);

    let (min, max, inv_gamma) = (min as f32 / 255.0, max as f32 / 255.0, 1.0 / gamma);
    actual_buf.pixels_mut().zip(fg_buf.pixels()).for_each(|(p1, p2)| {
        let r = blend_pixel(p1[0], p2[0], BLEND_RATIO);
        let g = blend_pixel(p1[1], p2[1], BLEND_RATIO);
        let b = blend_pixel(p1[2], p2[2], BLEND_RATIO);

        let (h, s, mut v) = rgb_to_hsv((r, g, b));
        if v <= min {
            v = 0.0;
        } else if v >= max {
            v = 1.0;
        } else {
            v = ((v - min) / (max - min)).powf(inv_gamma);
        }

        let (r, g, b) = hsv_to_rgb((h, s, v));
        p1.data = [
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        ];
    });

    let detailed = DynamicImage::ImageRgb8(actual_buf);
    let final_img = DynamicImage::ImageLuma8(detailed.to_luma());

    let multiplier = (CHARS.len() - 1) as f32;
    for y in 0..height {
        let text = (0..width).map(|x| {
            let p = final_img.get_pixel(x, y).data[0] as f32 / 255.0;
            CHARS[(p * multiplier + 0.5) as usize]
        }).collect::<String>();

        let div = document.create_element("div")?
            .dyn_into::<web_sys::HtmlElement>()?;
        div.set_inner_text(&text);
        art_element.append_child(&div)?;
    }

    Ok(())
}

/* min/max workaround for floats */

#[inline]
fn max(v1: f32, v2: f32) -> f32 {
    if v1 > v2 { v1 } else { v2 }
}

#[inline]
fn min(v1: f32, v2: f32) -> f32 {
    if v1 < v2 { v1 } else { v2 }
}

/* RGB <-> HSV conversion impl based on Python `colorsys` module. */

fn rgb_to_hsv((r, g, b): (f32, f32, f32)) -> (f32, f32, f32) {
    let max = max(r, max(g, b));
    let min = min(r, min(g, b));
    let v = max;
    if min == max {
        return (0.0, 0.0, v)
    }

    let s = (max - min) / max;
    let r = (max - r) / (max - min);
    let g = (max - g) / (max - min);
    let b = (max - b) / (max - min);
    let h = if r == max {
        b - g
    } else if g == max {
        2.0 + r - b
    } else {
        4.0 + g - r
    };

    return (h / 6.0, s, v)
}

fn hsv_to_rgb((h, s, v): (f32, f32, f32)) -> (f32, f32, f32) {
    if s == 0.0 {
        return (v, v, v)
    }

    let i = (h * 6.0) as u8;
    let f = (h * 6.0) - (h * 6.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        5 => (v, p, q),
        _ => unreachable!(),
    }
}
