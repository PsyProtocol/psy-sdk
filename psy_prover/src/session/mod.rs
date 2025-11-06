pub mod session;
pub use session::*;
pub mod utils;
pub use utils::*;

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(dur: std::time::Duration) {
    tokio::time::sleep(dur).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(dur: std::time::Duration) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::window;

    let window = window().expect("Unable to obtain the browser window object");

    let ms = dur.as_millis() as i32;

    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("Unable to set the timer");
    });

    JsFuture::from(promise).await.expect("Error occurred during sleep");
}
