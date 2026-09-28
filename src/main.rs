//! # The Quarry — the Thread's model store
//!
//! Where you go for stone that is already cut. An asset store, with one
//! difference that changes what it can promise: **you publish a recipe, not
//! a file.** The Quarry runs the recipe itself — evaluates the Weft program,
//! carves the mesh, bakes the PBR maps, writes the `.glb`, renders the
//! preview — so an entry cannot misrepresent its artifact. There is nothing
//! to compare and nothing to trust: the file *is* the program's output.
//!
//! That also makes every model **re-derivable**. The CDN is an optimisation,
//! not a dependency: lose the Quarry and every world rebuilds from recipes.
//!
//! It is also the **viewing room**: the one page the models are opened,
//! turned, judged and made in. That page is rendered here, by the binary
//! that owns the models, so a crawler and an answer engine get the store's
//! actual contents rather than an empty div (see [`page`]).
//!
//! Routes (HTTP/1.1, TLS at the proxy):
//!   GET  /                          → the viewing room (HTML), or the
//!        index JSON for anything that did not ask for HTML
//!   GET  /index.json                → index: motto, counts, kinds
//!   GET  /m/<design>                → the viewing room, opened on one model
//!   GET  /healthz                   → 200 ok
//!   GET  /models                    → every entry (facts included)
//!   GET  /models/search?kind=&h=&w=&d=&tol=&style=&tags=&limit=
//!                                   → ranked by fit, for a layout engine
//!   GET  /models/<design>.json      → one entry
//!   GET  /models/<design>.glb       → the artifact
//!   GET  /models/<design>.png       → the preview sheet
//!   GET  /library                   → what the Make door may offer: the
//!        weft-model shapes with the arity the library itself declares, its
//!        materials, and the grower's blank recipe
//!   POST /publish                   → a submission (recipe + words):
//!        derived, then stored
//!   POST /derive                    → the same, kept off the shelf: a look
//!        before you publish. The artifact lands in a swept scratch drawer
//!        and is served from /derived/<design>.glb
//!   POST /models/<design>/verdict   → {verdict, note} → kept on the entry
//!   POST /models/<design>/concept   → {codex, concept} → kept on the entry
//!
//! Every POST is gated by `authorization: Bearer <QUARRY_TOKEN>` when that
//! variable is set: they all either cost the store real work or write to an
//! entry.
//!
//! Env: PORT (default 3000), QUARRY_DATA (default ./data), QUARRY_TOKEN.
//! An empty shelf seeds itself from the built-in `weft-model` library, so
//! the store is never empty on arrival.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

mod avatar;
mod entry;
mod library;
mod page;
use entry::{derive, Submission};

struct App {
    data: PathBuf,
    /// Derived-to-be-looked-at, never shelved. Swept as it fills.
    scratch: PathBuf,
    token: Option<String>,
    /// Carving and growing are CPU work, and the store has one machine. Two
    /// at a time keeps a browser responsive without letting a stampede of
    /// derives take the whole box down.
    kiln: Semaphore,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let data = PathBuf::from(std::env::var("QUARRY_DATA").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data).expect("data dir");

    // "Empty" means no entries — not "no files". A scratch drawer left over
    // from the last run is not stock.
    let shelved = std::fs::read_dir(&data)
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".json")).count())
        .unwrap_or(0);
    if shelved == 0 {
        seed(&data);
    }

    let scratch = data.join(".derived");
    let _ = std::fs::create_dir_all(&scratch);
    let app = Arc::new(App {
        data,
        scratch,
        token: std::env::var("QUARRY_TOKEN").ok().filter(|t| !t.is_empty()),
        kiln: Semaphore::new(2),
    });
    let listener = TcpListener::bind(("0.0.0.0", port)).await.expect("bind");
    println!("quarry listening on :{port} — the stone is cut");
    loop {
        let Ok((stream, _)) = listener.accept().await else { continue };
        let app = app.clone();
        tokio::spawn(async move {
            let _ = handle(stream, app).await;
        });
    }
}

