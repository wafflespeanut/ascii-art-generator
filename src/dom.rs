use crate::art::AsciiArtGenerator;

use base64::prelude::*;
use image::DynamicImage;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use std::cell::{Cell, RefCell};
use std::cmp;
use std::io::{BufWriter, Cursor};
use std::rc::Rc;

const THUMB_HEIGHT: u32 = 50;

/// A thing for reading files and injecting the art.
pub struct DomAsciiArtInjector {
    pub window: Rc<web_sys::Window>,
    pub document: Rc<web_sys::Document>,
    pub keeper: Rc<RefCell<TimingEventKeeper>>,
}

impl DomAsciiArtInjector {
    /// Initialize this injector with the IDs of `<pre>` element (for injecting art)
    /// and `<input>` element for subscribing to file loads.
    pub fn init() -> Self {
        let window = web_sys::window().map(Rc::new).expect("getting window");
        let document = window.document().map(Rc::new).expect("getting document");

        DomAsciiArtInjector {
            window,
            document,
            keeper: TimingEventKeeper::new(),
        }
    }

    /// Inject into the `<pre>` element matching the given ID using the given image data.
    pub fn inject_from_data(&self, pre_elem_id: &str, buffer: &[u8]) -> Result<(), JsValue> {
        let pre = get_elem_by_id!(self.document > pre_elem_id => web_sys::HtmlPreElement)?;
        let gen = AsciiArtGenerator::from_bytes(buffer)
            .map(Rc::new)
            .expect("failed to load demo.");
        Self::inject_from_data_using_document(
            gen,
            &self.document,
            &self.keeper,
            &pre,
            0,
            |_| Ok(()),
            |draw| {
                console_log!("Yay!");
                draw();
                Ok(())
            },
        );
        Ok(())
    }

