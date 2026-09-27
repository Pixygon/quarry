//! The **Portable Item Convention** — Rust reference implementation.
//!
//! The unit of exchange across the whole Pixygon ecosystem (Unity games, the
//! web Avatar Studio, and the Thread browser) is **one GLB file that carries
//! its own meaning**: mesh, materials, textures, AND identity/stats/lore baked
//! into the file at `asset.extras.pixygonItem` (schema 1, append-only).
//!
//! Keep in sync with: `com.pixygon.avatar` `ItemManifest` (C#) and
//! `@pixygon/avatar/parts` (TypeScript) — this module mirrors those types
//! field-for-field. See the package's `CONVENTION.md` for the normative rules
//! (meters · Y-up · +Z forward · origin at grip/anchor/feet · PBR
//! metallic-roughness · textures embedded · skinned parts bind to the shared
//! skeleton **by bone name**).
//!
//! Everything keys by **partId** (`AvatarAsset.partId` — the stable IdObject
//! id): never by name, never by database `_id`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Item kind — mirrors the Unity `ItemManifest` kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Garment,
    Weapon,
    Body,
    Part,
    Consumable,
}

/// One stat modifier. `id` is the stable Pixygon.Stats catalog id (identity);
/// `key` is documentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStat {
    pub id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub value: f32,
}

/// Worn/held placement offsets (applied when equipping, not on load).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPlacement {
    #[serde(default)]
    pub worn_on_back: bool,
    /// metres `[x, y, z]`
    #[serde(default)]
    pub offset: Option<[f32; 3]>,
    /// degrees `[x, y, z]`
    #[serde(default)]
    pub euler: Option<[f32; 3]>,
}

/// GLB spatial declaration — meters, Y-up, +Z forward per the convention.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GlbMeta {
    #[serde(default)]
    pub units: Option<String>,
    #[serde(default)]
    pub up: Option<String>,
    #[serde(default)]
    pub forward: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
}

/// The item's meaning, baked into the GLB at `asset.extras.pixygonItem`.
/// Schema 1 — append-only; never mutate client-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemManifest {
    pub schema: u32,
    /// IdObject full id — same value as `AvatarAsset.partId`. THE shared key.
    pub id: u32,
    pub kind: ItemKind,
    /// [`crate::AvatarSlot`] name; `"hand"` for weapons; `"Body"` for bodies.
    pub slot: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub lore: Vec<String>,
    /// The Codex entry this item IS.
    #[serde(default)]
    pub codex_slug: Option<String>,
    #[serde(default)]
    pub stats: Vec<ItemStat>,
    #[serde(default)]
    pub placement: Option<ItemPlacement>,
    #[serde(default)]
    pub casting_implement: bool,
    #[serde(default)]
    pub cast_power: Option<f32>,
    #[serde(default)]
    pub glb: Option<GlbMeta>,
}

/// How a part attaches to the shared skeleton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachType {
    /// Skinned mesh — rebinds to the shared skeleton's bones **by name**.
    Skinned,
    /// Rigid prop — parents to [`AvatarAssetDoc::snap_bone`].
    Bone,
}

/// One bone rename in [`AvatarAssetFix::bone_map`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoneRename {
    pub from: String,
    pub to: String,
}

/// Per-asset load-time corrections stored on `AvatarAsset.fix` — fix a bad
/// export in data, not by re-exporting. Applied on EVERY load, before
/// binding, in order: scale → rotation → position, then boneMap renames.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarAssetFix {
    #[serde(default)]
    pub scale: Option<f32>,
    /// euler DEGREES `[x, y, z]`
    #[serde(default)]
    pub rotation: Option<[f32; 3]>,
    /// metres `[x, y, z]`
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub bone_map: Vec<BoneRename>,
}

/// The server `AvatarAsset` document (PixygonAPI `/v1/avatar/assets`): one
/// hosted part GLB plus its catalog metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarAssetDoc {
    #[serde(rename = "_id", default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// `mesh` | `texture` | `animation` | `skeleton`.
    #[serde(rename = "type", default)]
    pub asset_type: String,
    /// `face` | `body` | `clothing` | `hair` | `accessory` | `other`.
    #[serde(default)]
    pub category: Option<String>,
    /// [`crate::AvatarSlot`] name this part fills.
    #[serde(default)]
    pub slot: Option<String>,
    /// The stable IdObject id — the shared key between web, Unity, and saved
    /// avatar specs. Key everything by this; never by `name` or `_id`.
    #[serde(default)]
    pub part_id: Option<u32>,
    /// The GLB's CDN URL — fetch it like any Thread asset.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub attach_type: Option<AttachType>,
    /// Bone name rigid parts snap to (attach type `bone`).
    #[serde(default)]
    pub snap_bone: Option<String>,
    #[serde(default)]
    pub fix: Option<AvatarAssetFix>,
    #[serde(default)]
    pub tint: Option<String>,
    #[serde(default)]
    pub owned_by_default: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub active: Option<bool>,
}

/// Index a catalog by partId. Assets without a partId are skipped (they need
/// the server-side backfill before they're wearable).
pub fn index_by_part_id(assets: &[AvatarAssetDoc]) -> HashMap<u32, &AvatarAssetDoc> {
    let mut map = HashMap::new();
    for a in assets {
        if let Some(pid) = a.part_id {
            map.insert(pid, a);
        }
    }
    map
}

/// Resolve an [`crate::AvatarSpec`] into asset docs through a partId index —
/// the same semantics as the web `resolveAvatarConfig`. Slots whose partId the
/// catalog doesn't know are omitted.
pub fn resolve_spec<'a>(
    spec: &crate::AvatarSpec,
    index: &HashMap<u32, &'a AvatarAssetDoc>,
) -> HashMap<crate::AvatarSlot, &'a AvatarAssetDoc> {
    let mut out = HashMap::new();
    for (slot, part_id) in spec.iter() {
        if let Some(asset) = index.get(&part_id) {
            out.insert(slot, *asset);
        }
    }
    out
}

// --- Reading the meaning out of a GLB ---

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"

/// Read the schema-1 `pixygonItem` manifest baked into a GLB's
/// `asset.extras.pixygonItem`. Returns `None` for non-GLB bytes or a GLB
/// without the manifest (fall back to the `.json` sidecar, then to the
/// catalog's `AvatarAsset` doc — the same order the web reader uses).
pub fn manifest_from_glb(bytes: &[u8]) -> Option<ItemManifest> {
    let u32_at = |o: usize| -> Option<u32> {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if u32_at(0)? != GLB_MAGIC || u32_at(4)? != 2 {
        return None;
    }
    // First chunk is required to be the JSON chunk (glTF 2.0 §4.4.3).
    let chunk_len = u32_at(12)? as usize;
    if u32_at(16)? != CHUNK_JSON {
        return None;
    }
    let json = bytes.get(20..20 + chunk_len)?;
    manifest_from_gltf_json(std::str::from_utf8(json).ok()?)
}

/// Read the manifest out of glTF JSON text (a `.gltf`, or a GLB's JSON chunk).
pub fn manifest_from_gltf_json(json: &str) -> Option<ItemManifest> {
    let doc: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = doc.get("asset")?.get("extras")?.get("pixygonItem")?;
    serde_json::from_value(item.clone()).ok()
}
