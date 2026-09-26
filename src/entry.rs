//! What the Quarry stores: a **design** (the recipe), its **artifact** (the
//! `.glb` the Quarry itself derived) and its **facts** (what a machine needs
//! in order to place it without looking at it).
//!
//! The split matters. `title`/`description`/`tags` are for people and for
//! search; `facts` are for the layout engine, which cannot use "a handsome
//! weathered column" but can use `size`, `origin: base`, `front`, `collider`
//! and `sockets`. Everything in `facts` is measured from the derived mesh —
//! it is observed, never claimed.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// What a publisher sends: words, and how to make the thing.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Submission {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The machine-usable category: `column`, `stairs`, `table`, `rock`…
    #[serde(default)]
    pub kind: String,
    /// Family the layout engine keeps coherent: `classical`, `rustic`…
    #[serde(default)]
    pub style: String,
    /// The recipe: a package on wpm (or the built-in `weft-model`), one of
    /// its exports, and the arguments.
    #[serde(default = "weft_model")]
    pub package: String,
    #[serde(default)]
    pub export: String,
    #[serde(default)]
    pub args: Vec<serde_json::Value>,
    /// For `package: "grove"`: the grow recipe itself (a `GrowRecipe`), the
    /// whole bill of rules a tree is grown from. The Quarry grows it here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<serde_json::Value>,
    /// Material export to dress a bare part in (empty = the model brings its own).
    #[serde(default)]
    pub material: String,
    #[serde(default = "cc0")]
    pub license: String,
    #[serde(default)]
    pub author: String,
    /// `authored` · `seed` · `commissioned` (a layout asked for it).
    #[serde(default = "authored")]
    pub origin: String,
    /// Declared attachment points, if the publisher knows them.
    #[serde(default)]
    pub sockets: Vec<Socket>,
}

fn weft_model() -> String {
    "weft-model".to_string()
}
fn cc0() -> String {
    "CC0-1.0".to_string()
}
fn authored() -> String {
    "authored".to_string()
}

/// Where something attaches to something else — the fact that turns
/// placement from arithmetic into matching.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Socket {
    pub name: String,
    pub at: [f32; 3],
    /// What belongs here: `capital`, `threshold`, `surface`…
    #[serde(default)]
    pub kind: String,
    /// Free size at the socket, if it constrains what fits.
    #[serde(default)]
    pub size: [f32; 3],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recipe {
    pub package: String,
    pub export: String,
    pub args: Vec<serde_json::Value>,
    #[serde(default)]
    pub material: String,
    /// The grow recipe for `grove` designs — re-derivable, same as args.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artifact {
    pub url: String,
    pub bytes: usize,
    pub tris: usize,
    pub sha256: String,
    /// Coarser LODs, nearest first, when the recipe makes them (grown things do).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lods: Vec<String>,
}

/// Measured, not claimed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Facts {
    /// Bounding size in metres.
    pub size: [f32; 3],
    /// `base` when the model rests on y = 0 (the common case for placeable
    /// things), else `center`.
    pub origin: String,
    /// Which way it faces — always `+z` for library parts, kept explicit so
    /// a layout engine never has to guess.
    pub front: String,
    pub parts: usize,
    pub materials: Vec<String>,
    /// The cheapest honest collider: `box`, `cylinder`, or `none`.
    pub collider: String,
    #[serde(default)]
    pub sockets: Vec<Socket>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entry {
    pub design: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub kind: String,
    pub style: String,
    pub recipe: Recipe,
    pub artifact: Artifact,
    pub facts: Facts,
    pub preview: String,
    pub license: String,
    pub author: String,
    pub origin: String,
}