    /// Downloads image from the given URL and updates the `<pre>` element.
    pub fn inject_from_url<F>(
        &self,
        url: &str,
        pre_elem_id: &str,
        min: Option<u8>,
        max: Option<u8>,
        gamma: Option<f32>,
        width: Option<u32>,
        timeout_ms: u32,
        final_callback: F,
    ) -> Result<(), JsValue>
    where
        F: Fn(Box<dyn FnOnce() + 'static>) -> Result<(), JsValue> + Clone + 'static,
    {
        let pre = get_elem_by_id!(self.document > pre_elem_id => web_sys::HtmlPreElement)?;

        let xhr = web_sys::XmlHttpRequest::new().map(Rc::new)?;
        xhr.open("GET", url)?;
        xhr.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);

        let (x, d, k) = (xhr.clone(), self.document.clone(), self.keeper.clone());
        let download = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if x.ready_state() != web_sys::XmlHttpRequest::DONE {
                console_log!("Ajax not ready yet.");
                return;
            }

            let status = x.status().expect("getting status");
            if status != 200 {
                console_log!("Error fetching image. Got {} status code.", status);
                return;
            }

            let value = x.response().expect("loading complete but no result?");
            let buffer = Uint8Array::new(&value);
            let mut bytes = vec![0; buffer.length() as usize];
            buffer.copy_to(&mut bytes);
            let gen = AsciiArtGenerator::from_bytes(&bytes)
                .map(|mut gen| {
                    if let Some(w) = width {
                        gen.set_width(w);
                    }

                    gen
                })
                .map(Rc::new)
                .expect("failed to load image.");

            if let Some(m) = min {
                gen.min_level.set(m);
            }

            if let Some(m) = max {
                gen.max_level.set(m);
            }

            if let Some(m) = gamma {
                gen.gamma.set(m);
            }

            console_log!("Loaded {} bytes", bytes.len());
            Self::inject_from_data_using_document(
                gen,
                &d,
                &k,
                &pre,
                timeout_ms,
                |_| -> Result<(), JsValue> { Ok(()) },
                final_callback.clone(),
            );
        }) as Box<dyn Fn(_)>);

        xhr.set_onload(Some(download.as_ref().unchecked_ref()));
        download.forget();
        xhr.send()?;

        Ok(())
    }

    /// Adds an event listener to watch and update the `<pre>` element
    /// whenever a file is loaded.
    pub fn inject_on_file_loads<F>(
        &self,
        input_elem_id: &str,
        pre_elem_id: &str,
        progress_elem_id: &str,
        timeout_ms: u32,
        final_callback: F,
    ) -> Result<(), JsValue>
    where
        F: Fn(Box<dyn FnOnce() + 'static>) -> Result<(), JsValue> + Clone + 'static,
    {
        // Setup the stage.
        let reader = web_sys::FileReader::new().map(Rc::new)?;
        let pre = get_elem_by_id!(self.document > pre_elem_id => web_sys::HtmlPreElement)?;
        let prog = get_elem_by_id!(self.document > progress_elem_id => web_sys::Element)?;
        let input = get_elem_by_id!(self.document > input_elem_id => web_sys::HtmlInputElement)?;
        input.set_value(""); // reset input element

        let min_inp =
            query_selector!(self.document > "#min-level > .range" => web_sys::HtmlInputElement)?;
        let max_inp =
            query_selector!(self.document > "#max-level > .range" => web_sys::HtmlInputElement)?;
        let gamma_inp =
            query_selector!(self.document > "#gamma > .range" => web_sys::HtmlInputElement)?;

        {
            let (r, k, doc) = (reader.clone(), self.keeper.clone(), self.document.clone());
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                // Something has changed. Reset progress and get new values and buffer.
                prog.set_inner_html("");
                let (min, max, gamma) = (
                    min_inp.value_as_number() as u8,
                    max_inp.value_as_number() as u8,
                    gamma_inp.value_as_number() as f32,
                );

                let value = r.result().expect("reading complete but no result?");
                let buffer = Uint8Array::new(&value);
                let mut bytes = vec![0; buffer.length() as usize];
                buffer.copy_to(&mut bytes);
                let gen = AsciiArtGenerator::from_bytes(&bytes)
                    .map(Rc::new)
                    .expect("failed to load image.");
                gen.min_level.set(min);
                gen.max_level.set(max);
                gen.gamma.set(gamma);

                console_log!("Loaded {} bytes", bytes.len());
                let (doc, prog) = (doc.clone(), prog.clone());
                Self::inject_from_data_using_document(
                    gen,
                    &doc.clone(),
                    &k,
                    &pre,
                    timeout_ms,
                    move |img: &DynamicImage| -> Result<(), JsValue> {
                        // Whenever we get an image, resize it to a thumbnail.
                        let new_h = cmp::min(img.height(), THUMB_HEIGHT);
                        let new_w =
                            (new_h as f32 * img.width() as f32 / img.height() as f32) as u32;
                        let img = img.resize_exact(new_w, new_h, image::imageops::Lanczos3);
                        let bytes = Cursor::new(vec![]);
                        let mut writer = BufWriter::new(bytes);
                        img.write_to(&mut writer, image::ImageFormat::Jpeg)
                            .expect("invalid image?");

                        // Encode the image to base64 and append it to the document for preview.
                        let b64 = BASE64_STANDARD.encode(
                            writer
                                .into_inner()
                                .expect("getting bytes from writer")
                                .into_inner(),
                        );
                        let img = doc
                            .create_element("img")?
                            .dyn_into::<web_sys::HtmlImageElement>()?;
                        img.set_src(&format!("data:image/jpeg;base64,{}", b64));
                        prog.append_child(&img)?;

                        Ok(())
                    },
                    final_callback.clone(),
                );
            }) as Box<dyn Fn(_)>);

            reader.set_onload(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

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
            console_log!("change event");
            let file = match inp
                .files()
                .and_then(|l| l.get(l.length().saturating_sub(1)))
            {
                Some(f) => f.slice().expect("failed to get blob"),
                None => return,
            };

            reader
                .read_as_array_buffer(&file)
                .expect("failed to read file");
        }) as Box<dyn Fn(_)>);

        input.set_onchange(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        Ok(())
    }

    /// Gets image data from buffer, generates ASCII art and injects into `<pre>` element.
    /// Each step produces an image, steps can be spaced by timeouts, and a callback is
    /// called after each step. Also takes a final callback for invoking the final draw.
    // NOTE: Yes, this is unnecessarily complicated, I know!
    fn inject_from_data_using_document<F, U>(
        gen: Rc<AsciiArtGenerator>,
        doc: &Rc<web_sys::Document>,
        keeper: &Rc<RefCell<TimingEventKeeper>>,
        pre: &Rc<web_sys::HtmlPreElement>,
        step_timeout_ms: u32,
        callback: F,
        final_callback: U,
    ) where
        F: Fn(&DynamicImage) -> Result<(), JsValue> + 'static,
        U: FnOnce(Box<dyn FnOnce() + 'static>) -> Result<(), JsValue> + 'static,
    {
        pre.set_inner_html(""); // reset <pre> element
        let delay = Rc::new(Cell::new(step_timeout_ms));

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
                        let draw = Box::new(move || {
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
                        }) as Box<_>;

                        final_callback(draw).expect("final callback")
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
/// registering the timeouts (`FnMut` thingies for intervals) and clears them when
/// it goes out of scope (also dropping the closures).
pub struct TimingEventKeeper {
    stuff: Vec<(i32, Closure<dyn FnMut()>, bool)>,
}

impl TimingEventKeeper {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(TimingEventKeeper { stuff: vec![] }))
    }

    /// Adds an `FnOnce` closure with a timeout.
    pub fn add<F>(&mut self, f: F, timeout_ms: u32)
    where
        F: FnOnce() + 'static,
    {
        let f = Closure::once(Box::new(f) as Box<dyn FnOnce()>);
        let id = crate::set_timeout_simple(&f, timeout_ms as i32);
        self.stuff.push((id, f, false));
    }

    /// Adds an `FnMut` closure with an interval for repetitive callback.
    pub fn add_repetitive<F>(&mut self, f: F, interval_ms: u32)
    where
        F: FnMut() + 'static,
    {
        let f = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
        let id = crate::set_interval_simple(&f, interval_ms as i32);
        self.stuff.push((id, f, true))
    }
}

impl Drop for TimingEventKeeper {
    fn drop(&mut self) {
        self.stuff.drain(..).for_each(|(id, _, repeating)| {
            if repeating {
                crate::clear_interval(id);
            } else {
                crate::clear_timeout(id);
            }
        });
    }
}
