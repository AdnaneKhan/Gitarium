//! Browser-side side effects that need the DOM: opening links and saving
//! files via a transient object-URL anchor click. All no-ops off the web
//! target so the native `px` test suite stays linkable.

/// Open a hyperlink in a new browser tab.
#[cfg(target_arch = "wasm32")]
pub(super) fn open_url(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(url, "_blank");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn open_url(_url: &str) {}

/// Save `text` as a downloaded file.
#[cfg(target_arch = "wasm32")]
pub(super) fn download_text(filename: &str, text: &str) {
    use wasm_bindgen::JsValue;
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(text));
    if let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) {
        save_blob(filename, &blob);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn download_text(_filename: &str, _text: &str) {}

/// Save raw `bytes` as a downloaded file (e.g. a `.tar.gz`).
#[cfg(target_arch = "wasm32")]
pub(super) fn download_bytes(filename: &str, bytes: &[u8]) {
    let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&arr);
    if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&parts) {
        save_blob(filename, &blob);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn download_bytes(_filename: &str, _bytes: &[u8]) {}

/// Trigger a download of `blob` as `filename` via a transient object URL.
#[cfg(target_arch = "wasm32")]
fn save_blob(filename: &str, blob: &web_sys::Blob) {
    use wasm_bindgen::JsCast;
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(blob) else { return };
    if let Ok(a) = doc.create_element("a") {
        let _ = a.set_attribute("href", &url);
        let _ = a.set_attribute("download", filename);
        if let Some(el) = a.dyn_ref::<web_sys::HtmlElement>() {
            el.click();
        }
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