/// A shelf worth arriving at: derive a starter set from the built-in library
/// so the first visitor finds stock, not an empty room.
const STARTERS: &[(&str, &str, &[f32], &str, &[&str])] = &[
    ("Doric column, 5.2 m", "column", &[5.2, 0.44], "marble",
     &["column", "classical", "load-bearing"]),
    ("Stone column, 3.6 m", "column", &[3.6, 0.36], "granite",
     &["column", "classical", "load-bearing"]),
    ("Stair flight, 12 steps", "stairs", &[12.0, 0.18, 0.28, 1.2], "granite",
     &["stairs", "circulation"]),
    ("Stone arch, 3 × 4 m", "arch", &[3.0, 4.0, 0.6], "sandstone",
     &["arch", "threshold", "doorway"]),
    ("Amphora", "amphora", &[1.2], "", &["vessel", "prop", "classical"]),
    ("Terracotta vase", "vase", &[0.9, 0.3, 0.12], "terracotta",
     &["vessel", "prop"]),
    ("Oak table", "table", &[1.6, 0.9, 0.75], "wood", &["furniture", "table"]),
    ("Field boulder", "rock", &[0.8, 3.0], "granite", &["rock", "nature", "scatter"]),
    ("Stone bowl", "bowl", &[0.5, 0.06], "granite", &["vessel", "prop"]),
];

/// The ids the starters have carried on quarry.pixygon.io since the shelf
/// was first seeded (2026-09-25). A test holds the canonical form to these:
/// move one and nine published designs become orphans.
#[cfg(test)]
pub fn starters_pinned() -> Vec<(&'static str, &'static [f32], &'static str, &'static str)> {
    vec![
        ("column", &[5.2, 0.44], "marble", "5e41fec7b0ce3898"),
        ("column", &[3.6, 0.36], "granite", "3ecde3178de1066a"),
        ("stairs", &[12.0, 0.18, 0.28, 1.2], "granite", "7d225db1e5b4ff0d"),
        ("arch", &[3.0, 4.0, 0.6], "sandstone", "b4509a9ba0dee026"),
        ("amphora", &[1.2], "", "55e84461f295b569"),
        ("vase", &[0.9, 0.3, 0.12], "terracotta", "0c20a90c4f7dee65"),
        ("table", &[1.6, 0.9, 0.75], "wood", "94a7a6168043098f"),
        ("rock", &[0.8, 3.0], "granite", "7085cfb3aae497f9"),
        ("bowl", &[0.5, 0.06], "granite", "d94d6155bb7bf4f7"),
    ]
}

