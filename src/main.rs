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
//! Routes (HTTP/1.1, TLS at the proxy):
//!   GET  /                          → index: motto, counts, kinds
//!   GET  /healthz                   → 200 ok
//!   GET  /models                    → every entry (facts included)
//!   GET  /models/search?kind=&h=&w=&d=&tol=&style=&tags=&limit=
//!                                   → ranked by fit, for a layout engine
//!   GET  /models/<design>.json      → one entry
//!   GET  /models/<design>.glb       → the artifact
//!   GET  /models/<design>.png       → the preview sheet
//!   POST /publish                   → body = a submission (recipe + words);
//!        derived, then stored. If QUARRY_TOKEN is set, requires
//!        `authorization: Bearer <token>`.
//!
//! Env: PORT (default 3000), QUARRY_DATA (default ./data), QUARRY_TOKEN.
//! An empty shelf seeds itself from the built-in `weft-model` library, so
//! the store is never empty on arrival.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod entry;
use entry::{derive, Submission};

struct App {
    data: PathBuf,
    token: Option<String>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let data = PathBuf::from(std::env::var("QUARRY_DATA").unwrap_or_else(|_| "data".into()));
    std::fs::create_dir_all(&data).expect("data dir");

    if std::fs::read_dir(&data).map(|mut d| d.next().is_none()).unwrap_or(true) {
        seed(&data);
    }

    let app = Arc::new(App {
        data,
        token: std::env::var("QUARRY_TOKEN").ok().filter(|t| !t.is_empty()),
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
fn seed(data: &PathBuf) {
    let starters: &[(&str, &str, &[f32], &str, &[&str])] = &[
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
    for (title, export, args, material, tags) in starters {
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
            return respond(&mut stream, 431, "text/plain", b"head too large").await;
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

    match (method, path.as_str()) {
        ("GET", "/healthz") => respond(&mut stream, 200, "text/plain", b"ok").await,
        ("GET", "/") => {
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
                "routes": ["/models", "/models/search?kind=…&h=…&tol=…", "/models/<design>.glb", "/publish"],
            });
            respond(&mut stream, 200, "application/json", body.to_string().as_bytes()).await
        }
        ("GET", "/models") => {
            let entries = load_all(&app.data);
            let body = serde_json::json!({ "count": entries.len(), "models": entries });
            respond(&mut stream, 200, "application/json", body.to_string().as_bytes()).await
        }
        ("GET", "/models/search") => {
            let q = parse_query(&query);
            let hits = entry::search(&load_all(&app.data), &q);
            let body = serde_json::json!({ "query": q, "count": hits.len(), "models": hits });
            respond(&mut stream, 200, "application/json", body.to_string().as_bytes()).await
        }
        ("GET", p) if p.starts_with("/models/") => {
            let name = &p["/models/".len()..];
            let (id, ext) = match name.rsplit_once('.') {
                Some((i, e)) => (i, e),
                None => (name, "json"),
            };
            if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.is_empty() {
                return respond(&mut stream, 400, "text/plain", b"bad design id").await;
            }
            let (file, ctype) = match ext {
                "glb" => (format!("{id}.glb"), "model/gltf-binary"),
                "png" => (format!("{id}.png"), "image/png"),
                _ => (format!("{id}.json"), "application/json"),
            };
            match std::fs::read(app.data.join(&file)) {
                Ok(bytes) => respond(&mut stream, 200, ctype, &bytes).await,
                Err(_) => respond(&mut stream, 404, "text/plain", b"no such model").await,
            }
        }
        ("POST", "/publish") => {
            if let Some(tok) = &app.token {
                let ok = header("authorization")
                    .is_some_and(|a| a.strip_prefix("Bearer ") == Some(tok.as_str()));
                if !ok {
                    return respond(&mut stream, 401, "text/plain", b"bad token").await;
                }
            }
            let want: usize = header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
            if want == 0 || want > 8 * 1024 * 1024 {
                return respond(&mut stream, 413, "text/plain", b"bad content-length (max 8MB)")
                    .await;
            }
            let mut body = buf[head_end..].to_vec();
            while body.len() < want {
                let n = stream.read(&mut tmp).await?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&tmp[..n]);
            }
            let Ok(text) = String::from_utf8(body) else {
                return respond(&mut stream, 400, "text/plain", b"not utf-8").await;
            };
            let sub: Submission = match serde_json::from_str(&text) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("not a submission: {e}");
                    return respond(&mut stream, 400, "text/plain", msg.as_bytes()).await;
                }
            };
            // The gate: the Quarry MAKES the artifact. Nothing is taken on
            // faith because nothing is taken at all.
            match derive(&sub, &app.data) {
                Ok(e) => {
                    println!("published {} — {} ({} tris)", e.design, e.title, e.artifact.tris);
                    let body = serde_json::to_string(&e).unwrap_or_default();
                    respond(&mut stream, 200, "application/json", body.as_bytes()).await
                }
                Err(err) => {
                    let msg = format!("REFUSED: {err}");
                    respond(&mut stream, 422, "text/plain", msg.as_bytes()).await
                }
            }
        }
        _ => respond(&mut stream, 405, "text/plain", b"method not allowed").await,
    }
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

async fn respond(
    stream: &mut TcpStream,
    status: u16,
    ctype: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        431 => "Request Header Fields Too Large",
        _ => "",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\naccess-control-allow-origin: *\r\ncache-control: public, max-age=31536000, immutable\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}
