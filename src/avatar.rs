//! The Avatar door: a **manifested GLB**, checked against the Portable Item
//! Convention and measured.
//!
//! Chisel carves and Grove grows; an avatar part is neither — it is made in
//! Unity (or anywhere) and arrives as the convention's own unit of exchange,
//! one GLB that *is* the item: mesh, materials, textures, and its meaning at
//! `asset.extras.pixygonItem`. The Quarry does not derive it. It does the
//! next most useful thing: it reads the file the way every consumer will,
//! holds it to the rules the convention wrote down, and measures it. The
//! bytes are stored untouched — "never mutate meaning" — and the design id
//! is their hash, so the same file is the same design whoever sends it.
//!
//! Every rule below is a line of `CONVENTION.md` made executable. A check
//! that fails is reported, not hidden; the ones that make the file not an
//! item at all refuse it.

use avatar::manifest::{AttachType, ItemKind, ItemManifest};
use avatar::AvatarSlot;
use chisel::model::{Built, BuiltPart};
use chisel::MeshData;
use serde::{Deserialize, Serialize};

/// One rule, and how the file did against it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub rule: String,
    pub ok: bool,
    pub note: String,
}

/// What the Quarry learned from the file.
pub struct Checked {
    pub built: Built,
    pub manifest: ItemManifest,
    pub checks: Vec<Check>,
    /// Joint names of the first skin, in joint order — what a rebind by
    /// name has to match.
    pub bones: Vec<String>,
    pub skins: usize,
    pub textures: usize,
    pub materials: Vec<String>,
    /// `skinned` (a skin is present) · `bone` (a rigid prop) — measured off
    /// the file, not read from a form.
    pub attach: String,
}

/// The shortest honest summary of a bounding box, for a check's note.
fn dims(min: [f32; 3], max: [f32; 3]) -> String {
    format!("{:.2} × {:.2} × {:.2} m", max[0] - min[0], max[1] - min[1], max[2] - min[2])
}