fn seed(data: &PathBuf) {
    for (title, export, args, material, tags) in STARTERS {
        let sub = Submission {
            title: title.to_string(),
            description: String::new(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            kind: tags.first().unwrap_or(&"prop").to_string(),
            style: String::new(),
            package: "weft-model".into(),
            export: export.to_string(),
            args: args.iter().map(|a| serde_json::json!(a)).collect(),
            recipe: None,
            material: material.to_string(),
            license: "CC0-1.0".into(),
            author: "did:pixygon:quarry".into(),
            origin: "seed".into(),
            sockets: Vec::new(),
            codex: String::new(),
            concept: String::new(),
            glb: None,
            source: String::new(),
            rest: false,
            hang: None,
        };
        match derive(&sub, data) {
            Ok(e) => println!("seeded {} ({})", e.design, e.title),
            Err(err) => eprintln!("seed '{title}' REFUSED: {err}"),
        }
    }
}

async fn handle(mut stream: TcpStream, app: Arc<App>) -> std::io::Result<()> {
    let mut buf: Vec<u8> = Vec::with_capacity(8192);
    let mut tmp = [0u8; 8192];
    let head_end;
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            head_end = pos + 4;
            break;
        }
        if buf.len() > 64 * 1024 {
            return respond(&mut stream, false, 431, "text/plain", Cache::None, b"head too large").await;
        }
    }
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let mut request = lines.next().unwrap_or("").split_whitespace();
    let (method, target) = (request.next().unwrap_or(""), request.next().unwrap_or("/"));
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.to_string(), String::new()),
    };
    let header = |name: &str| -> Option<String> {
        head.lines()
            .find(|l| l.to_ascii_lowercase().starts_with(&format!("{name}:")))
            .map(|l| l[name.len() + 1..].trim().to_string())
    };

    // A HEAD is a GET that stops at the headers — same route, same headers,
    // no body. Routing it separately answered 404 to every crawler that
    // asked politely whether a model was there.
    let head_only = method == "HEAD";
    let method = if head_only { "GET" } else { method };

    let wants_html = header("accept").is_some_and(|a| a.contains("text/html"));
    let authorised = || -> bool {
        match &app.token {
            None => true,
            Some(tok) => header("authorization")
                .is_some_and(|a| a.strip_prefix("Bearer ") == Some(tok.as_str())),
        }
    };

    // Any POST carries a body; read it once, here, rather than in four places.
    let body: Vec<u8> = if method == "POST" {
        let want: usize = header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        // A recipe is kilobytes; a manifested GLB with its textures embedded
        // is tens of megabytes, and the convention says embed them.
        let is_glb = header("content-type").is_some_and(|c| c.starts_with("model/gltf-binary"));
        let cap = if is_glb { 64 * 1024 * 1024 } else { 8 * 1024 * 1024 };
        if want == 0 || want > cap {
            return respond(&mut stream, head_only, 413, "text/plain", Cache::None,
                if is_glb { b"bad content-length (max 64MB for a GLB)" as &[u8] } else { b"bad content-length (max 8MB)" }).await;
        }
        let mut b = buf[head_end..].to_vec();
        while b.len() < want {
            let n = stream.read(&mut tmp).await?;
            if n == 0 {
                break;
            }
            b.extend_from_slice(&tmp[..n]);
        }
        b
    } else {
        Vec::new()
    };

    match (method, path.as_str()) {
        ("GET", "/healthz") => respond(&mut stream, head_only, 200, "text/plain", Cache::None, b"ok").await,
        ("OPTIONS", _) => respond(&mut stream, head_only, 204, "text/plain", Cache::None, b"").await,
        ("GET", "/app.css") => {
            respond(&mut stream, head_only, 200, "text/css; charset=utf-8", Cache::Immutable, page::css().as_bytes()).await
        }
        ("GET", "/app.js") => {
            respond(&mut stream, head_only, 200, "text/javascript; charset=utf-8", Cache::Immutable, page::js().as_bytes()).await
        }
        ("GET", "/library") => {
            let body = library::catalog().to_string();
            respond(&mut stream, head_only, 200, "application/json", Cache::Short, body.as_bytes()).await
        }
        ("GET", "/robots.txt") => {
            let body = "User-agent: *\nAllow: /\nSitemap: https://quarry.pixygon.io/sitemap.xml\n";
            respond(&mut stream, head_only, 200, "text/plain", Cache::Short, body.as_bytes()).await
        }
        ("GET", "/sitemap.xml") => {
            let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n<url><loc>https://quarry.pixygon.io/</loc></url>\n");
            for e in load_all(&app.data) {
                x.push_str(&format!("<url><loc>https://quarry.pixygon.io/m/{}</loc></url>\n", e.design));
            }
            x.push_str("</urlset>\n");
            respond(&mut stream, head_only, 200, "application/xml", Cache::Short, x.as_bytes()).await
        }
        // The front door answers in the visitor's language: a browser or a
        // crawler asked for HTML and gets the viewing room; everything else
        // — the CLI, a layout engine — gets the index it has always got.
        ("GET", "/") if wants_html => {
            let entries = load_all(&app.data);
            let html = page::render(&entries, None, app.token.is_some());
            respond(&mut stream, head_only, 200, "text/html; charset=utf-8", Cache::None, html.as_bytes()).await
        }
        ("GET", "/") | ("GET", "/index.json") => {
            let entries = load_all(&app.data);
            let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
            for e in &entries {
                *kinds.entry(e.kind.clone()).or_default() += 1;
            }
            let body = serde_json::json!({
                "registry": "quarry",
                "motto": "models that are what they say they are — the file is the program's output",
                "spec": "https://github.com/Pixygon/Infinite/blob/main/docs/spec/model-v0.1.md",
                "models": entries.len(),
                "kinds": kinds,
                "routes": ["/models", "/models/search?kind=…&h=…&tol=…", "/models/<design>.glb", "/publish", "/derive", "/library"],
            });
            respond(&mut stream, head_only, 200, "application/json", Cache::None, body.to_string().as_bytes()).await
        }
        ("GET", p) if p.starts_with("/m/") => {
            let id = &p["/m/".len()..];
            let entries = load_all(&app.data);
            let cur = entries.iter().find(|e| e.design == id);
            if cur.is_none() {
                let html = page::render(&entries, None, app.token.is_some());
                return respond(&mut stream, head_only, 404, "text/html; charset=utf-8", Cache::None, html.as_bytes()).await;
            }
            let html = page::render(&entries, cur, app.token.is_some());
            respond(&mut stream, head_only, 200, "text/html; charset=utf-8", Cache::None, html.as_bytes()).await
        }
        ("GET", "/models") => {
            let entries = load_all(&app.data);
            let body = serde_json::json!({ "count": entries.len(), "models": entries });
            respond(&mut stream, head_only, 200, "application/json", Cache::None, body.to_string().as_bytes()).await
        }
        ("GET", "/models/search") => {
            let q = parse_query(&query);
            let hits = entry::search(&load_all(&app.data), &q);
            let body = serde_json::json!({ "query": q, "count": hits.len(), "models": hits });
            respond(&mut stream, head_only, 200, "application/json", Cache::None, body.to_string().as_bytes()).await
        }
        ("GET", p) if p.starts_with("/models/") || p.starts_with("/derived/") => {
            let scratch = p.starts_with("/derived/");
            let dir = if scratch { &app.scratch } else { &app.data };
            let name = p.split_once('/').and_then(|(_, r)| r.split_once('/')).map(|(_, n)| n).unwrap_or("");
            let Some((id, ext)) = split_artifact(name) else {
                return respond(&mut stream, head_only, 400, "text/plain", Cache::None, b"bad design id").await;
            };
            let (file, ctype, cache) = match ext.as_str() {
                "glb" => (name.to_string(), "model/gltf-binary", Cache::Immutable),
                "png" => (name.to_string(), "image/png", Cache::Immutable),
                _ => (format!("{id}.json"), "application/json", Cache::None),
            };
            match std::fs::read(dir.join(&file)) {
                Ok(bytes) => respond(&mut stream, head_only, 200, ctype, cache, &bytes).await,
                Err(_) => respond(&mut stream, head_only, 404, "text/plain", Cache::None, b"no such model").await,
            }
        }
        ("POST", "/publish") | ("POST", "/derive") => {
            if !authorised() {
                return respond(&mut stream, head_only, 401, "text/plain", Cache::None, b"bad token").await;
            }
            // A GLB body is an avatar item; its words come from the manifest,
            // with the query string allowed to add tags, a style, a codex.
            let sub = if body.starts_with(b"glTF") {
                let q = parse_query(&query);
                let get = |k: &str| q.get(k).cloned().unwrap_or_default();
                Submission {
                    title: get("title"),
                    description: String::new(),
                    tags: get("tags").split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
                    kind: String::new(),
                    style: get("style"),
                    package: "avatar".into(),
                    export: String::new(),
                    args: Vec::new(),
                    recipe: None,
                    material: String::new(),
                    license: if get("license").is_empty() { "CC0-1.0".into() } else { get("license") },
                    author: get("author"),
                    origin: "imported".into(),
                    sockets: Vec::new(),
                    codex: get("codex"),
                    concept: get("concept"),
                    glb: Some(body.clone()),
                    source: String::new(),
                    rest: false,
                    hang: None,
                }
            } else {
                match parse_submission(&body) {
                    Ok(s) => s,
                    Err(msg) => return respond(&mut stream, head_only, 400, "text/plain", Cache::None, msg.as_bytes()).await,
                }
            };
            let shelve = path == "/publish";
            let dir = if shelve { app.data.clone() } else { app.scratch.clone() };
            let shelf = app.data.clone();
            // Carving and growing block; keep them off the runtime's threads.
            let _permit = app.kiln.acquire().await;
            let made = tokio::task::spawn_blocking(move || {
                if shelve { derive(&sub, &dir) } else { entry::derive_scratch(&sub, &dir, &shelf) }
            })
            .await
            .unwrap_or_else(|e| Err(format!("the kiln died: {e}")));
            match made {
                Ok(e) => {
                    let what = if shelve { "published" } else { "derived (not shelved)" };
                    println!("{what} {} — {} ({} tris)", e.design, e.title, e.artifact.tris);
                    let body = serde_json::to_string(&e).unwrap_or_default();
                    respond(&mut stream, head_only, 200, "application/json", Cache::None, body.as_bytes()).await
                }
                Err(err) => {
                    let msg = format!("REFUSED: {err}");
                    respond(&mut stream, head_only, 422, "text/plain", Cache::None, msg.as_bytes()).await
                }
            }
        }
        ("POST", p) if p.starts_with("/models/") => {
            if !authorised() {
                return respond(&mut stream, head_only, 401, "text/plain", Cache::None, b"bad token").await;
            }
            let rest = &p["/models/".len()..];
            let Some((id, what)) = rest.split_once('/') else {
                return respond(&mut stream, head_only, 404, "text/plain", Cache::None, b"no such route").await;
            };
            if !is_design(id) {
                return respond(&mut stream, head_only, 400, "text/plain", Cache::None, b"bad design id").await;
            }
            let file = app.data.join(format!("{id}.json"));
            let Ok(text) = std::fs::read_to_string(&file) else {
                return respond(&mut stream, head_only, 404, "text/plain", Cache::None, b"no such model").await;
            };
            let Ok(mut e) = serde_json::from_str::<entry::Entry>(&text) else {
                return respond(&mut stream, head_only, 500, "text/plain", Cache::None, b"the entry on disk is unreadable").await;
            };
            let patch: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
            let field = |k: &str| patch.get(k).and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            match what {
                // A verdict is a judgement about the design, so it lives with
                // the design — not in the browser that happened to make it.
                "verdict" => {
                    let verdict = field("verdict");
                    if !matches!(verdict.as_str(), "yes" | "close" | "no") {
                        return respond(&mut stream, head_only, 400, "text/plain", Cache::None,
                            b"a verdict is 'yes', 'close' or 'no'").await;
                    }
                    let mut note = field("note");
                    note.truncate(400);
                    e.verdicts.push(entry::Verdict { verdict, note, by: field("by"), at: entry::now() });
                    if e.verdicts.len() > 50 {
                        let cut = e.verdicts.len() - 50;
                        e.verdicts.drain(..cut);
                    }
                }
                "concept" => {
                    e.codex = field("codex");
                    e.concept = field("concept");
                    if !e.concept.is_empty() && !e.concept.starts_with("https://") {
                        return respond(&mut stream, head_only, 400, "text/plain", Cache::None,
                            b"a concept is an https url").await;
                    }
                }
                _ => return respond(&mut stream, head_only, 404, "text/plain", Cache::None, b"no such route").await,
            }
            let json = serde_json::to_string_pretty(&e).unwrap_or_default();
            if std::fs::write(&file, &json).is_err() {
                return respond(&mut stream, head_only, 500, "text/plain", Cache::None, b"cannot write the entry").await;
            }
            respond(&mut stream, head_only, 200, "application/json", Cache::None, json.as_bytes()).await
        }
        ("GET", _) => respond(&mut stream, head_only, 404, "text/plain", Cache::None, b"no such route").await,
        _ => respond(&mut stream, head_only, 405, "text/plain", Cache::None, b"method not allowed").await,
    }
}

