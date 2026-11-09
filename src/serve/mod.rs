//! `hourglazs serve`: the full app with editing, on 127.0.0.1 only.
//!
//! Every request re-reads the JSON, so hand edits in your editor show up on
//! refresh. Edits go through [`api`] and are written straight back to disk.

mod api;
mod publish;

use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, anyhow};
use maud::html;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::data;
use crate::domain::Site;
use crate::render::{self, assets, routes};

use publish::Publisher;

pub struct Options {
    pub data: PathBuf,
    pub port: u16,
    /// Commit and push `data/` after this much quiet time; `None` = off.
    pub publish: Option<Duration>,
}

const MAX_BODY: u64 = 1 << 20;

type Resp = Response<Cursor<Vec<u8>>>;

pub fn run(opts: Options) -> Result<()> {
    data::load(&opts.data)?; // fail fast on a broken data directory
    let addr = format!("127.0.0.1:{}", opts.port);
    let server = Server::http(&addr).map_err(|e| anyhow!("cannot listen on {addr}: {e}"))?;
    let publisher = opts.publish.map(|quiet| publish::spawn(opts.data.clone(), quiet)).transpose()?;

    println!("hourglazs is running at http://{addr}/");
    println!("  public page preview: http://{addr}/{}", routes::PUBLIC_PREVIEW.trim_end_matches("index.html"));
    match opts.publish {
        Some(q) => println!("  auto-publish: on (commit + push {}s after your last edit)", q.as_secs()),
        None => println!("  auto-publish: off (add --publish to commit and push edits)"),
    }
    println!("Press Ctrl+C to stop.");

    for mut req in server.incoming_requests() {
        let resp = respond(&opts, &mut req, publisher.as_ref());
        let _ = req.respond(resp);
    }
    Ok(())
}

fn respond(opts: &Options, req: &mut Request, publisher: Option<&Publisher>) -> Resp {
    let path = req.url().split('?').next().unwrap_or("/").trim_start_matches('/').to_string();
    match req.method() {
        Method::Get | Method::Head => get(opts, &path),
        Method::Post if path.starts_with("api/") => post(opts, req, &path["api/".len()..], publisher),
        _ => json_resp(405, json!({ "error": "method not allowed" })),
    }
}

fn get(opts: &Options, path: &str) -> Resp {
    let path = if path.is_empty() || path.ends_with('/') { format!("{path}index.html") } else { path.to_string() };
    if let Some(bytes) = assets::get(&path) {
        return bytes_resp(200, bytes.to_vec(), assets::mime(&path));
    }

    let loaded = match data::load(&opts.data) {
        Ok(l) => l,
        Err(e) => return problem_page(&[format!("{e:#}")]),
    };
    let report = data::validate(&loaded);
    if !report.is_ok() {
        return problem_page(&report.errors);
    }
    let site = Site::new(loaded, None);
    let page = if path == routes::PUBLIC_PREVIEW {
        Some(render::public_page(&site, &path))
    } else {
        render::app_page(&site, &path, true)
    };
    match page {
        Some(html) => bytes_resp(200, html.0.into_bytes(), assets::mime("x.html")),
        None => {
            let body = html! { h1 { "Not found" } p { a href="/" { "Back to hourglazs" } } };
            bytes_resp(404, body.0.into_bytes(), assets::mime("x.html"))
        }
    }
}

fn post(opts: &Options, req: &mut Request, op: &str, publisher: Option<&Publisher>) -> Resp {
    // Only same-origin JSON requests: a random web page can't send these
    // without a CORS preflight, which this server never approves.
    let json_body = header(req, "Content-Type").is_some_and(|v| v.starts_with("application/json"));
    let same_origin = header(req, "Origin").is_none_or(|o| {
        o == format!("http://127.0.0.1:{}", opts.port) || o == format!("http://localhost:{}", opts.port)
    });
    if !json_body || !same_origin {
        return json_resp(403, json!({ "error": "forbidden" }));
    }

    let mut body = String::new();
    if req.as_reader().take(MAX_BODY).read_to_string(&mut body).is_err() {
        return json_resp(400, json!({ "error": "unreadable body" }));
    }
    match api::handle(&opts.data, op, &body) {
        Ok((reply, summary)) => {
            println!("saved: {summary}");
            if let Some(p) = publisher {
                p.notify(summary);
            }
            json_resp(200, reply)
        }
        Err(e) => json_resp(400, json!({ "error": format!("{e:#}") })),
    }
}

fn header(req: &Request, name: &'static str) -> Option<String> {
    req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str().to_string())
}

/// Shown instead of the app when the JSON on disk is broken.
fn problem_page(errors: &[String]) -> Resp {
    let body = html! {
        (maud::DOCTYPE)
        html { head { meta charset="utf-8"; title { "hourglazs: data problem" } }
            body style="font:15px system-ui;max-width:760px;margin:48px auto;padding:0 16px" {
                h1 { "The data has a problem" }
                p { "Fix these in your editor, then refresh:" }
                ul { @for e in errors { li { pre style="white-space:pre-wrap" { (e) } } } }
            }
        }
    };
    bytes_resp(500, body.0.into_bytes(), assets::mime("x.html"))
}

fn json_resp(code: u16, v: Value) -> Resp {
    bytes_resp(code, v.to_string().into_bytes(), assets::mime("x.json"))
}

fn bytes_resp(code: u16, bytes: Vec<u8>, mime: &str) -> Resp {
    Response::from_data(bytes)
        .with_status_code(code)
        .with_header(Header::from_bytes("Content-Type", mime).expect("valid header"))
        .with_header(Header::from_bytes("Cache-Control", "no-store").expect("valid header"))
}
