#![allow(clippy::wildcard_imports)]

use gloo_file::{Blob, futures::read_as_bytes};
use seed::{prelude::*, *};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::closure::Closure;
use web_sys::{MediaStreamConstraints, MediaStream, MediaRecorder, MediaRecorderOptions, BlobEvent};

fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.perform_cmd(get_audio_stream());
    Model::default()
}

// This is essentially copied from the seed user_media example;
// just getting a stream from the client's media source (in my
// case, the microphone)
async fn get_audio_stream() -> Msg {
    let mut constraints = MediaStreamConstraints::new();
    constraints.audio(&JsValue::from(true));

    let navigator = seed::window().navigator();

    let media_devices = navigator.media_devices()
        .map_err(|v| {
            let sopt = v.dyn_ref::<js_sys::JsString>();
            log!("Error getting media devices: {:?}", sopt);
            // return error msg
        })
        .unwrap();

    // We need to request access to user's video or audio through constraints.
    // Otherwise it fails (at least on Windows).
    let stream_promise = media_devices.get_user_media_with_constraints(&constraints)
        .map_err(|v| {
            let sopt = v.dyn_ref::<js_sys::JsString>();
            log!("Error getting user media: {:?}", sopt);
            // return error msg
        })
        .unwrap();

    let stream = JsFuture::from(stream_promise)
        .await
        .map(MediaStream::from)
        .map_err(|e| {
            log!("Error extracting audio stream: {}", e);
        })
        .unwrap();

    Msg::AudioStream(stream)
}

// I change the quickstart Model to a struct because, eventually,
// I would like to have another field Bytes, or Vec<u8>, to which
// I can write bytes captured from the microphone.
// This would be the equivalent of `chunks` from
// https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder#Example
#[derive(Default)]
struct Model {
    recorder: Option<MediaRecorder>,
    on_data_callback: Option<Closure<dyn Fn(JsValue)>>,
    last_chunk: Vec<u8>,
}

enum Msg {
    AudioStream(MediaStream),
    BlobReceived(Blob),
    BlobRead(Vec<u8>),
    StopRecording,
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::AudioStream(stream) => { 
            // `App` clone is cheap. `msg_mapper` is necessary to satisfy Rust types 
            // (`Msg` in `Orders` is hidden in an associated type).
            let (app, msg_mapper) = (orders.clone_app(), orders.msg_mapper());

            // `Closure::wrap` can be written as `Closure::new` 
            //- `new` doesn't need boilerplate like `Box::new` and `as Box<..` however it's not stable yet.
            // `Closure` is a bridge between Rust closures and JS callbacks. That's why the input is `JsValue`.
            let on_data_callback = Closure::wrap(Box::new(move |blob: JsValue| {
                // We are sure that our `JsValue` represents `BlobEvent` so we can use `unchecked_into` to improve performance.
                let web_sys_blob = blob.unchecked_into::<BlobEvent>().data().unwrap();
                // Convert `web_sys::Blob` into `gloo_file`'s one because `gloo_file`'s `Blob` wrapper has some nice methods
                // that eliminate boilerplate a lot.
                let msg = Msg::BlobReceived(Blob::from(web_sys_blob));
                // Pass the message to Seed. Then Seed invokes our `update` function with passed message.
                app.update(msg_mapper(msg));
            }) as Box<dyn Fn(JsValue)>);

            let recorder = create_recorder(stream);
            recorder.set_ondataavailable(Some(on_data_callback.as_ref().unchecked_ref()));
            // We want to receive recorded data each second.
            recorder.start_with_time_slice(1000).unwrap();

            // Store `recorder` in `Model` so we can control it later. 
            // Also there are often attached some drop procedures so it's also safer to store the instance.
            model.recorder = Some(recorder);
            // We need to store callback handle into `Model` or `.forget()` (aka leak) it.
            // Otherwise it'll be dropped and JS throw error once the callback is invoked 
            // because JS callback is stored in recorder and still alive.
            model.on_data_callback = Some(on_data_callback);
            log!("Listening");
        },
        Msg::BlobReceived(blob) => {
            log!("Blob received");
            orders.perform_cmd(async move {
                Msg::BlobRead(read_as_bytes(&blob).await.unwrap())
            });
        }
        Msg::BlobRead(bytes) => {
            model.last_chunk = bytes;
        },
        Msg::StopRecording => {
            // Stop recorder and drop it. 
            //
            //In an ideal world you should:
            // 1. Stop the recorder.
            // 2. Handle the last chunk.
            // 3. Wait for official recorder death (register `onclose` and maybe also `onerror` callbacks).
            // 4. Drop the recorder and drop all callbacks (aka `Closure`s).
            if let Some(recorder) = model.recorder.take() {
                recorder.stop().unwrap();
                log!("Recording stopped")
            }
        }
    }
}

// This is where I am trying to implement the functionality of the
// closure from lines 8-67 of https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder#Example
fn create_recorder(stream: MediaStream) -> MediaRecorder {
    let mut options = MediaRecorderOptions::new();
    options.audio_bits_per_second(64_000);
    options.mime_type("audio/ogg;codecs=opus");

    // And here is where I am stuck.
    // `recorder` is a web_sys::MediaRecorder struct; it has a method `set_ondataavailable`
    // which is where I should be doing something with audio bytes as they become
    // available from the microphone.  What I _want_ to do is to write them to
    // my Model.chunks field (and trigger a Msg that tells the seed runtime that there
    // are bytes to process); but `set_ondataavailable` is a js_sys::Function, and I
    // don't know how to do the interop between JS functions and memory allocation
    // and `js_sys` or `wasm_bindgen` structs.
    // I've looked through various chapters of the wasm_bindgen book
    // https://rustwasm.github.io/wasm-bindgen/introduction.html
    // and tried to figure out how to make the example from the wasm_bindgen Closure docs
    // https://docs.rs/wasm-bindgen/0.2.63/wasm_bindgen/closure/struct.Closure.html
    // work, but haven't been able to wrap my head around them -- at least not for this
    // particular use case.
    MediaRecorder::new_with_media_stream_and_media_recorder_options(
        &stream,
        &options,
    ).unwrap()
}

fn view(model: &Model) -> Node<Msg> {
    div![
        "Last chunk length: ",
        model.last_chunk.len(),
        button!("Stop", ev(Ev::Click, |_| Msg::StopRecording))
    ]
}

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