/// `<design>.<ext>`, where the design is the hex the Quarry itself minted —
/// nothing else may name a file, which is what keeps `..` out of the path.
fn split_artifact(name: &str) -> Option<(String, String)> {
    let (id, ext) = name.split_once('.').unwrap_or((name, "json"));
    if !is_design(id) || !ext.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') {
        return None;
    }
    Some((id.to_string(), ext.rsplit('.').next().unwrap_or("").to_string()))
}

fn is_design(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_submission(body: &[u8]) -> Result<Submission, String> {
    let text = std::str::from_utf8(body).map_err(|_| "not utf-8".to_string())?;
    serde_json::from_str(text).map_err(|e| format!("not a submission: {e}"))
}

fn load_all(data: &PathBuf) -> Vec<entry::Entry> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".json") {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<entry::Entry>(
                &std::fs::read_to_string(e.path()).unwrap_or_default(),
            ) {
                out.push(entry);
            }
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

fn parse_query(q: &str) -> BTreeMap<String, String> {
    q.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), urldecode(v)))
        .collect()
}

fn urldecode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("20");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(b' ') as char);
                i += 3;
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// How long a thing may be believed. The artifacts are content-addressed —
/// their name IS their hash, so they can be cached forever — and nothing
/// else can be: an entry gains verdicts, the shelf gains models, and a page
/// that went stale would be lying about a store whose whole claim is that
/// it does not.
#[derive(Clone, Copy)]
enum Cache {
    Immutable,
    Short,
    None,
}

