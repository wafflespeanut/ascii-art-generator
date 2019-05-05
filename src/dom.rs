use crate::art::AsciiArtGenerator;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use std::rc::Rc;

/// A thing for reading files and injecting the art.
pub struct DomAsciiArtInjector {
    pub window: Rc<web_sys::Window>,
    pub document: Rc<web_sys::Document>,
}

impl DomAsciiArtInjector {
    /// Initialize this injector with the IDs of `<pre>` element (for injecting art)
    /// and `<input>` element for subscribing to file loads.
    pub fn init() -> Result<Self, JsValue> {
        let window = web_sys::window().map(Rc::new).expect("getting window");
        let document = window.document().map(Rc::new).expect("getting document");

        Ok(DomAsciiArtInjector { window, document })
    }

    /// Inject into the `<pre>` element matching the given ID using the given image data.
    pub fn inject_from_data(&self, pre_elem_id: &str, buffer: &[u8]) -> Result<(), JsValue> {
        let pre = self.get_element_by_id::<web_sys::HtmlPreElement>(pre_elem_id)?;
        Self::inject_from_data_using_document(buffer, &self.document, &pre)
    }

    /// Adds an event listener to watch and update the `<pre>` element
    /// whenever a file is loaded.
    pub fn inject_on_file_loads(
        &self,
        input_elem_id: &str,
        pre_elem_id: &str,
    ) -> Result<(), JsValue> {
        let reader = web_sys::FileReader::new().map(Rc::new)?;

        let pre = self.get_element_by_id::<web_sys::HtmlPreElement>(pre_elem_id)?;
        let input = self
            .get_element_by_id::<web_sys::HtmlInputElement>(input_elem_id)
            .map(Rc::new)?;
        input.set_value(""); // reset input element

        let (r, doc) = (reader.clone(), self.document.clone());
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let value = r.result().expect("reading complete but no result?");
            let buffer = Uint8Array::new(&value);
            let mut bytes = vec![0; buffer.length() as usize];
            buffer.copy_to(&mut bytes);
            Self::inject_from_data_using_document(&bytes, &doc, &pre).expect("failed to inject")
        }) as Box<FnMut(_)>);

        reader.set_onload(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        self.add_file_listener(input, reader)
    }

    /// Adds event listener for reading files.
    fn add_file_listener(
        &self,
        input: Rc<web_sys::HtmlInputElement>,
        reader: Rc<web_sys::FileReader>,
    ) -> Result<(), JsValue> {
        let inp = input.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            let file = match inp
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

        input.set_onchange(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        Ok(())
    }

    /// Abstraction for `document.getElementById`
    fn get_element_by_id<T>(&self, id: &str) -> Result<T, web_sys::Element>
    where
        T: JsCast,
    {
        self.document
            .get_element_by_id(id)
            .expect("missing element?")
            .dyn_into::<T>()
    }

    /// Gets image data from buffer, generates ASCII art and injects into `<pre>` element.
    fn inject_from_data_using_document(
        buffer: &[u8],
        doc: &web_sys::Document,
        pre: &web_sys::HtmlPreElement,
    ) -> Result<(), JsValue> {
        console_log!("Image size: {} bytes", buffer.len());

        pre.set_inner_html(""); // reset <pre> element
        let gen = AsciiArtGenerator::from_bytes(&buffer).expect("failed to load image.");
        console_log!(
            "Approx. final image dimensions: {} x {}",
            gen.width,
            gen.height
        );

        for text in gen.generate() {
            let div = doc
                .create_element("div")?
                .dyn_into::<web_sys::HtmlElement>()?;
            div.set_inner_text(&text);
            pre.append_child(&div)?;
        }

        Ok(())
    }
}
