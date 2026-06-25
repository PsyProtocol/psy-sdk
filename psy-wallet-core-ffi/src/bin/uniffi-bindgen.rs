//! uniffi binding generator. Produces the Swift + Kotlin source from this crate.
//! Usage (see build-ios.sh / build-android.sh):
//!   cargo run --bin uniffi-bindgen -- generate \
//!     --library <path-to-built-lib> --language swift|kotlin --out-dir <dir>
fn main() {
    uniffi::uniffi_bindgen_main()
}