impl Cache {
    fn header(self) -> &'static str {
        match self {
            Cache::Immutable => "public, max-age=31536000, immutable",
            Cache::Short => "public, max-age=300",
            Cache::None => "no-cache",
        }
    }
}

async fn respond(
    stream: &mut TcpStream,
    head_only: bool,
    status: u16,
    ctype: &str,
    cache: Cache,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\naccess-control-allow-origin: *\r\naccess-control-allow-headers: authorization, content-type\r\naccess-control-allow-methods: GET, POST, OPTIONS\r\ncache-control: {}\r\nconnection: close\r\n\r\n",
        body.len(),
        cache.header()
    );
    stream.write_all(head.as_bytes()).await?;
    if !head_only {
        stream.write_all(body).await?;
    }
    stream.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Quarry mints every filename it serves — a design id is hex and
    /// nothing else. That is the whole defence against `..`, and it is worth
    /// a test because the next person to add a route will copy this one.
    #[test]
    fn only_a_design_can_name_a_file() {
        assert!(is_design("5272b8f10550f094"));
        assert!(!is_design(""));
        assert!(!is_design(".."));
        assert!(!is_design("../../etc/passwd"));
        assert!(!is_design("5272b8f10550f094 "));
        assert!(!is_design(&"a".repeat(65)));
    }

    #[test]
    fn an_artifact_name_is_a_design_and_an_extension() {
        assert_eq!(
            split_artifact("5272b8f10550f094.glb"),
            Some(("5272b8f10550f094".into(), "glb".into()))
        );
        // the LOD spelling the grower writes
        assert_eq!(
            split_artifact("5272b8f10550f094.lod2.glb"),
            Some(("5272b8f10550f094".into(), "glb".into()))
        );
        // bare = the entry
        assert_eq!(
            split_artifact("5272b8f10550f094"),
            Some(("5272b8f10550f094".into(), "json".into()))
        );
        for bad in ["../../etc/passwd", "..%2f..%2fetc", "", ".", "a/b.glb", "5272b8f1.gl/b"] {
            assert_eq!(split_artifact(bad), None, "accepted {bad:?}");
        }
    }

    #[test]
    fn a_submission_must_be_json() {
        assert!(parse_submission(b"not json").is_err());
        assert!(parse_submission(&[0xff, 0xfe]).is_err());
        let ok = parse_submission(br#"{"title":"x","export":"column","args":[1,2]}"#).unwrap();
        assert_eq!(ok.title, "x");
        assert_eq!(ok.package, "weft-model", "the default package is the library");
    }

    #[test]
    fn only_the_artifacts_are_immutable() {
        assert!(Cache::Immutable.header().contains("immutable"));
        assert_eq!(Cache::None.header(), "no-cache");
        assert!(!Cache::Short.header().contains("immutable"));
    }
}
