use console_error_panic_hook;
use wasm_bindgen::prelude::*;

mod dom_helpers;
mod types;
mod video_chat;
mod webrtc_client;
mod websocket_client;

pub use video_chat::VideoChat;

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    Ok(())
}
