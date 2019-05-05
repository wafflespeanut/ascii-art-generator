#![feature(const_fn)]

use wasm_bindgen::prelude::*;

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
    injector.inject_on_file_loads("file-thingy", "art-box")?;

    Ok(())
}
