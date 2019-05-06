#![feature(const_fn)]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// `println!`-like macro for JS `console.log`
macro_rules! console_log {
    ($($arg:tt)*) => (crate::js_log(&std::fmt::format(format_args!($($arg)*))))
}

mod art;
mod dom;
mod utils;
include!(concat!(env!("OUT_DIR"), "/demo_output.rs"));

pub use self::art::AsciiArtGenerator;
pub use self::dom::DomAsciiArtInjector;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn js_log(s: &str);
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let injector = DomAsciiArtInjector::init()?;
    injector.inject_from_data("header-box", &DEMO_DATA)?;
    display_success(&injector.document)?;
    injector.inject_on_file_loads("file-thingy", "art-box")?;

    Ok(())
}

fn display_success(doc: &web_sys::Document) -> Result<(), JsValue> {
    let banner = match doc.query_selector(".success-banner")? {
        Some(e) => e.dyn_into::<web_sys::Element>()?,
        None => {
            console_log!("Missing success banner?");
            return Ok(());
        }
    };

    let list = banner.class_list();
    list.add_1("show")
}