/// Run the recipe and store what comes out: the entry, the `.glb`, and a
/// preview sheet. Identity is the hash of the recipe, so publishing the same
/// recipe twice is idempotent — the shelf never fills with near-duplicates
/// of the same design.
pub fn derive(sub: &Submission, data: &Path) -> Result<Entry, String> {
    if sub.title.trim().is_empty() {
        return Err("a model needs a title".into());
    }
    // Two suppliers so far: the built-in Weft modelling library (carved,
    // inorganic) and Grove (grown: trees and what hangs on them). Third-party
    // wpm packages come later. Sockets are MEASURED where the recipe yields
    // them (a grown tree ends in tips); declared ones are kept otherwise.
    let (built, sockets, lod_meshes): (chisel::model::Built, Vec<Socket>, Vec<chisel::MeshData>) =
        match sub.package.as_str() {
            "weft-model" => {
                let library = chisel::weft_model::standard_library();
                let material = (!sub.material.is_empty()).then_some(sub.material.as_str());
                let model = chisel::weft_model::eval_model_or_part(&library, &sub.export, &sub.args, material)?;
                (chisel::model::build(&model)?, sub.sockets.clone(), Vec::new())
            }
            "grove" => {
                let value = sub.recipe.clone().ok_or("a grove submission carries its grow recipe in `recipe`")?;
                let recipe: grove::GrowRecipe =
                    serde_json::from_value(value).map_err(|e| format!("not a grow recipe: {e}"))?;
                let grown = grove::grow(&recipe)?;
                let sockets = grown
                    .sockets
                    .iter()
                    .map(|s| Socket { name: s.name.clone(), at: s.position, kind: "tip".into(), size: [s.radius * 2.0; 3] })
                    .collect();
                let lods = grown.lods.into_iter().skip(1).collect();
                (grown.built, sockets, lods)
            }
            other => {
                return Err(format!("unknown package '{other}' — 'weft-model' and 'grove' are available in this Quarry"));
            }
        };
    let glb = chisel::model::export_glb(&built)?;

    let design = design_id(sub);
    std::fs::write(data.join(format!("{design}.glb")), &glb)
        .map_err(|e| format!("cannot store artifact: {e}"))?;
    let mut lods: Vec<String> = Vec::new();
    for (i, mesh) in lod_meshes.into_iter().enumerate() {
        let Some(part) = built.parts.first() else { break };
        let lod_built = chisel::model::Built {
            name: format!("{}-lod{}", built.name, i + 1),
            parts: vec![chisel::model::BuiltPart {
                name: part.name.clone(),
                mesh,
                baked: part.baked.clone(),
                color: part.color,
                emissive: part.emissive,
            }],
        };
        let lod_glb = chisel::model::export_glb(&lod_built)?;
        let file = format!("{design}.lod{}.glb", i + 1);
        std::fs::write(data.join(&file), &lod_glb).map_err(|e| format!("cannot store lod: {e}"))?;
        lods.push(format!("/models/{file}"));
    }
    let preview_opts = chisel::preview::PreviewOptions { width: 384, height: 384, views: 3, ..Default::default() };
    let png_path = data.join(format!("{design}.png"));
    let _ = chisel::preview::write_png(&built, preview_opts, &png_path.to_string_lossy());

    let (min, max) = built.bounds();
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let entry = Entry {
        design: design.clone(),
        title: sub.title.clone(),
        description: sub.description.clone(),
        tags: sub.tags.clone(),
        kind: if !sub.kind.is_empty() {
            sub.kind.clone()
        } else if sub.package == "grove" {
            "tree".into()
        } else {
            sub.export.clone()
        },
        style: sub.style.clone(),
        recipe: Recipe {
            package: sub.package.clone(),
            export: sub.export.clone(),
            args: sub.args.clone(),
            material: sub.material.clone(),
            recipe: canonical_recipe(sub),
        },
        artifact: Artifact {
            url: format!("/models/{design}.glb"),
            bytes: glb.len(),
            tris: built.triangles(),
            sha256: sha256_hex(&glb),
            lods,
        },
        facts: Facts {
            size,
            // A leaning trunk's base ring dips a few centimetres under y = 0;
            // it still rests on the ground. Tolerance scales with height.
            origin: if min[1].abs() < 0.02f32.max(0.01 * size[1]) { "base".into() } else { "center".into() },
            front: "+z".into(),
            parts: built.parts.len(),
            materials: built.parts.iter().map(|p| p.name.clone()).collect(),
            collider: collider_for(size, &sub.kind),
            sockets,
        },
        preview: format!("/models/{design}.png"),
        license: sub.license.clone(),
        author: sub.author.clone(),
        origin: sub.origin.clone(),
    };
    let json = serde_json::to_string_pretty(&entry).map_err(|e| e.to_string())?;
    std::fs::write(data.join(format!("{design}.json")), json)
        .map_err(|e| format!("cannot store entry: {e}"))?;
    Ok(entry)
}

/// The cheapest honest collider for a shape of this footprint. A layout
/// engine needs *something*; a box that is nearly square in plan and much
/// taller than wide is better served by a cylinder (columns, vases).
fn collider_for(size: [f32; 3], kind: &str) -> String {
    if matches!(kind, "prop" | "vessel" | "scatter") && size[1] < 0.4 {
        return "none".into();
    }
    let square = (size[0] - size[2]).abs() < 0.15 * size[0].max(size[2]).max(0.001);
    if square && size[1] > size[0] * 1.5 {
        "cylinder".into()
    } else {
        "box".into()
    }
}

