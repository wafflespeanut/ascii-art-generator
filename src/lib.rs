#![feature(const_fn)]

use wasm_bindgen::prelude::*;

/// `println!`-like macro for JS `console.log`
macro_rules! console_log {
    ($($arg:tt)*) => (crate::js_log(&std::fmt::format(format_args!($($arg)*))))
}

mod art;
mod dom;
mod utils;

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

    let injector = DomAsciiArtInjector::init("art-box", "file-thingy")?;
    injector.subscribe_to_file_loads()?;
    injector.add_file_listener()?;

    Ok(())
}
