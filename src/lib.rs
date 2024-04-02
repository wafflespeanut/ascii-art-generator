#![feature(cell_update, const_fn_floating_point_arithmetic)]

use wasm_bindgen::prelude::*;

use std::cell::Cell;
use std::rc::Rc;

/// `println!`-like macro for JS `console.log`
macro_rules! console_log {
    ($($arg:tt)*) => (crate::js_log_simple(&std::fmt::format(format_args!($($arg)*))))
}

/// `document.getElementById`
macro_rules! get_elem_by_id {
    ($($foo:ident).* > $id:expr => $ty:ty) => {
        $($foo).*.get_element_by_id($id)
            .expect(&format!("cannot find {}", $id))
            .dyn_into::<$ty>()
            .map(std::rc::Rc::new)
    };
}

/// `document.querySelector`
macro_rules! query_selector {
    ($($foo:ident).* > $rule:expr => $ty:ty) => {
        $($foo).*.query_selector($rule)?
            .expect(&format!("no items match {}", $rule))
            .dyn_into::<$ty>()
            .map(std::rc::Rc::new)
    };
}

mod art;
mod dom;
mod utils;
include!(concat!(env!("OUT_DIR"), "/demo_output.rs"));

pub use self::art::AsciiArtGenerator;
pub use self::dom::{DomAsciiArtInjector, TimingEventKeeper};

use self::art::{DEFAULT_GAMMA, DEFAULT_MAX_LEVEL, DEFAULT_MIN_LEVEL};

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    utils::set_panic_hook();
    let injector = DomAsciiArtInjector::init();
    let search_str = injector.window.location().search()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search_str)?;
    let content = query_selector!(injector.document > ".outline" => web_sys::Element)?;

    if let Some(url) = params.get("url") {
        content.class_list().add_1("remove")?;

        return injector.inject_from_url(
            &url,
            "art-box",
            params.get("min").and_then(|v| v.parse().ok()),
            params.get("max").and_then(|v| v.parse().ok()),
            params.get("gamma").and_then(|v| v.parse().ok()),
            params.get("width").and_then(|v| v.parse().ok()),
            50,
            |draw: Box<dyn FnOnce() + 'static>| {
                draw();

                Ok(())
            },
        );
    }

    injector.inject_from_data("header-box", &DEMO_DATA)?;
    display_success(&injector.document)?;

    let (k, o) = (injector.keeper.clone(), content.clone());
    // Currently, image resizing takes an awful lot of time for huge images.
    // `image` doesn't use SIMD, and we can't use rayon in wasm.
    injector.inject_on_file_loads(
        "file-thingy",  // input element
        "art-box",      // art <pre> element
        "progress-box", // progress element
        50,             // step timeout
        move |draw: Box<dyn FnOnce() + 'static>| {
            let list = o.class_list();
            // If we've already shown the contents, then we're done.
            if list.contains("show") {
                return Ok(draw());
            }

            list.add_1("show")?;
            let inner_k = k.clone();
            k.borrow_mut().add(
                move || {
                    let _ = inner_k; // move keeper to avoid cancelling timeouts.
                    draw();
                },
                1000,
            );

            Ok(())
        },
    )?;

    set_listeners(&injector.document, content)
}

// FIXME: Need to clean this up!