/// The design id: a hash of the recipe alone. Same recipe → same design, no
/// matter who published it or what they called it.
fn design_id(sub: &Submission) -> String {
    let mut canonical = serde_json::json!({
        "package": sub.package,
        "export": sub.export,
        "args": sub.args.iter().map(canonical_number).collect::<Vec<_>>(),
        "material": sub.material,
    });
    if let Some(r) = canonical_recipe(sub) {
        canonical["recipe"] = r;
    }
    sha256_hex(canonical.to_string().as_bytes())[..16].to_string()
}

/// A grow recipe with every default filled in, so two spellings of the same
/// tree (one terse, one exhaustive) are one design.
fn canonical_recipe(sub: &Submission) -> Option<serde_json::Value> {
    if sub.package != "grove" {
        return None;
    }
    let r: grove::GrowRecipe = serde_json::from_value(sub.recipe.clone()?).ok()?;
    serde_json::to_value(r).ok()
}

/// Float noise must not fork a design: 5.200000001 and 5.2 are one thing.
fn canonical_number(v: &serde_json::Value) -> serde_json::Value {
    match v.as_f64() {
        Some(f) => serde_json::json!((f * 10_000.0).round() / 10_000.0),
        None => v.clone(),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Search, ranked by **fit** — the order a layout engine actually wants:
/// the right kind first, then the right size, then the right style, with
/// everything it needs to decide carried in the result.
pub fn search(entries: &[Entry], q: &BTreeMap<String, String>) -> Vec<serde_json::Value> {
    let num = |k: &str| q.get(k).and_then(|v| v.parse::<f32>().ok());
    let (want_w, want_h, want_d) = (num("w"), num("h"), num("d"));
    let tol = num("tol").unwrap_or(0.25);
    let kind = q.get("kind").map(|s| s.to_lowercase());
    let style = q.get("style").map(|s| s.to_lowercase());
    let tags: Vec<String> = q
        .get("tags")
        .map(|t| t.split(',').map(|s| s.trim().to_lowercase()).collect())
        .unwrap_or_default();
    let text = q.get("q").map(|s| s.to_lowercase());
    let limit = q.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(10).min(100);

    let mut scored: Vec<(f32, &Entry, String)> = Vec::new();
    for e in entries {
        let mut score = 0.0f32;
        let mut fit = "exact".to_string();

        if let Some(k) = &kind {
            if e.kind.to_lowercase() == *k {
                score += 4.0;
            } else if e.tags.iter().any(|t| t.to_lowercase() == *k) {
                score += 2.0;
            } else {
                continue; // wrong kind is not a near miss, it is a wrong answer
            }
        }
        if let Some(s) = &style {
            if e.style.to_lowercase() == *s || e.tags.iter().any(|t| t.to_lowercase() == *s) {
                score += 2.0;
            } else if !s.is_empty() {
                score -= 1.5; // off-palette: allowed, never preferred
            }
        }
        for t in &tags {
            if e.tags.iter().any(|x| x.to_lowercase() == *t) {
                score += 0.75;
            }
        }
        if let Some(t) = &text {
            let hay = format!("{} {} {}", e.title, e.description, e.tags.join(" ")).to_lowercase();
            if hay.contains(t) {
                score += 1.0;
            }
        }

        // Size fit: exact wins, in-tolerance is fine, a uniform scale within
        // 15 % is acceptable, and anything beyond that is a commission — we
        // never squash a model to make it fit.
        let mut worst_ratio = 1.0f32;
        let mut within_tol = true;
        for (want, have) in [(want_w, e.facts.size[0]), (want_h, e.facts.size[1]), (want_d, e.facts.size[2])] {
            let Some(want) = want else { continue };
            if want <= 0.0 {
                continue;
            }
            if (want - have).abs() > tol {
                within_tol = false;
                let ratio = (want / have.max(0.001)).max(have / want.max(0.001));
                worst_ratio = worst_ratio.max(ratio);
            }
        }
        if within_tol {
            score += 3.0;
        } else if worst_ratio <= 1.15 {
            score += 1.0;
            fit = "scale".to_string();
        } else {
            fit = "poor".to_string();
            score -= 2.0 * worst_ratio.min(4.0);
        }

        scored.push((score, e, fit));
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(score, e, fit)| {
            let mut v = serde_json::to_value(e).unwrap_or_default();
            v["score"] = serde_json::json!((score * 100.0).round() / 100.0);
            v["fit"] = serde_json::json!(fit);
            v
        })
        .collect()
}
