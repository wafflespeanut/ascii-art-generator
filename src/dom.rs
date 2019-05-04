use crate::art::AsciiArtGenerator;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use std::rc::Rc;

/// A thing for reading files and injecting the art.
pub struct DomAsciiArtInjector {
    window: Rc<web_sys::Window>,
    document: Rc<web_sys::Document>,
    input: Rc<web_sys::HtmlInputElement>,
    pre: Rc<web_sys::HtmlPreElement>,
    reader: Rc<web_sys::FileReader>,
}

impl DomAsciiArtInjector {
    /// Initialize this injector with the IDs of `<pre>` element (for injecting art)
    /// and `<input>` element for subscribing to file loads.
    pub fn init(pre_elem_id: &str, input_elem_id: &str) -> Result<Self, JsValue> {
        let window = web_sys::window().map(Rc::new).expect("getting window");
        let document = window.document().map(Rc::new).expect("getting document");
        let input = document
            .get_element_by_id(input_elem_id)
            .expect("missing input element?")
            .dyn_into::<web_sys::HtmlInputElement>()
            .map(Rc::new)?;
        input.set_value(""); // reset input element

        Ok(DomAsciiArtInjector {
            window,
            pre: document
                .get_element_by_id(pre_elem_id)
                .expect("missing pre element?")
                .dyn_into::<web_sys::HtmlPreElement>()
                .map(Rc::new)?,
            input,
            document,
            reader: web_sys::FileReader::new().map(Rc::new)?,
        })
    }

    /// Adds an event listener to watch and update the `<pre>` element
    /// whenever a file is loaded.
    pub fn subscribe_to_file_loads(&self) -> Result<(), JsValue> {
        let (document, reader, _input, pre) = (
            self.document.clone(),
            self.reader.clone(),
            self.input.clone(),
            self.pre.clone(),
        );
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let value = reader.result().expect("reading complete but no result?");
            let buffer = Uint8Array::new(&value);
            let len = buffer.length();
            console_log!("Bytes read: {}", len);

            let mut bytes = vec![0; len as usize];
            buffer.copy_to(&mut bytes);

            pre.set_inner_html(""); // reset <pre> element
            let gen = AsciiArtGenerator::from_bytes(&bytes).expect("failed to load image.");
            console_log!("Approx. final image size: {} x {}", gen.width, gen.height);

            for text in gen.generate() {
                let div = document
                    .create_element("div")
                    .expect("creating div element")
                    .dyn_into::<web_sys::HtmlElement>()
                    .expect("casting element?");
                div.set_inner_text(&text);
                pre.append_child(&div).expect("appending div element");
            }
        }) as Box<FnMut(_)>);

        self.reader
            .set_onload(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        Ok(())
    }

    /// Adds event listener for reading files.
    pub fn add_file_listener(&self) -> Result<(), JsValue> {
        let (input, reader) = (self.input.clone(), self.reader.clone());
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let file = match input
                .files()
                .and_then(|l| l.get(l.length().saturating_sub(1)))
            {
                Some(f) => f.slice().expect("failed to get blob"),
                None => panic!("change event triggered for no files?"),
            };

            reader
                .read_as_array_buffer(&file)
                .expect("failed to read file");
        }) as Box<FnMut(_)>);

        self.input
            .set_onchange(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        Ok(())
    }
}