fn set_listeners(
    doc: &Rc<web_sys::Document>,
    content: Rc<web_sys::Element>,
) -> Result<(), JsValue> {
    // Add listeners to change value whenever the range input is changed.
    let inputs = doc.query_selector_all("#art-params > .range-slider > .range")?;
    (0..inputs.length())
        .filter_map(|i| inputs.get(i))
        .try_for_each(|node| -> Result<(), JsValue> {
            let input = node.dyn_into::<web_sys::HtmlInputElement>().map(Rc::new)?;
            // Whenever a slider is changed, we need to update the relevant spans.
            let i = input.clone();
            let f = move || {
                let value = i.value();
                let n = i
                    .next_sibling()
                    .and_then(|n| n.next_sibling())
                    .expect("no slider value?")
                    .dyn_into::<web_sys::Node>()
                    .expect("casting span?");
                n.set_text_content(Some(&value));
            };

            let wrapped =
                Closure::wrap(Box::new(move |_: web_sys::Event| f()) as Box<dyn FnMut(_)>);
            input.set_oninput(Some(wrapped.as_ref().unchecked_ref()));
            wrapped.forget();
            Ok(())
        })?;

    let (min, max, gamma) = (
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0.0)),
    );

    let f_inp = get_elem_by_id!(doc > "file-thingy" => web_sys::HtmlInputElement)?;
    let reset_button = query_selector!(doc > "#art-params #reset" => web_sys::EventTarget)?;
    let min_inp = query_selector!(doc > "#min-level > .range" => web_sys::HtmlInputElement)?;
    let max_inp = query_selector!(doc > "#max-level > .range" => web_sys::HtmlInputElement)?;
    let gamma_inp = query_selector!(doc > "#gamma > .range" => web_sys::HtmlInputElement)?;

    let (mi_in, mx_in, g_in) = (min_inp.clone(), max_inp.clone(), gamma_inp.clone());
    let reset = move || {
        for &(e, v) in &[
            (&mi_in, DEFAULT_MIN_LEVEL as f64),
            (&mx_in, DEFAULT_MAX_LEVEL as f64),
            (&g_in, DEFAULT_GAMMA as f64),
        ] {
            e.set_value_as_number(v);
            let ev = web_sys::Event::new("input").expect("creating reset event");
            e.dispatch_event(&ev).expect("dispatching reset event");
        }
    };

    reset(); // initial slider reset to defaults

    let change_button = query_selector!(doc > "#art-params #change" => web_sys::EventTarget)?;

    let emit = move || {
        let (mi, mx, g) = (
            min_inp.value_as_number() as u8,
            max_inp.value_as_number() as u8,
            gamma_inp.value_as_number() as f32,
        );

        let mut changed = false; // check if any parameter has changed and is valid.
        changed |= mi != min.get() && (0..=255).contains(&mi);
        changed |= mx != max.get() && (0..=255).contains(&mx);
        changed |= g != gamma.get() && (0.0..=1.0).contains(&g);

        if changed {
            // If something's changed, emit a change event at the input.
            min.set(mi);
            max.set(mx);
            gamma.set(g);

            let list = content.class_list();
            // If we've already shown the contents, then hide it.
            if list.contains("show") {
                list.remove_1("show").expect("removing class?");
            }

            let event = web_sys::Event::new("change").expect("creating event");
            f_inp.dispatch_event(&event).expect("dispatching event");
        } else {
            console_log!("Nothing to do.");
        }
    };

    emit(); // initial sync of slider spans with slider values.

    let e = emit.clone();
    let f = Closure::wrap(Box::new(move |_: web_sys::Event| {
        reset();
        e(); // also emit during reset.
    }) as Box<dyn FnMut(_)>);
    reset_button.add_event_listener_with_callback("click", f.as_ref().unchecked_ref())?;
    f.forget();

    let f = Closure::wrap(Box::new(move |_: web_sys::Event| emit()) as Box<dyn FnMut(_)>);
    change_button.add_event_listener_with_callback("click", f.as_ref().unchecked_ref())?;
    f.forget();

    Ok(())
}

fn display_success(doc: &web_sys::Document) -> Result<(), JsValue> {
    let banner = query_selector!(doc > ".success-banner" => web_sys::Element)?;
    let list = banner.class_list();
    list.add_1("show")
}

/* FFI */

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout_simple(closure: &Closure<dyn FnMut()>, timeout_ms: i32) -> i32;

    #[wasm_bindgen(js_name = clearTimeout)]
    fn clear_timeout(id: i32);

    #[wasm_bindgen(js_name = setInterval)]
    fn set_interval_simple(closure: &Closure<dyn FnMut()>, interval_ms: i32) -> i32;

    #[wasm_bindgen(js_name = clearInterval)]
    fn clear_interval(id: i32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn js_log_simple(s: &str);
}