pub fn check(bytes: &[u8]) -> Result<Checked, String> {
    let mut checks: Vec<Check> = Vec::new();
    let mut rule = |rule: &str, ok: bool, note: String| {
        checks.push(Check { rule: rule.into(), ok, note });
        ok
    };

    // ── the container ───────────────────────────────────────────────────
    let (doc, buffers, images) = gltf::import_slice(bytes).map_err(|e| format!("not a GLB the loader accepts: {e}"))?;
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return Err("not a GLB: the file does not start with `glTF`".into());
    }
    rule("glb", true, "binary glTF 2.0".into());

    // ── the meaning ─────────────────────────────────────────────────────
    let manifest = match avatar::manifest::manifest_from_glb(bytes) {
        Some(m) => m,
        None => {
            return Err("no manifest: the convention puts the item's meaning at `asset.extras.pixygonItem` (schema 1) — without it this is a mesh, not an item".into());
        }
    };
    rule("manifest", manifest.schema == 1, format!("pixygonItem schema {}", manifest.schema));
    if manifest.title.trim().is_empty() {
        rule("title", false, "the manifest carries no title".into());
    }
    let slot_known = match manifest.kind {
        ItemKind::Weapon => manifest.slot == "hand" || AvatarSlot::from_name(&manifest.slot).is_some(),
        ItemKind::Body => manifest.slot == "Body",
        _ => AvatarSlot::from_name(&manifest.slot).is_some(),
    };
    rule(
        "slot",
        slot_known,
        if slot_known {
            format!("{:?} in `{}`", manifest.kind, manifest.slot).to_lowercase()
        } else {
            format!("`{}` is not an AvatarSlot name (weapons say `hand`, bodies say `Body`)", manifest.slot)
        },
    );
    rule("id", manifest.id != 0, if manifest.id != 0 { format!("partId {}", manifest.id) } else { "partId is 0 — nothing can key this part".into() });

    // ── the file is whole ───────────────────────────────────────────────
    let external: Vec<String> = doc
        .images()
        .filter_map(|i| match i.source() {
            gltf::image::Source::Uri { uri, .. } => Some(uri.to_string()),
            _ => None,
        })
        .chain(doc.buffers().filter_map(|b| match b.source() {
            gltf::buffer::Source::Uri(uri) => Some(uri.to_string()),
            _ => None,
        }))
        .filter(|u| !u.starts_with("data:"))
        .collect();
    rule(
        "embedded",
        external.is_empty(),
        if external.is_empty() {
            format!("{} textures, all inside the file", images.len())
        } else {
            format!("reaches outside the file: {}", external.join(", "))
        },
    );

    let spec_gloss = doc.extensions_used().any(|e| e == "KHR_materials_pbrSpecularGlossiness");
    let unlit = doc.extensions_used().any(|e| e == "KHR_materials_unlit");
    rule(
        "pbr",
        !spec_gloss,
        if spec_gloss {
            "specular-glossiness — the convention is metallic-roughness".into()
        } else if unlit {
            "metallic-roughness (some materials unlit)".into()
        } else {
            "metallic-roughness".into()
        },
    );

    // ── the geometry, measured ──────────────────────────────────────────
    let mut parts: Vec<BuiltPart> = Vec::new();
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let scene_nodes: Vec<gltf::Node> = match doc.default_scene().or_else(|| doc.scenes().next()) {
        Some(s) => s.nodes().collect(),
        None => doc.nodes().filter(|n| n.children().len() == 0 && n.mesh().is_some()).collect(),
    };
    let mut stack: Vec<(gltf::Node, [[f32; 4]; 4])> = scene_nodes.into_iter().map(|n| (n, IDENTITY)).collect();
    while let Some((node, parent)) = stack.pop() {
        let world = mul(parent, node.transform().matrix());
        // A skinned mesh's vertices live in the skeleton's space; the glTF
        // spec says its own node transform is ignored. Rigid ones move.
        let skinned = node.skin().is_some();
        if let Some(mesh) = node.mesh() {
            for prim in mesh.primitives() {
                let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d.0[..]));
                let Some(pos) = reader.read_positions() else { continue };
                let positions: Vec<[f32; 3]> = pos.map(|p| if skinned { p } else { transform(world, p) }).collect();
                for p in &positions {
                    for i in 0..3 {
                        min[i] = min[i].min(p[i]);
                        max[i] = max[i].max(p[i]);
                    }
                }
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(ix) => ix.into_u32().collect(),
                    None => (0..positions.len() as u32).collect(),
                };
                let normals: Vec<[f32; 3]> = match reader.read_normals() {
                    Some(n) => n.map(|n| if skinned { n } else { rotate(world, n) }).collect(),
                    None => flat_normals(&positions, &indices),
                };
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|t| t.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
                let colors: Vec<[f32; 4]> = reader
                    .read_colors(0)
                    .map(|c| c.into_rgba_f32().collect())
                    .unwrap_or_else(|| vec![[1.0; 4]; positions.len()]);
                let mat = prim.material();
                let name = mat
                    .name()
                    .map(str::to_string)
                    .or_else(|| mesh.name().map(str::to_string))
                    .unwrap_or_else(|| format!("part {}", parts.len()));
                let mut color = mat.pbr_metallic_roughness().base_color_factor();
                // A grey-white factor with a texture reads as bare plaster on
                // the turntable; the real colour is in the texture we do not
                // sample here. Tint it toward a neutral so the sheet reads.
                if mat.pbr_metallic_roughness().base_color_texture().is_some() && color[..3].iter().all(|c| *c > 0.95) {
                    color = [0.72, 0.7, 0.66, 1.0];
                }
                let n = positions.len();
                parts.push(BuiltPart {
                    name,
                    mesh: MeshData {
                        positions,
                        normals,
                        uvs,
                        tangents: vec![[1.0, 0.0, 0.0, 1.0]; n],
                        colors,
                        uv2: vec![[0.0, 0.0]; n],
                        indices,
                    },
                    baked: None,
                    color,
                    emissive: mat.emissive_factor().iter().cloned().fold(0.0, f32::max),
                    double_sided: mat.double_sided(),
                });
            }
        }
        for child in node.children() {
            stack.push((child, world));
        }
    }
    if parts.is_empty() || parts.iter().all(|p| p.mesh.indices.is_empty()) {
        return Err("no geometry: the GLB has no mesh in its scene".into());
    }
    rule("triangles", true, format!("{} triangles in {} parts", parts.iter().map(|p| p.mesh.indices.len() / 3).sum::<usize>(), parts.len()));

    let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let longest = size.iter().cloned().fold(0.0, f32::max);
    // Metres. A body is 0.5–3 m tall; a garment or a prop is 2 cm–4 m on its
    // longest side. 180 for a human is centimetres that nobody converted.
    let metres_ok = match manifest.kind {
        ItemKind::Body => (0.5..=3.0).contains(&size[1]),
        _ => (0.02..=4.0).contains(&longest),
    };
    rule(
        "metres",
        metres_ok,
        if metres_ok {
            dims(min, max)
        } else {
            format!("{} — not a size a {:?} has in metres", dims(min, max), manifest.kind).to_lowercase()
        },
    );

    // Origin: feet on the ground for a body; the anchor near the mesh for the
    // rest (an item whose origin is metres away from it was exported in place).
    let origin_ok = match manifest.kind {
        ItemKind::Body => min[1].abs() <= 0.05,
        _ => (0..3).all(|i| min[i] <= size[i] * 0.5 + 0.02 && max[i] >= -(size[i] * 0.5) - 0.02),
    };
    rule(
        "origin",
        origin_ok,
        match manifest.kind {
            ItemKind::Body if origin_ok => "feet at y = 0".into(),
            ItemKind::Body => format!("feet at y = {:.2} — a body's origin is its feet", min[1]),
            _ if origin_ok => "anchor within the mesh".into(),
            _ => format!("origin is outside the mesh (min {:.2},{:.2},{:.2}) — export at the grip or worn anchor, not in place", min[0], min[1], min[2]),
        },
    );

    // ── the rig ─────────────────────────────────────────────────────────
    let skins = doc.skins().count();
    let bones: Vec<String> = doc
        .skins()
        .next()
        .map(|s| s.joints().enumerate().map(|(i, j)| j.name().map(str::to_string).unwrap_or_else(|| format!("joint {i}"))).collect())
        .unwrap_or_default();
    let unnamed = bones.iter().filter(|b| b.starts_with("joint ")).count();
    let wants_skin = matches!(manifest.kind, ItemKind::Body | ItemKind::Garment);
    let attach = if skins > 0 { "skinned" } else { "bone" };
    if wants_skin {
        rule(
            "skinned",
            skins > 0 && unnamed == 0,
            if skins == 0 {
                format!("a {:?} binds to the shared skeleton by bone name, and this file has no skin", manifest.kind).to_lowercase()
            } else if unnamed > 0 {
                format!("{unnamed} of {} joints have no name — nothing can rebind them", bones.len())
            } else {
                format!("{} named joints", bones.len())
            },
        );
    } else {
        rule("attach", true, if skins > 0 { format!("skinned, {} joints", bones.len()) } else { "rigid — snaps to a named bone".into() });
    }
    let _ = AttachType::Bone; // the vocabulary the doc uses; the file decides which

    let materials: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
    Ok(Checked {
        built: Built { name: manifest.title.clone(), parts },
        manifest,
        checks,
        bones,
        skins,
        textures: images.len(),
        materials,
        attach: attach.into(),
    })
}

