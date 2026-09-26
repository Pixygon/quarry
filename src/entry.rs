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
    /// The Codex entity this was made for — the slug, so provenance points
    /// back at the lore the model is answering to.
    #[serde(default)]
    pub codex: String,
    /// The concept image to judge it against. A URL, because most of the
    /// Codex is sealed and a browser arrives at the Quarry anonymous: the
    /// gallery link is public, the entity behind it is not.
    #[serde(default)]
    pub concept: String,
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

/// What a person said when they looked at it next to the concept. Kept as a
/// list, newest last: a "not it" is a task waiting to be written, and the
/// history says whether a republish actually fixed it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Verdict {
    /// `yes` — matches concept · `close` — close, fix noted · `no` — not it.
    pub verdict: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub by: String,
    /// Unix seconds. The Quarry stamps it; a client cannot claim a time.
    pub at: u64,
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
    /// Which system made it: `chisel` (carved), `grove` (grown), `avatar`.
    /// Derived from the package, stored so nobody has to re-derive it.
    #[serde(default)]
    pub supplier: String,
    #[serde(default)]
    pub codex: String,
    #[serde(default)]
    pub concept: String,
    /// Judgements, oldest first. Empty until somebody looks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verdicts: Vec<Verdict>,
    /// When the Quarry last ran this recipe. Unix seconds.
    #[serde(default)]
    pub derived_at: u64,
}

/// Which of the three systems a package comes from.
pub fn supplier_of(package: &str) -> &'static str {
    match package {
        "grove" => "grove",
        "avatar" => "avatar",
        _ => "chisel",
    }
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Run the recipe and store what comes out: the entry, the `.glb`, and a
/// preview sheet. Identity is the hash of the recipe, so publishing the same
/// recipe twice is idempotent — the shelf never fills with near-duplicates
/// of the same design.
pub fn derive(sub: &Submission, data: &Path) -> Result<Entry, String> {
    make(sub, data, "/models", true)
}

/// Run the recipe and keep nothing on the shelf — the "look before you
/// publish" door. The artifact still has to exist for a browser to open it,
/// so it lands in a scratch drawer served at `/derived/…` and swept when it
/// gets crowded; it is not an entry, it is not searchable, it is a look.
pub fn derive_scratch(sub: &Submission, scratch: &Path) -> Result<Entry, String> {
    std::fs::create_dir_all(scratch).map_err(|e| format!("cannot open the scratch drawer: {e}"))?;
    sweep(scratch, 40);
    make(sub, scratch, "/derived", false)
}

/// Keep the scratch drawer from becoming a shelf nobody swept: oldest
/// designs out first, by the mtime of their `.glb`.
fn sweep(scratch: &Path, keep: usize) {
    let mut glbs: Vec<(std::time::SystemTime, std::path::PathBuf)> = std::fs::read_dir(scratch)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".glb"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if glbs.len() <= keep {
        return;
    }
    glbs.sort_by_key(|(t, _)| *t);
    for (_, path) in glbs.iter().take(glbs.len() - keep) {
        let _ = std::fs::remove_file(path);
        for ext in ["png", "json"] {
            let _ = std::fs::remove_file(path.with_extension(ext));
        }
    }
}

