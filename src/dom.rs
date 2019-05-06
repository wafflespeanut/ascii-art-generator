use crate::art::AsciiArtGenerator;

use image::{DynamicImage, GenericImageView};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use std::cell::{Cell, RefCell};
use std::cmp;
use std::rc::Rc;

const THUMB_HEIGHT: u32 = 100;

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
        let pre = self
            .get_element_by_id::<web_sys::HtmlPreElement>(pre_elem_id)
            .map(Rc::new)?;
        Self::inject_from_data_using_document(buffer, &self.document, &pre, 0, |_| Ok(()));
        Ok(())
    }

    /// Adds an event listener to watch and update the `<pre>` element
    /// whenever a file is loaded.
    pub fn inject_on_file_loads(
        &self,
        input_elem_id: &str,
        pre_elem_id: &str,
        progress_elem_id: &str,
        timeout_ms: u32,
    ) -> Result<(), JsValue> {
        // Setup the stage.
        let reader = web_sys::FileReader::new().map(Rc::new)?;
        let pre = self
            .get_element_by_id::<web_sys::HtmlPreElement>(pre_elem_id)
            .map(Rc::new)?;
        let prog = self
            .get_element_by_id::<web_sys::Element>(progress_elem_id)
            .map(Rc::new)?;
        let input = self
            .get_element_by_id::<web_sys::HtmlInputElement>(input_elem_id)
            .map(Rc::new)?;
        input.set_value(""); // reset input element

        {
            let (r, doc) = (reader.clone(), self.document.clone());
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                prog.set_inner_html("");
                let value = r.result().expect("reading complete but no result?");
                let buffer = Uint8Array::new(&value);
                let mut bytes = vec![0; buffer.length() as usize];
                buffer.copy_to(&mut bytes);

                let (doc, prog) = (doc.clone(), prog.clone());
                Self::inject_from_data_using_document(
                    &bytes,
                    &doc.clone(),
                    &pre,
                    timeout_ms,
                    Box::new(move |img: &DynamicImage| -> Result<(), JsValue> {
                        // Whenever we get an image, resize it to a thumbnail.
                        let new_h = cmp::min(img.height(), THUMB_HEIGHT);
                        let new_w =
                            (new_h as f32 * img.width() as f32 / img.height() as f32) as u32;
                        let img = img.resize_exact(new_w, new_h, image::FilterType::Lanczos3);
                        let mut bytes = vec![];
                        img.write_to(&mut bytes, image::ImageFormat::JPEG)
                            .expect("invalid image?");

                        // Encode the image to base64 and append it to the document for preview.
                        let b64 = base64::encode(&bytes);
                        let img = doc
                            .create_element("img")?
                            .dyn_into::<web_sys::HtmlImageElement>()?;
                        img.set_src(&format!("data:image/jpeg;base64,{}", b64));
                        prog.append_child(&img)?;

                        Ok(())
                    }) as Box<_>,
                );
            }) as Box<FnMut(_)>);

            reader.set_onload(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        self.add_file_listener(input, reader)
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

    /// Gets image data from buffer, generates ASCII art and injects into `<pre>` element.
    /// Also takes a callback for invoking with the images created in each step.
    fn inject_from_data_using_document<F>(
        buffer: &[u8],
        doc: &Rc<web_sys::Document>,
        pre: &Rc<web_sys::HtmlPreElement>,
        step_timeout_ms: u32,
        mut callback: F,
    ) where
        F: FnMut(&DynamicImage) -> Result<(), JsValue> + 'static,
    {
        console_log!("Image size: {} bytes", buffer.len());

        pre.set_inner_html(""); // reset <pre> element
        let gen = AsciiArtGenerator::from_bytes(&buffer)
            .map(Rc::new)
            .expect("failed to load image.");

        let delay = Rc::new(Cell::new(step_timeout_ms));
        let keeper = TimeoutKeeper::new();

        // Callback hell begins!
        let (pre, doc, inner_d, inner_k) =
            (pre.clone(), doc.clone(), delay.clone(), keeper.clone());
        let f = move || {
            let proc = gen.processor();
            let img = proc.resize();
            callback(&img).expect("queueing resized image");

            let (outer_d, outer_k) = (inner_d.clone(), inner_k.clone());
            let f = move || {
                let proc = gen.processor();
                let fg = proc.blur_and_invert(&img);
                callback(&fg).expect("queueing blending image");

                let (outer_d, outer_k) = (inner_d.clone(), inner_k.clone());
                let f = move || {
                    let proc = gen.processor();
                    let final_img = proc.blend_and_adjust(&img, &fg);
                    callback(&final_img).expect("queueing final image");

                    let (outer_d, outer_k) = (inner_d.clone(), inner_k.clone());
                    let f = move || {
                        // Move the timeout keeper inside to prevent clearing all timeouts.
                        let _keeper = inner_k.clone();
                        let proc = gen.processor();
                        for text in proc.generate_from_img(&final_img) {
                            let div = doc
                                .create_element("div")
                                .expect("creating art element")
                                .dyn_into::<web_sys::HtmlElement>()
                                .expect("casting created element");
                            div.set_inner_text(&text);
                            pre.append_child(&div).expect("appending div");
                        }
                    };

                    outer_k
                        .borrow_mut()
                        .add(f, outer_d.update(|x| x + step_timeout_ms));
                };

                outer_k
                    .borrow_mut()
                    .add(f, outer_d.update(|x| x + step_timeout_ms));
            };

            outer_k
                .borrow_mut()
                .add(f, outer_d.update(|x| x + step_timeout_ms));
        };

        keeper.borrow_mut().add(f, delay.get());
    }
}

/// Abstraction for keeping track of timeouts. This takes `FnOnce` thingies for
/// registering the timeouts and clears them when it goes out of scope.
struct TimeoutKeeper {
    stuff: Vec<(i32, Closure<FnMut()>)>,
}

impl TimeoutKeeper {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(TimeoutKeeper { stuff: vec![] }))
    }

    fn add<F>(&mut self, f: F, timeout_ms: u32)
    where
        F: FnOnce() + 'static,
    {
        let f = Closure::once(Box::new(f) as Box<FnOnce()>);
        let id = crate::set_timeout_simple(&f, timeout_ms as i32);
        self.stuff.push((id, f));
    }
}

impl Drop for TimeoutKeeper {
    fn drop(&mut self) {
        self.stuff.drain(..).for_each(|(id, _)| {
            crate::clear_timeout(id);
        });
    }
}
