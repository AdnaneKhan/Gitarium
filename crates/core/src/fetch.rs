//! HTTP on top of globalThis.fetch — works identically in browsers and Bun.

use std::cell::Cell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = fetch)]
    fn global_fetch(req: &web_sys::Request) -> js_sys::Promise;
}

thread_local! {
    /// (remaining, limit) from the most recent GitHub API response.
    pub static RATE_LIMIT: Cell<Option<(u32, u32)>> = Cell::new(None);
}

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub async fn request(
    method: &str,
    url: &str,
    headers: &[(&str, String)],
    body: Option<String>,
) -> Result<HttpResponse, String> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    // Bypass the browser HTTP cache. GitHub sets `Cache-Control: private,
    // max-age=60` on list responses, so without this a refetch right after a
    // mutation (delete/create) returns the stale pre-mutation list for up to a
    // minute — e.g. a deleted secret lingering in the UI.
    init.set_cache(web_sys::RequestCache::NoStore);
    let h = web_sys::Headers::new().map_err(js_err)?;
    for (k, v) in headers {
        h.set(k, v).map_err(js_err)?;
    }
    init.set_headers(h.as_ref());
    if let Some(b) = &body {
        init.set_body(&JsValue::from_str(b));
    }
    let req = web_sys::Request::new_with_str_and_init(url, &init).map_err(js_err)?;

    let resp_value = JsFuture::from(global_fetch(&req)).await.map_err(js_err)?;
    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "fetch did not return a Response".to_string())?;

    let hdrs = resp.headers();
    let remaining: Option<u32> = hdrs
        .get("x-ratelimit-remaining")
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok());
    let limit: Option<u32> = hdrs
        .get("x-ratelimit-limit")
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok());
    if let (Some(r), Some(l)) = (remaining, limit) {
        RATE_LIMIT.with(|c| c.set(Some((r, l))));
    }

    let text = JsFuture::from(resp.text().map_err(js_err)?)
        .await
        .map_err(js_err)?;
    Ok(HttpResponse {
        status: resp.status(),
        body: text.as_string().unwrap_or_default(),
    })
}

fn js_err(e: JsValue) -> String {
    e.dyn_ref::<js_sys::Error>()
        .map(|er| String::from(er.message()))
        .unwrap_or_else(|| format!("{:?}", e))
}
