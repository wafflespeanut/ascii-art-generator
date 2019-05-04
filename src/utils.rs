/// Sets panic hook for debugging.
///
/// Available only when `console_error_panic_hook` feature is enabled.
pub(crate) fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/* RGB <-> HSV conversion impl based on Python `colorsys` module. */

/// Converts an RGB pixel value in [0, 1] to HSV.
pub(crate) fn convert_rgb_to_hsv((r, g, b): (f32, f32, f32)) -> (f32, f32, f32) {
    let max = max(r, max(g, b));
    let min = min(r, min(g, b));
    let v = max;
    if min == max {
        return (0.0, 0.0, v);
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

    return (h / 6.0, s, v);
}

/// Converts a HSV pixel value to RGB (in range [0, 1]).
pub(crate) fn convert_hsv_to_rgb((h, s, v): (f32, f32, f32)) -> (f32, f32, f32) {
    if s == 0.0 {
        return (v, v, v);
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
        _ => unreachable!("bleh?!?@!"),
    }
}

/* min/max workaround for floats */

#[inline]
fn max(v1: f32, v2: f32) -> f32 {
    if v1 > v2 {
        v1
    } else {
        v2
    }
}

#[inline]
fn min(v1: f32, v2: f32) -> f32 {
    if v1 < v2 {
        v1
    } else {
        v2
    }
}
