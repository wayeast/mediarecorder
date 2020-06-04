#![allow(clippy::wildcard_imports)]

use seed::{prelude::*, *};
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStreamConstraints, MediaStream, MediaRecorder, MediaRecorderOptions};

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
    let stream_promise = media_devices.get_user_media()
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
    counter: i32,
    // chunks: Bytes??, Vec<u8>??
}

#[derive(Clone)]
enum Msg {
    Increment,
    AudioStream(MediaStream),
}

fn update(msg: Msg, model: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {
        Msg::Increment => model.counter += 1,
        Msg::AudioStream(stream) => record(stream, model),
    }
}

// This is where I am trying to implement the functionality of the
// closure from lines 8-67 of https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder#Example
fn record(stream: MediaStream, _: &mut Model) {
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
    let recorder = MediaRecorder::new_with_media_stream_and_media_recorder_options(
        &stream,
        &options,
    );
}

fn view(model: &Model) -> Node<Msg> {
    div![
        "This is a counter: ",
        C!["counter"],
        button![model.counter, ev(Ev::Click, |_| Msg::Increment),],
    ]
}

#[wasm_bindgen(start)]
pub fn start() {
    App::start("app", init, update, view);
}
