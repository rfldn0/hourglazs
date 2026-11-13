//! Static assets, compiled into the binary.

use super::routes;

/// Shipped with the public build.
pub const PUBLIC: [(&str, &[u8]); 5] = [
    (routes::PUBLIC_STYLE, include_bytes!("../../assets/public.css")),
    (routes::SCRIPT, include_bytes!("../../assets/app.js")),
    (routes::ICON, include_bytes!("../../assets/hourglass.svg")),
    (routes::STARS_DUST, include_bytes!("../../assets/sky/stars-dust.svg")),
    (routes::STARS_BRIGHT, include_bytes!("../../assets/sky/stars-bright.svg")),
];

/// Only served by the local editing server.
pub const LOCAL: [(&str, &[u8]); 2] = [
    (routes::STYLE, include_bytes!("../../assets/style.css")),
    (routes::EDIT_SCRIPT, include_bytes!("../../assets/edit.js")),
];

/// Look up an asset by its route.
pub fn get(path: &str) -> Option<&'static [u8]> {
    PUBLIC.iter().chain(LOCAL.iter()).find(|(p, _)| *p == path).map(|(_, b)| *b)
}

pub fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("avif") => "image/avif",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