fn make(sub: &Submission, dir: &Path, url: &str, shelve: bool) -> Result<Entry, String> {
    if sub.title.trim().is_empty() {
        return Err("a model needs a title".into());
    }
    // Two suppliers so far: the built-in Weft modelling library (carved,
    // inorganic) and Grove (grown: trees and what hangs on them). Third-party
    // wpm packages come later. Sockets are MEASURED where the recipe yields
    // them (a grown tree ends in tips); declared ones are kept otherwise.
    let (built, sockets, lod_models): (chisel::model::Built, Vec<Socket>, Vec<chisel::model::Built>) =
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
                (grown.built, sockets, grown.lods)
            }
            other => {
                return Err(format!("unknown package '{other}' — 'weft-model' and 'grove' are available in this Quarry"));
            }
        };
    let glb = chisel::model::export_glb(&built)?;

    let design = design_id(sub);
    std::fs::write(dir.join(format!("{design}.glb")), &glb)
        .map_err(|e| format!("cannot store artifact: {e}"))?;
    let mut lods: Vec<String> = Vec::new();
    for (i, lod_built) in lod_models.iter().enumerate() {
        let lod_glb = chisel::model::export_glb(lod_built)?;
        let file = format!("{design}.lod{}.glb", i + 1);
        std::fs::write(dir.join(&file), &lod_glb).map_err(|e| format!("cannot store lod: {e}"))?;
        lods.push(format!("{url}/{file}"));
    }
    let preview_opts = chisel::preview::PreviewOptions { width: 384, height: 384, views: 3, ..Default::default() };
    let png_path = dir.join(format!("{design}.png"));
    let _ = chisel::preview::write_png(&built, preview_opts, &png_path.to_string_lossy());

    let (min, max) = built.bounds();
    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    // Re-deriving a design must not erase what people said about it. The
    // recipe is the identity; the verdicts and the concept it answers to
    // belong to the design, not to this run of the grower.
    let before: Option<Entry> = shelve
        .then(|| std::fs::read_to_string(dir.join(format!("{design}.json"))).ok())
        .flatten()
        .and_then(|t| serde_json::from_str(&t).ok());
    let kept = |new: &str, old: fn(&Entry) -> &String| -> String {
        if !new.is_empty() {
            return new.to_string();
        }
        before.as_ref().map(|e| old(e).clone()).unwrap_or_default()
    };
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
            url: format!("{url}/{design}.glb"),
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
        preview: format!("{url}/{design}.png"),
        license: sub.license.clone(),
        author: sub.author.clone(),
        origin: sub.origin.clone(),
        supplier: supplier_of(&sub.package).to_string(),
        codex: kept(&sub.codex, |e| &e.codex),
        concept: kept(&sub.concept, |e| &e.concept),
        verdicts: before.as_ref().map(|e| e.verdicts.clone()).unwrap_or_default(),
        derived_at: now(),
    };
    if shelve {
        let json = serde_json::to_string_pretty(&entry).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{design}.json")), json)
            .map_err(|e| format!("cannot store entry: {e}"))?;
    }
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

