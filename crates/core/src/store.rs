//! A tiny localStorage key/value helper (used to cache job logs so a given
//! log is never re-fetched). All ops degrade to no-ops off the web target or
//! when storage is unavailable / over quota.

/// Read a value, or `None` if absent / storage is unavailable.
pub fn get(key: &str) -> Option<String> {
    web_sys::window()?.local_storage().ok()??.get_item(key).ok()?
}

/// Write a value, ignoring failures (e.g. the quota being exceeded).
pub fn set(key: &str, val: &str) {
    if let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = s.set_item(key, val);
    }
}
