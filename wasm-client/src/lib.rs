use wasm_bindgen::prelude::*;

mod websocket_client;
mod webrtc_client;
mod dom_helpers;
mod types;
mod video_chat;

pub use video_chat::VideoChat;

// Initialize WASM module
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"WebRTC Video Chat WASM module loaded".into());
}