/// A grow recipe reduced to what it actually says: parsed, then every field
/// equal to the grower's default dropped. Two spellings of one tree (terse,
/// exhaustive) are one design — and a new recipe field with a default does
/// not fork every existing design (the 2026-09-26 `leaves` lesson).
fn canonical_recipe(sub: &Submission) -> Option<serde_json::Value> {
    if sub.package != "grove" {
        return None;
    }
    let r: grove::GrowRecipe = serde_json::from_value(sub.recipe.clone()?).ok()?;
    let mut full = serde_json::to_value(r).ok()?;
    let defaults = serde_json::to_value(grove::GrowRecipe::default()).ok()?;
    if let (Some(obj), Some(def)) = (full.as_object_mut(), defaults.as_object()) {
        obj.retain(|k, v| def.get(k) != Some(v));
    }
    Some(full)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("quarry-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp dir");
        d
    }

    fn column(title: &str, height: f32) -> Submission {
        Submission {
            title: title.into(),
            description: String::new(),
            tags: vec!["column".into()],
            kind: "column".into(),
            style: String::new(),
            package: "weft-model".into(),
            export: "column".into(),
            args: vec![serde_json::json!(height), serde_json::json!(0.44)],
            recipe: None,
            material: "marble".into(),
            license: "CC0-1.0".into(),
            author: String::new(),
            origin: "authored".into(),
            sockets: Vec::new(),
            codex: String::new(),
            concept: String::new(),
        }
    }

    /// The design is the hash of the RECIPE. What a publisher calls it, tags
    /// it or licenses it must not fork the shelf into near-duplicates.
    #[test]
    fn words_do_not_change_the_design() {
        let mut a = column("Doric column, 5.2 m", 5.2);
        let mut b = column("A handsome weathered column", 5.2);
        b.tags = vec!["fancy".into()];
        b.license = "MIT".into();
        b.author = "someone else".into();
        b.kind = "pillar".into();
        assert_eq!(design_id(&a), design_id(&b));
        a.material = "granite".into();
        assert_ne!(design_id(&a), design_id(&b), "the material is part of the recipe");
    }

    /// 5.2 and 5.200000001 are one column, not two.
    #[test]
    fn float_noise_does_not_fork_a_design() {
        let a = column("c", 5.2);
        let b = column("c", 5.200_000_1);
        assert_eq!(design_id(&a), design_id(&b));
        assert_ne!(design_id(&a), design_id(&column("c", 5.3)));
    }

    /// The lesson of 2026-09-26: the Quarry hashed the DEFAULT-FILLED grow
    /// recipe, so adding one field with a default renamed every grown design
    /// and orphaned five of them. A terse spelling and an exhaustive one are
    /// the same tree, and a new rule nobody set must not rename anything.
    #[test]
    fn a_grove_design_hashes_only_what_the_recipe_says() {
        let terse = serde_json::json!({ "name": "oak", "seed": 5, "height": 2.2 });
        let mut exhaustive = serde_json::to_value(grove::GrowRecipe::default()).unwrap();
        exhaustive["name"] = serde_json::json!("oak");
        exhaustive["seed"] = serde_json::json!(5);
        exhaustive["height"] = serde_json::json!(2.2);

        let mut a = column("Oak", 1.0);
        a.package = "grove".into();
        a.export = String::new();
        a.args = Vec::new();
        a.material = String::new();
        let mut b = a.clone();
        a.recipe = Some(terse);
        b.recipe = Some(exhaustive);
        assert_eq!(design_id(&a), design_id(&b), "one tree, two spellings");

        // …and a rule left at its default contributes nothing to the id.
        let canon = canonical_recipe(&a).expect("a grove recipe canonicalises");
        let obj = canon.as_object().expect("an object");
        assert!(obj.contains_key("height"), "a set rule is kept");
        assert!(!obj.contains_key("taper"), "a defaulted rule is dropped: {obj:?}");
    }

    /// Re-deriving is how a grower change reaches the shelf. The judgement
    /// and the concept belong to the design, not to the run that made it.
    #[test]
    fn re_deriving_keeps_the_verdicts_and_the_concept() {
        let data = tmp("carry");
        let mut sub = column("Doric column, 5.2 m", 5.2);
        sub.codex = "lantern-desert".into();
        sub.concept = "https://example.test/concept.jpg".into();
        let first = derive(&sub, &data).expect("derives");
        assert!(first.verdicts.is_empty());

        // a person looks at it and says something
        let mut stored = first.clone();
        stored.verdicts.push(Verdict {
            verdict: "close".into(),
            note: "shaft is thin".into(),
            by: String::new(),
            at: 1,
        });
        std::fs::write(
            data.join(format!("{}.json", stored.design)),
            serde_json::to_string(&stored).unwrap(),
        )
        .unwrap();

        // the same recipe, republished with no words about the codex at all
        let mut again = column("Doric column, 5.2 m", 5.2);
        again.title = "Renamed".into();
        let second = derive(&again, &data).expect("re-derives");
        assert_eq!(second.design, first.design);
        assert_eq!(second.verdicts.len(), 1, "the verdict survived the republish");
        assert_eq!(second.verdicts[0].note, "shaft is thin");
        assert_eq!(second.codex, "lantern-desert", "the codex link survived");
        assert_eq!(second.concept, "https://example.test/concept.jpg");
        assert_eq!(second.title, "Renamed", "but the words are the new ones");
        assert_eq!(second.supplier, "chisel");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// "Look before you publish" means exactly that: an artifact to open, and
    /// nothing on the shelf.
    #[test]
    fn a_scratch_derive_is_not_an_entry() {
        let scratch = tmp("scratch");
        let e = derive_scratch(&column("Probe", 3.0), &scratch).expect("derives");
        assert!(e.artifact.url.starts_with("/derived/"), "{}", e.artifact.url);
        assert!(e.preview.starts_with("/derived/"));
        assert!(scratch.join(format!("{}.glb", e.design)).exists());
        let entries: Vec<_> = std::fs::read_dir(&scratch)
            .unwrap()
            .flatten()
            .filter(|f| f.file_name().to_string_lossy().ends_with(".json"))
            .collect();
        assert!(entries.is_empty(), "a look is not an entry: {entries:?}");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// The drawer is swept, or it becomes a shelf nobody curates.
    #[test]
    fn the_scratch_drawer_is_swept() {
        let scratch = tmp("sweep");
        for i in 0..6u32 {
            std::fs::write(scratch.join(format!("{i:016x}.glb")), b"x").unwrap();
            std::fs::write(scratch.join(format!("{i:016x}.png")), b"x").unwrap();
            // mtime resolution is coarse; make the order unambiguous
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        sweep(&scratch, 2);
        let left: Vec<String> = std::fs::read_dir(&scratch)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".glb"))
            .collect();
        assert_eq!(left.len(), 2, "kept: {left:?}");
        assert!(left.iter().all(|n| n.starts_with("0000000000000004") || n.starts_with("0000000000000005")),
                "the newest two stay: {left:?}");
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// Facts are measured off the mesh, not copied from the submission.
    #[test]
    fn the_facts_are_measured() {
        let data = tmp("facts");
        let e = derive(&column("Doric column, 5.2 m", 5.2), &data).expect("derives");
        assert!((e.facts.size[1] - 5.2).abs() < 0.01, "height is the mesh's: {:?}", e.facts.size);
        assert_eq!(e.facts.origin, "base", "a column rests on y = 0");
        assert_eq!(e.facts.collider, "cylinder", "tall and square in plan");
        assert_eq!(e.facts.front, "+z");
        assert!(e.artifact.tris > 0 && e.artifact.bytes > 0);
        assert_eq!(e.artifact.sha256.len(), 64);
        assert_eq!(e.facts.materials, vec!["marble".to_string()]);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn a_submission_without_a_title_is_refused() {
        let data = tmp("untitled");
        let mut sub = column("  ", 5.2);
        sub.title = "  ".into();
        assert!(derive(&sub, &data).is_err());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn an_unknown_package_is_refused_by_name() {
        let data = tmp("unknown");
        let mut sub = column("x", 1.0);
        sub.package = "speedtree".into();
        let err = derive(&sub, &data).unwrap_err();
        assert!(err.contains("speedtree"), "{err}");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// Search is for a layout engine: the wrong kind is a wrong answer, not a
    /// near miss, and the right size outranks the wrong one.
    #[test]
    fn search_ranks_by_fit_and_refuses_the_wrong_kind() {
        let data = tmp("search");
        let tall = derive(&column("Tall", 5.2), &data).expect("derives");
        let short = derive(&column("Short", 3.0), &data).expect("derives");
        let all = vec![tall.clone(), short.clone()];

        let q: BTreeMap<String, String> = [("kind", "column"), ("h", "3.0"), ("tol", "0.3")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let hits = search(&all, &q);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0]["design"], serde_json::json!(short.design), "the 3 m one fits 3 m");
        assert_eq!(hits[0]["fit"], serde_json::json!("exact"));

        let q: BTreeMap<String, String> = [("kind", "amphora")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert!(search(&all, &q).is_empty(), "a column is not an amphora");
        let _ = std::fs::remove_dir_all(&data);
    }
}
