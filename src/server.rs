use crate::types::{CookieError, CookieOptions};
use dioxus::fullstack::http::header::{HeaderValue, COOKIE, SET_COOKIE};
use dioxus::fullstack::FullstackContext;

pub fn set(name: &str, value: &str, options: &CookieOptions) -> Result<(), CookieError> {
    let ctx = FullstackContext::current()
        .ok_or_else(|| CookieError::new("No server context available"))?;

    ctx.add_response_header(
        SET_COOKIE,
        HeaderValue::from_str(&options.build_header(name, value))
            .map_err(|e| CookieError::new(format!("Invalid cookie value: {}", e)))?,
    );

    Ok(())
}

pub fn get(name: &str) -> Option<String> {
    let ctx = FullstackContext::current()?;
    let parts = ctx.parts_mut();
    let cookie_header = parts.headers.get(COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;
    let prefix = format!("{}=", name);

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&prefix) {
            return urlencoding::decode(value).ok().map(|s| s.into_owned());
        }
    }

    None
}

/// Same as get() — HttpOnly is browser-enforced, not server-enforced.
pub fn get_internal(name: &str) -> Option<String> {
    get(name)
}

pub fn clear(name: &str) -> Result<(), CookieError> {
    let ctx = FullstackContext::current()
        .ok_or_else(|| CookieError::new("No server context available"))?;

    let mut s = String::with_capacity(name.len() + 50);
    s.push_str(name);
    s.push_str("=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0");

    ctx.add_response_header(
        SET_COOKIE,
        HeaderValue::from_str(&s)
            .map_err(|e| CookieError::new(format!("Invalid cookie value: {}", e)))?,
    );
    Ok(())
}

pub fn list_names() -> Vec<String> {
    let ctx = match FullstackContext::current() {
        Some(c) => c,
        None => return vec![],
    };
    let parts = ctx.parts_mut();

    let Some(cookie_header) = parts.headers.get(COOKIE) else {
        return vec![];
    };
    let Ok(cookie_str) = cookie_header.to_str() else {
        return vec![];
    };

    cookie_str
        .split(';')
        .filter_map(|cookie| {
            let cookie = cookie.trim();
            cookie.split('=').next().map(|name| name.to_string())
        })
        .collect()
}
