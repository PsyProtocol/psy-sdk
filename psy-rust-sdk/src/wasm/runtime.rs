use wasm_bindgen::prelude::*;

pub(super) async fn async_sleep_ms(ms: i32) {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{global, Reflect};
        use js_sys::{Function, Promise};
        use wasm_bindgen::{closure::Closure, JsCast, JsValue};

        let promise = Promise::new(&mut |resolve: Function, _reject: Function| {
            let global_obj = global();
            let resolve_for_cb = resolve.clone();
            let cb = Closure::<dyn FnMut()>::once(move || {
                let _ = resolve_for_cb.call0(&JsValue::NULL);
            });
            if let Some(set_timeout) = Reflect::get(&global_obj, &JsValue::from_str("setTimeout"))
                .ok()
                .and_then(|v| v.dyn_into::<Function>().ok())
            {
                let _ = set_timeout.call2(
                    &global_obj,
                    cb.as_ref().unchecked_ref(),
                    &JsValue::from_f64(ms as f64),
                );
            } else {
                let _ = resolve.call0(&JsValue::NULL);
            }
            cb.forget();
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

pub(super) fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

pub(super) fn parse_int_string(value: &str) -> Result<u64, JsError> {
    let normalized = value.strip_prefix("n:").unwrap_or(value);
    normalized
        .parse::<u64>()
        .map_err(|e| JsError::new(&e.to_string()))
}

pub(super) fn parse_u32_string(value: &str) -> Result<u32, JsError> {
    u32::try_from(parse_int_string(value)?).map_err(|_| JsError::new("integer exceeds u32 range"))
}

pub(super) fn parse_fixed_hex<const N: usize>(value: &str, field: &str) -> Result<[u8; N], String> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    let expected_len = N * 2;
    if hex.len() != expected_len {
        return Err(format!(
            "{} must be exactly {} bytes ({} hex characters), got {} hex characters",
            field,
            N,
            expected_len,
            hex.len()
        ));
    }

    fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    let bytes = hex.as_bytes();
    let mut parsed = [0u8; N];
    for (index, output) in parsed.iter_mut().enumerate() {
        let offset = index * 2;
        let high = nibble(bytes[offset])
            .ok_or_else(|| format!("{} contains non-hex character at index {}", field, offset))?;
        let low = nibble(bytes[offset + 1]).ok_or_else(|| {
            format!(
                "{} contains non-hex character at index {}",
                field,
                offset + 1
            )
        })?;
        *output = (high << 4) | low;
    }
    Ok(parsed)
}

pub(super) fn parse_fixed_hex_js<const N: usize>(
    value: &str,
    field: &str,
) -> Result<[u8; N], JsError> {
    parse_fixed_hex(value, field).map_err(|error| JsError::new(&error))
}

pub(super) fn parse_tx_trace_envelope(
    envelope_json: &str,
) -> Result<
    (
        psy_prover::trace::GeneratedTxTraceJson,
        psy_prover::trace::TxTrace,
    ),
    JsError,
> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let envelope: psy_prover::trace::GeneratedTxTraceJson = serde_json::from_str(envelope_json)
        .map_err(|e| JsError::new(&format!("Invalid trace envelope JSON: {}", e)))?;
    let trace = match envelope.trace.encoding.as_str() {
        "json" => serde_json::from_str(&envelope.trace.payload)
            .map_err(|e| JsError::new(&format!("Invalid tx trace JSON payload: {}", e)))?,
        "bincode-base64" => {
            let payload = BASE64
                .decode(envelope.trace.payload.as_bytes())
                .map_err(|e| JsError::new(&format!("Invalid tx trace base64 payload: {}", e)))?;
            bincode::deserialize(&payload)
                .map_err(|e| JsError::new(&format!("Invalid tx trace bincode payload: {}", e)))?
        }
        other => {
            return Err(JsError::new(&format!(
                "Unsupported trace encoding: {}",
                other
            )))
        }
    };
    Ok((envelope, trace))
}

// Initialize panic hook for better error messages in WASM
#[wasm_bindgen(start)]
pub fn main() {
    // Initialize panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Initialize wasm-logger for log crate compatibility
    wasm_logger::init(wasm_logger::Config::default());

    // Initialize tracing subscriber for WASM
    wasm_tracing::set_as_global_default();

    // Log initialization success
    tracing::info!("PSY Rust SDK WASM module initialized successfully with psy_prover support");
}

// Optional manual initialization function
#[wasm_bindgen]
pub fn init_logging() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    wasm_tracing::set_as_global_default();
    tracing::info!("PSY Rust SDK logging initialized manually");
}