// ── a little matrix algebra, column-major as glTF hands it over ──────────

const IDENTITY: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

fn mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (c, col) in out.iter_mut().enumerate() {
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

fn transform(m: [[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
        m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
        m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
    ]
}

fn rotate(m: [[f32; 4]; 4], n: [f32; 3]) -> [f32; 3] {
    let v = [
        m[0][0] * n[0] + m[1][0] * n[1] + m[2][0] * n[2],
        m[0][1] * n[0] + m[1][1] * n[1] + m[2][1] * n[2],
        m[0][2] * n[0] + m[1][2] * n[1] + m[2][2] * n[2],
    ];
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Per-vertex normals from the faces, for a file that shipped none.
fn flat_normals(pos: &[[f32; 3]], idx: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![[0.0f32; 3]; pos.len()];
    for t in idx.chunks_exact(3) {
        let (a, b, c) = (pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let f = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        for &i in t {
            for k in 0..3 {
                n[i as usize][k] += f[k];
            }
        }
    }
    for v in &mut n {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 1e-9 {
            *v = [v[0] / len, v[1] / len, v[2] / len];
        } else {
            *v = [0.0, 1.0, 0.0];
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A carved column, then the convention's manifest written into its JSON
    /// chunk — the GLB a Unity exporter would hand over, made here so the
    /// test owns every byte of it.
    fn glb(export: &str, args: &[f32]) -> Vec<u8> {
        let lib = chisel::weft_model::standard_library();
        let a: Vec<serde_json::Value> = args.iter().map(|x| serde_json::json!(x)).collect();
        let model = chisel::weft_model::eval_model_or_part(&lib, export, &a, Some("marble")).unwrap();
        let built = chisel::model::build(&model).unwrap();
        chisel::model::export_glb(&built).unwrap()
    }

    /// Rewrite a GLB's JSON chunk with the manifest at `asset.extras`,
    /// padding to four bytes as the spec demands and fixing both lengths.
    fn with_manifest(glb: &[u8], manifest: serde_json::Value) -> Vec<u8> {
        let u32_at = |o: usize| u32::from_le_bytes([glb[o], glb[o + 1], glb[o + 2], glb[o + 3]]);
        let json_len = u32_at(12) as usize;
        let mut json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        json["asset"]["extras"]["pixygonItem"] = manifest;
        let mut text = json.to_string().into_bytes();
        while text.len() % 4 != 0 {
            text.push(b' ');
        }
        let rest = &glb[20 + json_len..];
        let mut out = Vec::with_capacity(20 + text.len() + rest.len());
        out.extend_from_slice(&glb[0..8]);
        out.extend_from_slice(&((20 + text.len() + rest.len()) as u32).to_le_bytes());
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(&glb[16..20]);
        out.extend_from_slice(&text);
        out.extend_from_slice(rest);
        out
    }

    fn manifest(kind: &str, slot: &str) -> serde_json::Value {
        serde_json::json!({ "schema": 1, "id": 40001, "kind": kind, "slot": slot, "title": "Wayfarer's helm",
                            "codexSlug": "wayfarers-helm", "stats": [{ "id": 40001, "key": "Defense.Defense", "value": 6 }] })
    }

    fn rule<'a>(c: &'a Checked, name: &str) -> &'a Check {
        c.checks.iter().find(|x| x.rule == name).unwrap_or_else(|| panic!("no rule {name}: {:?}", c.checks))
    }

    #[test]
    fn a_manifested_prop_adheres() {
        let bytes = with_manifest(&glb("bowl", &[0.2, 0.03]), manifest("part", "Headgear"));
        let c = check(&bytes).expect("a manifested GLB is an item");
        assert_eq!(c.manifest.title, "Wayfarer's helm");
        assert_eq!(c.manifest.codex_slug.as_deref(), Some("wayfarers-helm"));
        let failed: Vec<&Check> = c.checks.iter().filter(|x| !x.ok).collect();
        assert!(failed.is_empty(), "failed: {failed:?}");
        assert_eq!(c.attach, "bone", "a rigid prop snaps to a bone");
        assert!(c.built.triangles() > 0);
        assert_eq!(c.textures, 3, "the marble set — base colour, normal, ORM — rides inside the file");
    }

    #[test]
    fn a_bare_mesh_is_not_an_item() {
        let err = check(&glb("bowl", &[0.2, 0.03])).err().expect("a bare mesh is refused");
        assert!(err.contains("pixygonItem"), "{err}");
    }

    #[test]
    fn junk_is_refused_by_name() {
        assert!(check(b"not a glb at all").is_err());
        assert!(check(&[]).is_err());
    }

    #[test]
    fn a_slot_the_avatar_does_not_have_is_caught() {
        let bytes = with_manifest(&glb("bowl", &[0.2, 0.03]), manifest("part", "Hat"));
        let c = check(&bytes).unwrap();
        assert!(!rule(&c, "slot").ok, "{:?}", rule(&c, "slot"));
        // and a weapon may say `hand`
        let bytes = with_manifest(&glb("cylinder", &[0.02, 1.0]), manifest("weapon", "hand"));
        assert!(rule(&check(&bytes).unwrap(), "slot").ok);
    }

    #[test]
    fn a_body_without_a_skin_cannot_be_dressed() {
        let bytes = with_manifest(&glb("capsule", &[0.25, 1.75]), manifest("body", "Body"));
        let c = check(&bytes).unwrap();
        let sk = rule(&c, "skinned");
        assert!(!sk.ok, "{sk:?}");
        assert!(sk.note.contains("no skin"), "{}", sk.note);
    }

    #[test]
    fn centimetres_are_caught_as_metres() {
        // a 175 "m" body: somebody exported in centimetres
        let bytes = with_manifest(&glb("capsule", &[25.0, 175.0]), manifest("body", "Body"));
        let c = check(&bytes).unwrap();
        assert!(!rule(&c, "metres").ok, "{:?}", rule(&c, "metres"));
        let bytes = with_manifest(&glb("capsule", &[0.25, 1.75]), manifest("body", "Body"));
        assert!(rule(&check(&bytes).unwrap(), "metres").ok);
    }

    #[test]
    fn the_facts_are_the_files_not_the_manifests() {
        let bytes = with_manifest(&glb("column", &[1.5, 0.2]), manifest("part", "Offhand"));
        let c = check(&bytes).unwrap();
        let (min, max) = c.built.bounds();
        assert!((max[1] - min[1] - 1.5).abs() < 0.02, "height measured off the mesh: {min:?} {max:?}");
        assert_eq!(c.materials, vec!["marble".to_string()]);
    }

    /// The bytes on the shelf are the bytes that came in. Nothing is
    /// re-exported, re-padded or "fixed": the maker's file is the file.
    #[test]
    fn the_stored_file_is_the_original() {
        let dir = std::env::temp_dir().join(format!("quarry-avatar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let bytes = with_manifest(&glb("bowl", &[0.2, 0.03]), manifest("part", "Headgear"));
        let sub = crate::entry::Submission {
            title: String::new(), description: String::new(), tags: vec![], kind: String::new(), style: String::new(),
            package: "avatar".into(), export: String::new(), args: vec![], recipe: None, material: String::new(),
            license: "CC0-1.0".into(), author: String::new(), origin: "imported".into(), sockets: vec![],
            codex: String::new(), concept: String::new(), glb: Some(bytes.clone()), source: String::new(),
        };
        let e = crate::entry::derive(&sub, &dir).expect("shelved");
        let stored = std::fs::read(dir.join(format!("{}.glb", e.design))).unwrap();
        assert_eq!(stored, bytes, "the shelf rewrote the file");
        assert_eq!(e.title, "Wayfarer's helm", "the manifest named it");
        assert_eq!(e.codex, "wayfarers-helm", "the manifest's codex slug is the concept to judge against");
        assert_eq!(e.kind, "part");
        assert!(e.tags.contains(&"headgear".to_string()) && e.tags.contains(&"avatar".to_string()));
        assert_eq!(e.supplier, "avatar");
        let item = e.facts.item.as_ref().unwrap();
        assert_eq!(item.failed, 0);
        assert_eq!(item.id, 40001);
        // and the same bytes are the same design, from anyone
        let mut again = sub.clone();
        again.title = "someone else's name for it".into();
        assert_eq!(crate::entry::derive(&again, &dir).unwrap().design, e.design);
        // re-checking from the shelf lands on the same design too
        let mut re = sub.clone();
        re.glb = None;
        re.source = e.design.clone();
        assert_eq!(crate::entry::derive(&re, &dir).unwrap().design, e.design);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
