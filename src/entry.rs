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
    /// For `package: "avatar"`: the manifested GLB itself. It arrives as the
    /// request body, never as JSON, so serde does not see it.
    #[serde(skip)]
    pub glb: Option<Vec<u8>>,
    /// For `package: "avatar"` without a body: re-check the GLB already on
    /// the shelf under this design (what Republish sends).
    #[serde(default)]
    pub source: String,
    /// Stand the carved thing on the ground: the library centres its
    /// primitives, and a block that is half underground when placed is no
    /// use to a layout engine. Part of the recipe, so part of the design.
    #[serde(default)]
    pub rest: bool,
    /// For `package: "grove"`: something on the shelf to hang at the plant's
    /// sockets — a lantern at every fruit socket. Part of the recipe: a tree
    /// with its lanterns is a different design from the bare tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hang: Option<Hang>,
}

/// What hangs where. `design` is a Quarry design on the shelf; the rest is
/// Grove's own `HangRecipe`, flat, defaults omitted.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hang {
    pub design: String,
    #[serde(flatten)]
    pub rule: grove::HangRecipe,
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
    /// Carved and stood on the ground. Part of the recipe.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rest: bool,
    /// What hangs at the sockets, and how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hang: Option<Hang>,
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
    /// Grown things only: where in its life and its year the plant stands
    /// at this moment. Measured by the grower, like everything else here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub life: Option<Life>,
    /// Avatar items only: what the file says it is, what the file actually
    /// contains, and how it did against the Portable Item Convention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<Item>,
    /// Grown things with something hung at their sockets: where each one
    /// sits. The artifact is the bare plant; an importer instances the hung
    /// design at these, exactly as the table does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hung: Option<Hung>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hung {
    /// The design instanced at each placement.
    pub design: String,
    pub kind: String,
    /// How many were hung — the sockets of that kind the plant had, capped
    /// by the recipe's `count`. Out of season a fruit tree hangs nothing.
    pub count: usize,
    pub placements: Vec<grove::Placement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Item {
    /// partId — the shared key between Unity, the web and a saved avatar.
    pub id: u32,
    /// `garment` · `weapon` · `body` · `part` · `consumable`.
    pub kind: String,
    pub slot: String,
    /// `skinned` · `bone`, measured off the file.
    pub attach: String,
    /// Joint names, in joint order — what a rebind by name must match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bones: Vec<String>,
    pub skins: usize,
    pub textures: usize,
    /// Every rule of the convention, and how the file did.
    pub checks: Vec<crate::avatar::Check>,
    /// How many of them failed. Zero is what "adheres" means.
    pub failed: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Life {
    /// `sapling` · `mature` · `old` · `dying`.
    pub stage: String,
    /// `bud` · `leaf` · `bloom` · `fruit` · `seeddrop` · `bare`.
    pub phase: String,
    /// 0..1 — at 1 it is the whole potential plant the seed decided.
    pub maturity: f32,
    /// How many branches it has at this moment.
    pub branches: usize,
}

/// A grower enum as the entry spells it: the lowercase serde name, which is
/// the one a client would type back.
fn kind_name<T: Serialize>(k: T) -> String {
    serde_json::to_value(k)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
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
    make(sub, data, data, "/models", true)
}

/// Run the recipe and keep nothing on the shelf — the "look before you
/// publish" door. The artifact still has to exist for a browser to open it,
/// so it lands in a scratch drawer served at `/derived/…` and swept when it
/// gets crowded; it is not an entry, it is not searchable, it is a look.
pub fn derive_scratch(sub: &Submission, scratch: &Path, shelf: &Path) -> Result<Entry, String> {
    std::fs::create_dir_all(scratch).map_err(|e| format!("cannot open the scratch drawer: {e}"))?;
    sweep(scratch, 40);
    make(sub, scratch, shelf, "/derived", false)
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

/// `dir` is where this run's files go; `shelf` is where designs already
/// published live — the same place when shelving, the data dir when a look
/// in the scratch drawer needs something to hang.
fn make(sub: &Submission, dir: &Path, shelf: &Path, url: &str, shelve: bool) -> Result<Entry, String> {
    // An avatar item names itself in its manifest; everything else must be
    // told what it is called.
    if sub.title.trim().is_empty() && sub.package != "avatar" {
        return Err("a model needs a title".into());
    }
    // Two suppliers so far: the built-in Weft modelling library (carved,
    // inorganic) and Grove (grown: trees and what hangs on them). Third-party
    // wpm packages come later. Sockets are MEASURED where the recipe yields
    // them (a grown tree ends in tips); declared ones are kept otherwise.
    let mut life: Option<Life> = None;
    let mut hung: Option<Hung> = None;
    let mut hung_thing: Option<(chisel::model::Built, Vec<grove::Placement>)> = None;
    let mut item: Option<Item> = None;
    let mut original: Option<Vec<u8>> = None;
    let mut manifest: Option<serde_json::Value> = None;
    let (built, sockets, lod_models): (chisel::model::Built, Vec<Socket>, Vec<chisel::model::Built>) =
        match sub.package.as_str() {
            "weft-model" => {
                let library = chisel::weft_model::standard_library();
                let material = (!sub.material.is_empty()).then_some(sub.material.as_str());
                let model = chisel::weft_model::eval_model_or_part(&library, &sub.export, &sub.args, material)?;
                let mut built = chisel::model::build(&model)?;
                // Coarser LODs from the same recipe: the carving grid and the
                // bake at a half and a quarter. A 21 000-triangle sphere is
                // right up close and absurd at fifty metres; Grove ships LODs
                // and carved things should too.
                let mut lods = Vec::new();
                for divisor in [2u32, 4] {
                    let mut coarse = model.clone();
                    for m in &mut coarse.materials {
                        m.resolution = Some((m.resolution.unwrap_or(48) / divisor).max(8));
                        if let Some(t) = &mut m.texture {
                            t.size = (t.size / divisor).max(64);
                        }
                    }
                    lods.push(chisel::model::build(&coarse)?);
                }
                if sub.rest {
                    // One shift for every level, measured off LOD0, so they
                    // stay in register.
                    let (min, _) = built.bounds();
                    let lift = -min[1];
                    for b in std::iter::once(&mut built).chain(lods.iter_mut()) {
                        for p in &mut b.parts {
                            for v in &mut p.mesh.positions {
                                v[1] += lift;
                            }
                        }
                    }
                }
                (built, sub.sockets.clone(), lods)
            }
            "grove" => {
                // A plant is a species, a seed and a clock — one flat object
                // in the submission, exactly as a `.grow.json` spells it. The
                // grower reads it; the Quarry does not second-guess the shape.
                let value = sub.recipe.clone().ok_or("a grove submission carries its grow recipe in `recipe`")?;
                let planting = grove::Planting::from_value(&value)?;
                let grown = grove::grow_planting(&planting)?;
                let sockets = grown
                    .sockets
                    .iter()
                    .map(|s| Socket {
                        name: s.name.clone(),
                        at: s.position,
                        kind: kind_name(s.kind),
                        size: [s.radius * 2.0; 3],
                    })
                    .collect();
                life = Some(Life {
                    stage: kind_name(grown.stage),
                    phase: kind_name(grown.phase),
                    maturity: grown.maturity,
                    branches: grown.branches.len(),
                });
                if let Some(h) = &sub.hang {
                    // The hung thing is a design on the shelf. Its own top
                    // decides how far under the tip it sits, so a lantern
                    // with its origin at the base hangs as well as one with
                    // it at the hook.
                    if !is_design(&h.design) {
                        return Err(format!("`{}` is not a design id", h.design));
                    }
                    let bytes = std::fs::read(shelf.join(format!("{}.glb", h.design)))
                        .map_err(|_| format!("nothing on the shelf under design {} to hang", h.design))?;
                    let thing = crate::avatar::built_from_glb(&bytes, &h.design)?;
                    let (_, tmax) = thing.bounds();
                    let placements = grove::hang(&grown.sockets, tmax[1], &h.rule);
                    hung_thing = Some((thing, placements.clone()));
                    hung = Some(Hung {
                        design: h.design.clone(),
                        kind: kind_name(h.rule.kind),
                        count: placements.len(),
                        placements,
                    });
                }
                (grown.built, sockets, grown.lods)
            }
            "avatar" => {
                // Not derived: checked. The file is the item, the convention
                // is the recipe it has to keep, and the bytes are stored
                // exactly as they came — a consumer must get the same file
                // the maker saw.
                let bytes = match (&sub.glb, sub.source.is_empty()) {
                    (Some(b), _) => b.clone(),
                    (None, false) => std::fs::read(dir.join(format!("{}.glb", sub.source)))
                        .map_err(|_| format!("no GLB on the shelf under design {}", sub.source))?,
                    (None, true) => return Err("an avatar submission is a manifested GLB — send the file as the body (content-type model/gltf-binary)".into()),
                };
                let checked = crate::avatar::check(&bytes)?;
                let m = &checked.manifest;
                let failed = checked.checks.iter().filter(|c| !c.ok).count();
                item = Some(Item {
                    id: m.id,
                    kind: kind_name(m.kind),
                    slot: m.slot.clone(),
                    attach: checked.attach.clone(),
                    bones: checked.bones.clone(),
                    skins: checked.skins,
                    textures: checked.textures,
                    checks: checked.checks.clone(),
                    failed,
                });
                original = Some(bytes);
                manifest = Some(serde_json::to_value(m).unwrap_or_default());
                (checked.built, Vec::new(), Vec::new())
            }
            other => {
                return Err(format!("unknown package '{other}' — 'weft-model', 'grove' and 'avatar' are available in this Quarry"));
            }
        };
    let glb = match original {
        Some(bytes) => bytes,
        None => chisel::model::export_glb(&built)?,
    };

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
    match &hung_thing {
        // The turntable shows the tree WITH its lanterns — that is what a
        // person judges against the concept — while the stored glb stays
        // the bare plant an importer instances onto.
        Some((thing, placements)) => {
            let shown = compose(&built, thing, placements);
            let _ = chisel::preview::write_png(&shown, preview_opts, &png_path.to_string_lossy());
        }
        None => {
            let _ = chisel::preview::write_png(&built, preview_opts, &png_path.to_string_lossy());
        }
    }

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
    let m_str = |k: &str| manifest.as_ref().and_then(|m| m.get(k)).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut tags = sub.tags.clone();
    if item.is_some() {
        for t in [m_str("kind"), m_str("slot").to_lowercase(), "avatar".into()] {
            if !t.is_empty() && !tags.contains(&t) {
                tags.push(t);
            }
        }
    }
    let entry = Entry {
        design: design.clone(),
        title: if !sub.title.trim().is_empty() { sub.title.clone() } else { m_str("title") },
        description: if !sub.description.is_empty() { sub.description.clone() } else { m_str("description") },
        tags,
        kind: if !sub.kind.is_empty() {
            sub.kind.clone()
        } else if sub.package == "grove" {
            "tree".into()
        } else if item.is_some() {
            m_str("kind")
        } else {
            sub.export.clone()
        },
        style: sub.style.clone(),
        recipe: Recipe {
            package: sub.package.clone(),
            export: sub.export.clone(),
            args: sub.args.clone(),
            material: sub.material.clone(),
            recipe: if item.is_some() { manifest.clone() } else { canonical_recipe(sub) },
            rest: sub.rest,
            hang: sub.hang.clone(),
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
            life,
            item,
            hung,
        },
        preview: format!("{url}/{design}.png"),
        license: sub.license.clone(),
        author: sub.author.clone(),
        origin: sub.origin.clone(),
        supplier: supplier_of(&sub.package).to_string(),
        // The manifest names the Codex entry the item IS; that is the concept
        // to judge it against unless somebody attached a different one.
        codex: {
            let c = kept(&sub.codex, |e| &e.codex);
            if c.is_empty() { m_str("codexSlug") } else { c }
        },
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
    // An item has no recipe but the file; the same bytes are the same design.
    if sub.package == "avatar" {
        if let Some(b) = &sub.glb {
            return sha256_hex(b)[..16].to_string();
        }
        if !sub.source.is_empty() {
            return sub.source.clone();
        }
    }
    let mut canonical = serde_json::json!({
        "package": sub.package,
        "export": sub.export,
        "args": sub.args.iter().map(canonical_number).collect::<Vec<_>>(),
        "material": sub.material,
    });
    // Only when set: every design published before this flag existed keeps
    // its name.
    if sub.rest {
        canonical["rest"] = serde_json::json!(true);
    }
    if let Some(h) = &sub.hang {
        canonical["hang"] = canonical_hang(h);
    }
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
    // The grower's own flat spelling — species fields, `seed`, `age`,
    // `season`, and the state — stripped of everything a blank planting
    // already says. `seed` and `age` stay in when they are set, because a
    // different individual or a sapling IS a different design; a field the
    // grower gained yesterday, left at its default, is not.
    let p = grove::Planting::from_value(sub.recipe.as_ref()?).ok()?;
    let full = strip_blank(canonical_numbers(p.to_value()));
    let defaults = strip_blank(canonical_numbers(grove::Planting::default().to_value()));
    let (Some(mut obj), Some(def)) = (full.as_object().cloned(), defaults.as_object()) else {
        return Some(full);
    };
    obj.retain(|k, v| def.get(k) != Some(v));
    Some(serde_json::Value::Object(obj))
}

/// A hang as the recipe says it: the design, and only the rules that
/// differ from Grove's default — so a knob the grower gains tomorrow does
/// not rename every tree with lanterns.
fn canonical_hang(h: &Hang) -> serde_json::Value {
    let mut v = canonical_numbers(serde_json::to_value(&h.rule).unwrap_or_default());
    let blank = canonical_numbers(serde_json::to_value(grove::HangRecipe::default()).unwrap_or_default());
    if let (Some(obj), Some(b)) = (v.as_object_mut(), blank.as_object()) {
        obj.retain(|k, x| b.get(k) != Some(x));
        obj.insert("design".into(), serde_json::json!(h.design));
    }
    v
}

/// The plant with the hung thing at every placement, as one model — for the
/// turntable. Instances are baked here and only here.
fn compose(plant: &chisel::model::Built, thing: &chisel::model::Built, placements: &[grove::Placement]) -> chisel::model::Built {
    let mut parts: Vec<chisel::model::BuiltPart> = plant
        .parts
        .iter()
        .map(|p| chisel::model::BuiltPart {
            name: p.name.clone(),
            mesh: p.mesh.clone(),
            baked: p.baked.clone(),
            color: p.color,
            emissive: p.emissive,
            double_sided: p.double_sided,
        })
        .collect();
    for pl in placements {
        let [qx, qy, qz, qw] = pl.rotation;
        for p in &thing.parts {
            let mut mesh = p.mesh.clone();
            for v in &mut mesh.positions {
                let s = [v[0] * pl.scale[0], v[1] * pl.scale[1], v[2] * pl.scale[2]];
                let r = rotate_q(s, qx, qy, qz, qw);
                *v = [r[0] + pl.translation[0], r[1] + pl.translation[1], r[2] + pl.translation[2]];
            }
            for n in &mut mesh.normals {
                *n = rotate_q(*n, qx, qy, qz, qw);
            }
            parts.push(chisel::model::BuiltPart {
                name: format!("{}@{}", p.name, pl.socket),
                mesh,
                baked: p.baked.clone(),
                color: p.color,
                emissive: p.emissive,
                double_sided: p.double_sided,
            });
        }
    }
    chisel::model::Built { name: plant.name.clone(), parts }
}

fn rotate_q(v: [f32; 3], x: f32, y: f32, z: f32, w: f32) -> [f32; 3] {
    // v' = v + 2w(q × v) + 2(q × (q × v))
    let c1 = [y * v[2] - z * v[1], z * v[0] - x * v[2], x * v[1] - y * v[0]];
    let c2 = [y * c1[2] - z * c1[1], z * c1[0] - x * c1[2], x * c1[1] - y * c1[0]];
    [v[0] + 2.0 * (w * c1[0] + c2[0]), v[1] + 2.0 * (w * c1[1] + c2[1]), v[2] + 2.0 * (w * c1[2] + c2[2])]
}

/// Sixteen hex digits the Quarry minted, or nothing.
fn is_design(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// The recipes a species carries INSIDE it — leaves, the wither curve, the
/// crops — each stripped of what its own blank says. The top-level strip
/// cannot see into them: on 2026-09-27 `LeafRecipe` gained `color_autumn`
/// with a default, and every design with leaves forked, because `leaves` was
/// compared as one whole object against `null`. Same lesson, one level down.
///
/// `bark` is not here: `TextureRecipe` (thread-manifest) has no `Default`,
/// so a field added to it is a field every old recipe must spell out, and an
/// old recipe fails to parse rather than silently changing its name.
fn strip_blank(mut v: serde_json::Value) -> serde_json::Value {
    let blanks: [(&str, serde_json::Value); 4] = [
        ("leaves", serde_json::to_value(grove::LeafRecipe::default()).unwrap_or_default()),
        ("wither", serde_json::to_value(grove::WitherRecipe::default()).unwrap_or_default()),
        ("blooms", serde_json::to_value(grove::CropRecipe::default()).unwrap_or_default()),
        ("fruit", serde_json::to_value(grove::CropRecipe::default()).unwrap_or_default()),
    ];
    if let Some(obj) = v.as_object_mut() {
        for (key, blank) in blanks {
            let blank = canonical_numbers(blank);
            if let (Some(serde_json::Value::Object(inner)), Some(b)) = (obj.get_mut(key), blank.as_object()) {
                inner.retain(|k, x| b.get(k) != Some(x));
            }
        }
    }
    v
}

/// Every number in a value, rounded to six decimals — recursively, because
/// the noise lives in nested arrays too. The oak's bark colour `0.11` came
/// out of one build as `0.10999999940395357` and out of the next as
/// `…355`, and the design changed its name over nothing.
fn canonical_numbers(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Number(_) => canonical_number(&v),
        serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(canonical_numbers).collect()),
        serde_json::Value::Object(o) => {
            serde_json::Value::Object(o.into_iter().map(|(k, x)| (k, canonical_numbers(x))).collect())
        }
        other => other,
    }
}

/// Float noise must not fork a design: 5.200000001 and 5.2 are one thing.
/// Six decimals — an f32 carries about seven significant digits, so its
/// noise sits below this and a recipe's real values (metres, degrees,
/// colour channels) sit well above it.
fn canonical_number(v: &serde_json::Value) -> serde_json::Value {
    // An integer is exact and stays one: `seed: 7` must not become `7.0`,
    // which is a different string and therefore a different design.
    if v.is_i64() || v.is_u64() {
        return v.clone();
    }
    match v.as_f64() {
        Some(f) => serde_json::json!((f * 1_000_000.0).round() / 1_000_000.0),
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
            glb: None,
            source: String::new(),
            rest: false,
            hang: None,
        }
    }

    /// Carved things get coarser levels from the same recipe, each with
    /// fewer triangles and the same footprint.
    #[test]
    fn a_carved_thing_has_lods() {
        let data = tmp("lods");
        let mut sub = column("Sphere", 1.0);
        sub.export = "sphere".into();
        sub.args = vec![serde_json::json!(0.5)];
        let e = derive(&sub, &data).expect("derives");
        assert_eq!(e.artifact.lods.len(), 2, "{:?}", e.artifact.lods);
        let sizes: Vec<u64> = std::iter::once(e.artifact.url.as_str())
            .chain(e.artifact.lods.iter().map(String::as_str))
            .map(|u| std::fs::metadata(data.join(u.rsplit('/').next().unwrap())).unwrap().len())
            .collect();
        assert!(sizes[0] > sizes[1] && sizes[1] > sizes[2], "each level is smaller than the last: {sizes:?}");
        assert!(sizes[2] * 4 < sizes[0], "LOD2 is a fraction of LOD0: {sizes:?}");
        let _ = std::fs::remove_dir_all(&data);
    }

    /// `rest` stands a centred shape on the ground — and is part of the
    /// recipe, so it is a different design, and its absence changes nothing.
    #[test]
    fn rest_grounds_a_centred_shape() {
        let data = tmp("rest");
        let mut sub = column("Block", 1.0);
        sub.export = "cube".into();
        sub.args = vec![serde_json::json!(1.0); 3];
        let centred = derive(&sub, &data).expect("derives");
        assert_eq!(centred.facts.origin, "center");
        let mut rested = sub.clone();
        rested.rest = true;
        let grounded = derive(&rested, &data).expect("derives");
        assert_eq!(grounded.facts.origin, "base", "{:?}", grounded.facts.size);
        assert_ne!(grounded.design, centred.design, "resting is a different recipe");
        assert!((grounded.facts.size[1] - centred.facts.size[1]).abs() < 1e-4, "same size");
        let _ = std::fs::remove_dir_all(&data);
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

    /// The nine carved starters have had these ids on the live store since
    /// the shelf was first seeded. A change to how numbers are canonicalised
    /// that moves ANY of them is a change that orphans nine designs.
    #[test]
    fn the_carved_starters_keep_their_ids() {
        for (export, args, material, live) in crate::starters_pinned() {
            let mut sub = column("pinned", 1.0);
            sub.export = export.to_string();
            sub.args = args.iter().map(|a| serde_json::json!(a)).collect();
            sub.material = material.to_string();
            assert_eq!(design_id(&sub), live, "{export}{args:?} moved off its live id");
        }
    }

    /// A nested recipe field the grower gains tomorrow, left at its default,
    /// must not rename a design. Spelled with a default the grower already
    /// has, because a test cannot add a field to a struct — the property is
    /// the same.
    #[test]
    fn a_defaulted_nested_field_does_not_fork_a_design() {
        let blank_leaf = serde_json::to_value(grove::LeafRecipe::default()).unwrap();
        let mut sub = column("Oak", 1.0);
        sub.package = "grove".into();
        sub.export = String::new();
        sub.args = Vec::new();
        sub.material = String::new();
        let mut terse = sub.clone();
        terse.recipe = Some(serde_json::json!({ "name": "oak", "leaves": { "per_tip": 9 } }));
        let mut spelled = sub.clone();
        spelled.recipe = Some(serde_json::json!({ "name": "oak", "leaves": {
            "per_tip": 9,
            "color_autumn": blank_leaf["color_autumn"],
            "droop": blank_leaf["droop"],
        }}));
        assert_eq!(design_id(&terse), design_id(&spelled), "a nested default reached the hash");

        // and the default wither curve, which is not an Option, weighs nothing
        let canon = canonical_recipe(&terse).unwrap();
        assert!(canon.get("wither").is_none(), "an all-default nested recipe is dropped: {canon}");
        assert_eq!(canon["leaves"], serde_json::json!({ "per_tip": 9 }));
    }

    /// f32 noise in a nested array is the same noise as at the top.
    #[test]
    fn nested_float_noise_does_not_fork_a_design() {
        let mut sub = column("Oak", 1.0);
        sub.package = "grove".into();
        sub.export = String::new();
        sub.args = Vec::new();
        sub.material = String::new();
        let mut a = sub.clone();
        a.recipe = Some(serde_json::json!({ "name": "oak", "color": [0.11, 0.2, 0.3, 1.0] }));
        let mut b = sub.clone();
        b.recipe = Some(serde_json::json!({ "name": "oak", "color": [0.10999999940395357_f64, 0.2, 0.3, 1.0] }));
        assert_eq!(design_id(&a), design_id(&b));
    }

    /// If Grove gains another nested recipe (`roots: Option<RootRecipe>`, say),
    /// its defaults will leak into the hash until `strip_blank` names it.
    /// This is the tripwire: it fails the day that happens, and says so.
    #[test]
    fn every_nested_recipe_is_stripped() {
        let blank = grove::Planting::default().to_value();
        let nested: Vec<&str> = blank
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, v)| v.is_null() || v.is_object())
            .map(|(k, _)| k.as_str())
            .collect();
        let known = ["bark", "blooms", "fruit", "leaves", "wither"];
        for k in &nested {
            assert!(known.contains(k), "Grove gained a nested recipe `{k}` — add it to strip_blank (or, for a type without Default, to this list with a note)");
        }
    }

    /// Republishing an entry's own stored recipe must land on the same
    /// design: the canonical form is a fixed point, or the shelf fills with
    /// a new copy of every tree each time someone presses Republish.
    #[test]
    fn the_canonical_form_is_a_fixed_point() {
        let mut sub = column("Oak", 1.0);
        sub.package = "grove".into();
        sub.export = String::new();
        sub.args = Vec::new();
        sub.material = String::new();
        sub.recipe = Some(serde_json::json!({
            "name": "oak", "seed": 5, "height": 2.2000000476837158, "levels": 4,
            "angle": [38.0, 36.0, 34.0, 32.0], "forks": [3, 2, 2, 2],
            "leaves": { "per_tip": 9, "color": [0.14000000059604645, 0.3199999928474426, 0.1, 1.0] },
            "fruit": { "per_tip": 2 }, "season": 0.35
        }));
        let once = canonical_recipe(&sub).unwrap();
        let first = design_id(&sub);
        let mut again = sub.clone();
        again.recipe = Some(once.clone());
        assert_eq!(canonical_recipe(&again).unwrap(), once, "canonical(canonical(x)) != canonical(x)");
        assert_eq!(design_id(&again), first);
        assert_eq!(once["seed"], serde_json::json!(5), "an integer stayed an integer");
        assert_eq!(once["height"], serde_json::json!(2.2), "f32 noise was rounded away");
        assert!(once.get("season").is_none(), "the default season weighs nothing");
    }

    /// A lantern at every fruit socket: the placements are facts, the
    /// artifact stays the bare tree, and the hang is part of the design.
    #[test]
    fn a_tree_hangs_a_design_from_the_shelf() {
        let data = tmp("hang");
        // the lantern: a small carved orb, published first
        let mut orb = column("Orb", 1.0);
        orb.export = "sphere".into();
        orb.args = vec![serde_json::json!(0.12)];
        orb.material = "brass".into();
        let orb = derive(&orb, &data).expect("the orb shelves");

        let mut tree = column("Lantern tree", 1.0);
        tree.package = "grove".into();
        tree.export = String::new();
        tree.args = Vec::new();
        tree.material = String::new();
        tree.recipe = Some(serde_json::json!({
            "name": "lantern", "seed": 7, "height": 1.9, "levels": 3, "forks": [3, 2, 2],
            "fruit": { "per_tip": 1, "season": [0.3, 0.9] }, "season": 0.55
        }));
        let bare = derive(&tree, &data).expect("the bare tree grows");
        let fruit_sockets = bare.facts.sockets.iter().filter(|s| s.kind == "fruit").count();
        assert!(fruit_sockets > 0, "the test tree must fruit in season: {:?}", bare.facts.life);

        let mut lit = tree.clone();
        lit.hang = Some(Hang {
            design: orb.design.clone(),
            rule: grove::HangRecipe { kind: grove::SocketKind::Fruit, count: 0, drop: 0.1, ..Default::default() },
        });
        let lit = derive(&lit, &data).expect("the lit tree grows");
        assert_ne!(lit.design, bare.design, "a tree with lanterns is a different design");
        let hung = lit.facts.hung.as_ref().expect("placements are facts");
        assert_eq!(hung.count, fruit_sockets, "one at every fruit socket");
        assert_eq!(hung.placements.len(), fruit_sockets);
        assert_eq!(hung.design, orb.design);
        // the artifact is the bare plant: same triangle count as without lanterns
        assert_eq!(lit.artifact.tris, bare.artifact.tris, "the stored glb must not bake the instances");
        // every lantern hangs BELOW its socket
        for p in &hung.placements {
            let s = lit.facts.sockets.iter().find(|s| s.name == p.socket).expect("placement names a socket");
            assert!(p.translation[1] < s.at[1], "{} hangs above its tip", p.socket);
        }
        // capped by count
        let mut two = tree.clone();
        two.hang = Some(Hang { design: orb.design.clone(), rule: grove::HangRecipe { kind: grove::SocketKind::Fruit, count: 2, ..Default::default() } });
        assert_eq!(derive(&two, &data).unwrap().facts.hung.unwrap().count, 2);
        // republishing the entry's own recipe lands on the same design
        let mut again = tree.clone();
        again.hang = lit.recipe.hang.clone();
        assert_eq!(derive(&again, &data).unwrap().design, lit.design);
        // and a design that is not on the shelf is refused by name
        let mut ghost = tree.clone();
        ghost.hang = Some(Hang { design: "0000000000000000".into(), rule: Default::default() });
        assert!(derive(&ghost, &data).unwrap_err().contains("nothing on the shelf"));
        let _ = std::fs::remove_dir_all(&data);
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
        let e = derive_scratch(&column("Probe", 3.0), &scratch, &scratch).expect("derives");
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
