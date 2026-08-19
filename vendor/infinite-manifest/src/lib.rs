//! # World Manifest — the document format of the Thread
//!
//! The Thread is the open, spatial, present, time-aware successor to the web:
//! worlds are *pearls* strung along one Thread, linked by **portals** (veils you
//! step through). This crate defines the **World Manifest** — the "HTML" of the
//! Thread: an open, **renderer-agnostic** description of a place that *any*
//! browser (Infinite is the first) can resolve and render.
//!
//! Design principles (mirroring why the web won):
//! - **Narrow waist.** Every browser implements this format or nothing interops.
//! - **Renderer-agnostic.** Geometry is glTF + prefab refs + a standard material
//!   model — never a specific engine's internals.
//! - **Static-first, presence-optional.** A world is just files (host it
//!   anywhere). Presence (shared, multi-user) is an *upgrade* you opt into by
//!   naming a relay; without one, the world gracefully degrades to solo.
//! - **Meaning is native.** Anything can link to a canonical [`Codex`] entry.
//! - **Time is an axis.** A world declares a `year`; the same place is
//!   addressable across time.
//!
//! See `docs/spec/world-manifest-v0.1.md` for the normative specification.

pub mod arch;
pub mod lint;
pub mod model;
pub mod shape;
pub mod texture;
mod locator;
pub mod markup;
pub use locator::{well_known_url, Locator, SCHEME};

use serde::{Deserialize, Serialize};
/// The prefab/item id scheme, re-exported so consumers of the manifest don't need
/// a direct `thread-id` dependency.
pub use thread_id::StructuredId;

/// The format version tag carried in every manifest's `thread` field.
pub const THREAD_VERSION: &str = "thread/0.1";

/// A World Manifest — one place on the Thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldManifest {
    /// Format version, e.g. `"thread/0.1"`.
    pub thread: String,
    pub world: WorldMeta,
    #[serde(default)]
    pub environment: Environment,
    /// Where visitors arrive. The first is the default; others are `#place` anchors.
    #[serde(default)]
    pub spawns: Vec<Spawn>,
    /// External content referenced by prefabs/behaviors (glTF, textures, WASM).
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// Deduplicated renderable prefabs, each keyed by a [`StructuredId`].
    #[serde(default)]
    pub prefabs: Vec<Prefab>,
    /// Instances of prefabs placed in the world.
    #[serde(default)]
    pub placements: Vec<Placement>,
    /// Veils to other worlds — the hyperlinks of the Thread.
    #[serde(default)]
    pub portals: Vec<Portal>,
    /// Sandboxed behavior modules (WASM) the world runs.
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
    /// CSS-like style rules — attach properties to placements by type / class /
    /// id instead of inline on every one (the cascade). See [`StyleRule`].
    #[serde(default)]
    pub styles: Vec<StyleRule>,
    /// Presence policy. Absent → the world is solo (static-first).
    #[serde(default)]
    pub presence: Option<Presence>,
}

/// Identity + descriptive metadata for the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMeta {
    /// Stable id, unique within the host (the last path segment of its Locator).
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// Provider-agnostic author identity (DID-style), if declared.
    #[serde(default)]
    pub author: Option<Identity>,
    /// Codex slugs describing this world (canonical meaning).
    #[serde(default)]
    pub codex: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
}

/// A portable, provider-agnostic identity (the open form of the Passport).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Decentralized-identifier-style id (`did:pixygon:…`, etc.).
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// Environmental setup: time, sky, bounds, and the game mechanics this place
/// enforces.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    /// Timeline year of this world (the `@when` axis). `None` = the present.
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub sky: Option<Sky>,
    #[serde(default)]
    pub bounds: Option<Bounds>,
    /// Opt-in game mechanics this world enforces on visitors. Absent → none (the
    /// browser stays a browser; most places are not games).
    #[serde(default)]
    pub rules: WorldRules,
}

/// Opt-in game mechanics a world enforces. **Default all off** — the browser
/// ships these as capabilities; a place turns on only what it needs (like a page
/// opting into pointer lock). Interact/inspect are always available and are NOT
/// gated here — only the game-y mechanics are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRules {
    /// Survival meters (hunger/…) decay and can harm the visitor.
    #[serde(default)]
    pub survival: bool,
    /// Harvestable resource nodes can be gathered.
    #[serde(default)]
    pub gathering: bool,
    /// Offensive combat is allowed.
    #[serde(default)]
    pub combat: bool,
}

/// A simple gradient sky (zenith→horizon) with an optional sun direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sky {
    pub zenith: [f32; 3],
    pub horizon: [f32; 3],
    #[serde(default)]
    pub sun_dir: Option<[f32; 3]>,
}

/// Axis-aligned world bounds (advisory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// An arrival point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spawn {
    /// Anchor name (the `#place` in a Locator); the first spawn is the default.
    #[serde(default)]
    pub name: String,
    pub position: [f32; 3],
    #[serde(default)]
    pub yaw: f32,
}

/// External content referenced by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    /// Where to fetch it — a relative path, absolute URL, or `ipfs://…`.
    pub uri: String,
    pub kind: AssetKind,
}

/// What an [`Asset`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Gltf,
    Texture,
    Wasm,
    /// A Weft module (the Thread's native code — weft-v0.1).
    Weft,
    Audio,
    Other,
}

/// A unique renderable prefab (mesh + optional material), keyed by [`StructuredId`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prefab {
    pub id: StructuredId,
    pub mesh: MeshRef,
    #[serde(default)]
    pub material: Option<MaterialRef>,
}

/// How a prefab's geometry is sourced. Exactly one field should be set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeshRef {
    /// The id of a glTF [`Asset`].
    #[serde(default)]
    pub asset: Option<String>,
    /// A built-in primitive: `cube`, `sphere`, `cylinder`, `capsule`, `plane`, `quad`.
    #[serde(default)]
    pub builtin: Option<String>,
    /// A procedural shape recipe (see [`shape`]) — the browser meshes it at
    /// load time. Ships the *intent*, not the vertices; additive, so browsers
    /// that predate shapes simply skip the prefab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<shape::Shape>,
    /// Sampling resolution for `shape` meshing (grid cells per axis; default
    /// 40, clamped by the mesher).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<u32>,
}

/// Standard PBR material (renderer-agnostic). Texture fields are [`Asset`] ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialRef {
    #[serde(default)]
    pub base_color_texture: Option<String>,
    /// Packed occlusion-roughness-metallic (glTF convention).
    #[serde(default)]
    pub orm_texture: Option<String>,
    #[serde(default)]
    pub normal_texture: Option<String>,
    #[serde(default = "white")]
    pub base_color: [f32; 4],
    #[serde(default)]
    pub metallic: f32,
    #[serde(default = "one")]
    pub roughness: f32,
    /// Glow strength: `> 0` renders unlit, glowing `base_color` (blooms in
    /// browsers with HDR). `0` (default) = lit normally.
    #[serde(default)]
    pub emissive: f32,
    /// A procedural material recipe (see [`texture`]) — the browser bakes the
    /// full PBR set (albedo / normal / occlusion-roughness-metallic) locally.
    /// Additive; when set, the baked maps replace the `*_texture` asset slots
    /// and the scalar factors still multiply (tinting works as usual).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<texture::TextureRecipe>,
}

impl Default for MaterialRef {
    /// Matches the serde defaults (white base colour, full roughness) — a
    /// derived Default would zero `base_color` and silently blacken any
    /// programmatically-built textured material.
    fn default() -> Self {
        MaterialRef {
            base_color_texture: None,
            orm_texture: None,
            normal_texture: None,
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 1.0,
            emissive: 0.0,
            texture: None,
        }
    }
}

/// One placed instance of a prefab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub prefab: StructuredId,
    #[serde(default)]
    pub name: String,
    /// Semantic tag for `styles` **type** selectors ("tree", "door", "sign") —
    /// the HTML-element analog. JSON key is `type`.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// CSS-like classes for `styles` **class** selectors (`.harvestable`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub class: Vec<String>,
    pub position: [f32; 3],
    #[serde(default = "unit_quat")]
    pub rotation: [f32; 4],
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
    /// Canonical Codex slug for this object — "inspect" surfaces its lore.
    #[serde(default)]
    pub codex: Option<String>,
    /// Id of a [`Behavior`] bound to this placement.
    #[serde(default)]
    pub behavior: Option<String>,
    /// A **codeless** interaction — the simple path (no WASM). Name the verb and
    /// list what happens. For anything richer, use [`behavior`](Self::behavior).
    #[serde(default)]
    pub interaction: Option<Interaction>,
    /// Readable text rendered ONTO this placement's surface by the browser —
    /// plaques, signs, reading boards. Text as content, not as texture: the
    /// manifest carries the words; every browser typesets them itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextPanel>,
    /// Data-driven idle motion, executed by the **browser's** native loop —
    /// the Thread's idiom for living worlds (the web's idiom is app code
    /// updating transforms every frame). `None` = static.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animate: Option<Animate>,
    /// Whether this placement blocks movement. Default `true`; set `false`
    /// for decor a traveler should walk (and jump) through — statues,
    /// figures, conjured motes. A curved decorative collider is how you get
    /// bounced into the sky.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solid: Option<bool>,
    /// This placement **emits light** — a point light at its position. Lamps,
    /// torches, and stage lights are content, not renderer configuration; the
    /// browser's lighting pass honors them. `None` = not a light source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<LightEmitter>,
    /// Arbitrary per-placement data (e.g. a product id + price for commerce).
    #[serde(default)]
    pub data: serde_json::Value,
    /// Child placements, positioned **relative to this one** (the tree / DOM
    /// model). Their transforms compose with this placement's, so you build
    /// scenes by *containment* — a table with a lamp on it — instead of absolute
    /// coordinates. Defaults to none (a flat world is just a forest of depth 1).
    #[serde(default)]
    pub children: Vec<Placement>,
}

impl Placement {
    /// This placement and all its descendants, depth-first (parent before
    /// children). Transform composition is the renderer's job; this is the flat
    /// list for reference checks and iteration.
    pub fn iter_tree(&self) -> impl Iterator<Item = &Placement> {
        let mut stack = vec![self];
        std::iter::from_fn(move || {
            let p = stack.pop()?;
            stack.extend(p.children.iter().rev());
            Some(p)
        })
    }
}

/// Readable text a browser typesets onto a placement's surface (a `quad`
/// prefab is the natural carrier). The browser owns fonts and layout — the
/// world ships only the words, exactly like the old web shipping text and
/// letting the browser render it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextPanel {
    pub content: String,
    /// Approximate line height as a fraction of panel height (default 0.09 —
    /// about ten lines to a panel).
    #[serde(default = "default_text_size")]
    pub size: f32,
    /// Foreground RGB (default near-black ink).
    #[serde(default = "default_ink")]
    pub color: [f32; 3],
    /// Background RGB (default warm parchment).
    #[serde(default = "default_paper")]
    pub background: [f32; 3],
    /// **Hyperlinks inside the text** — the Thread's `<a>` tag. Each link
    /// names a phrase of `content` and the Locator it leads to; browsers
    /// render the phrase as a link (colored, underlined) and interacting with
    /// the panel follows it — text you can walk through, exactly like the old
    /// web's anchor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<TextLink>,
}

/// One hyperlink in a [`TextPanel`]: `text` is the phrase inside the panel's
/// content it anchors to (first occurrence); `to` is any Locator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLink {
    pub text: String,
    pub to: String,
}

/// A point light emitted from a placement's position. Declarative, like
/// everything else in a manifest: worlds say *what glows*; the browser's
/// deferred lighting pass decides how the glow falls on surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightEmitter {
    /// Light RGB (default warm lamplight).
    #[serde(default = "default_light_color")]
    pub color: [f32; 3],
    /// Brightness multiplier (default 1.0).
    #[serde(default = "one_f32")]
    pub intensity: f32,
    /// Reach in metres — beyond this the light is fully faded (default 10).
    #[serde(default = "default_light_range")]
    pub range: f32,
}

fn default_light_color() -> [f32; 3] {
    [1.0, 0.85, 0.6]
}
fn default_light_range() -> f32 {
    10.0
}

fn default_text_size() -> f32 {
    0.09
}
fn default_ink() -> [f32; 3] {
    [0.13, 0.11, 0.10]
}
fn default_paper() -> [f32; 3] {
    [0.93, 0.89, 0.80]
}

/// Declarative idle motion for a placement. Kinds (v0.1): `"spin"` (rotate
/// about the object's own Y axis), `"bob"` (rise and fall around the base
/// position), and `"path"` (travel a looping polyline of `points`, offsets
/// from the placement's position — a patrol, a tour, a journey retraced).
/// `speed` scales time (metres/second for `path`); `amp` is metres of travel
/// for `bob`. Browsers phase-shift instances so fields of movers don't march
/// in lockstep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animate {
    pub kind: String,
    #[serde(default = "one_f32")]
    pub speed: f32,
    #[serde(default = "quarter_f32")]
    pub amp: f32,
    /// Waypoints for `"path"`, as offsets from the placement position. The
    /// loop closes from the last point back to the first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<[f32; 3]>,
}

fn one_f32() -> f32 {
    1.0
}
fn quarter_f32() -> f32 {
    0.25
}

/// A declarative, codeless interaction a creator attaches to a placed object.
///
/// The whole point is that it's trivial to author: no WASM, no scripting — you
/// name the verb the visitor sees ("Chop", "Mine", "Pick", "Talk") and list what
/// happens when it completes. The interact key (E) runs it.
///
/// ```json
/// "interaction": {
///   "label": "Chop",
///   "hits": 3,
///   "effects": [
///     { "give_item": { "item": "20100001", "count": 3 } },
///     "despawn",
///     { "message": "You chopped the old oak." }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// Creator-defined verb shown in the "[E] …" prompt (e.g. "Chop", "Mine").
    pub label: String,
    /// How many interactions to complete it — a tree takes a few chops. Default 1.
    #[serde(default = "one_u32")]
    pub hits: u32,
    /// What happens when it completes, applied top to bottom.
    #[serde(default)]
    pub effects: Vec<InteractionEffect>,
}

/// One outcome of a completed [`Interaction`]. Deliberately small + declarative;
/// serialized snake_case (unit variants as bare strings) so manifests read clean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionEffect {
    /// Put an item in the visitor's inventory.
    GiveItem {
        item: StructuredId,
        #[serde(default = "one_u32")]
        count: u32,
    },
    /// Remove the interacted object from the world.
    Despawn,
    /// Show the visitor a transient message.
    Message(String),
    /// Play a named visual/audio effect at the object (an [`Asset`] id or built-in).
    Effect(String),
    /// Veilwalk to a Locator — **any object can be a doorway**. This is how
    /// text becomes teleportable: a link-stone under a paragraph carries the
    /// linked article's address.
    Navigate(String),
}

/// A CSS-like style rule: a **selector** + the properties it attaches to matching
/// placements. Lets a world define behaviour by `type` / `.class` / `#id` once,
/// instead of inline on every placement — restyle everything from one rule.
///
/// ```json
/// "styles": [
///   { "select": ".harvestable", "interaction": { "label": "Chop", "effects": ["despawn"] } }
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleRule {
    /// A simple selector: `"tree"` (type), `".harvestable"` (class), `"#hero"` (id).
    pub select: String,
    /// The declarative interaction this rule attaches. (More properties — mesh,
    /// material — cascade here as the layer grows.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<Interaction>,
}

/// A parsed single selector. Specificity mirrors CSS: id > class > type.
enum Selector {
    Type(String),
    Class(String),
    Id(String),
}

impl Selector {
    fn parse(s: &str) -> Option<Selector> {
        let s = s.trim();
        if let Some(id) = s.strip_prefix('#') {
            return (!id.is_empty()).then(|| Selector::Id(id.to_string()));
        }
        if let Some(c) = s.strip_prefix('.') {
            return (!c.is_empty()).then(|| Selector::Class(c.to_string()));
        }
        (!s.is_empty()).then(|| Selector::Type(s.to_string()))
    }
    fn specificity(&self) -> u8 {
        match self {
            Selector::Id(_) => 2,
            Selector::Class(_) => 1,
            Selector::Type(_) => 0,
        }
    }
    fn matches(&self, pl: &Placement) -> bool {
        match self {
            Selector::Type(t) => pl.kind.as_deref() == Some(t.as_str()),
            Selector::Class(c) => pl.class.iter().any(|x| x == c),
            Selector::Id(i) => &pl.name == i,
        }
    }
}

/// A veil to another world — the hyperlink of the Thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub id: String,
    pub position: [f32; 3],
    #[serde(default = "unit_quat")]
    pub rotation: [f32; 4],
    #[serde(default = "unit_scale")]
    pub scale: [f32; 3],
    /// Destination [`Locator`] (`thread://…`), possibly on another host.
    pub to: String,
    #[serde(default)]
    pub label: String,
    /// How much of the far side to show before stepping through.
    #[serde(default)]
    pub preview: PreviewPolicy,
}

/// Portal preview fidelity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewPolicy {
    /// No preview — an opaque doorway.
    None,
    /// A still image of the destination.
    #[default]
    Static,
    /// A live window into the destination (crowd, weather, time-of-day).
    Live,
}

/// A sandboxed WASM behavior module the world runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    pub id: String,
    /// The id of a WASM [`Asset`] (the polyglot floor — Behavior ABI v0.1).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub wasm: String,
    /// The id of a **Weft** [`Asset`] — the Thread's native code (weft-v0.1):
    /// a content-addressed, verified module. Exactly one of `wasm`/`weft`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weft: Option<String>,
    /// The id of a **Weft package** [`Asset`] (`.weftpack.json`, weft-pack-v0.1)
    /// — bind a published package's export as this behavior. Use with
    /// [`weft_export`](Self::weft_export); the browser fetches, verifies, and
    /// links locally. The markup form is `weft-use="<uri>#<export>"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weft_pack: Option<String>,
    /// The package export (petname) that is this behavior's entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weft_export: Option<String>,
    /// Event names this module handles (e.g. `interact`, `tick`).
    #[serde(default)]
    pub on: Vec<String>,
}

/// Presence policy — the (optional) upgrade from a solo world to a shared one.
///
/// Presence on the Thread is **federated**: a world names its *own* relay(s), so
/// there is no global chokepoint. Prefer [`relays`](Presence::relays) — an ordered
/// list of interchangeable, conformant relays (primary first, then fallbacks) — so
/// no single relay URL is a point of failure. The singular [`relay`](Presence::relay)
/// is retained for backward compatibility; [`relay_list`](Presence::relay_list)
/// returns the effective, de-duplicated order to try.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Presence {
    /// Presence tier: `"solo"`, `"p2p"`, or `"relay"` (see presence-topology-v0.1).
    /// Absent → inferred from whether any relay is named. Forward-looking; a browser
    /// that only implements the relay tier treats `"p2p"` as a hint it may ignore.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// A single presence relay (legacy / old-browser compatibility). Absent → solo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<String>,
    /// Interchangeable relays to try in order (primary first, then fallbacks). Any
    /// conformant relay works; if one is unreachable the browser tries the next.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relays: Vec<String>,
    /// Tier-1 P2P rendezvous URL (`wss://…/rtc/<key>`), meaningful with
    /// `mode: "p2p"`. A **different protocol** from a relay (mesh `Signal`
    /// introduction, never poses), so it is a distinct field — `relays` keeps its
    /// ordered-failover meaning unconditionally and MAY be present alongside as the
    /// Tier-2 fallback when the mesh can't form (the hybrid story).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendezvous: Option<String>,
    #[serde(default)]
    pub max_occupants: Option<u32>,
    #[serde(default)]
    pub voice: bool,
    /// The room lives only while its owner is present (a traveler's home with
    /// guests over). When such a room closes, browsers SHOULD walk guests back
    /// to their own home rather than leave them in a dead copy of the world.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub owner_required: bool,
}

impl Presence {
    /// The effective, ordered, de-duplicated list of relays to try: `relays` first,
    /// then the legacy singular `relay` if it isn't already present. Empty → solo.
    pub fn relay_list(&self) -> Vec<String> {
        let mut out = self.relays.clone();
        if let Some(r) = &self.relay {
            if !out.contains(r) {
                out.push(r.clone());
            }
        }
        out
    }
}

// --- serde defaults ---
fn white() -> [f32; 4] { [1.0, 1.0, 1.0, 1.0] }
fn one() -> f32 { 1.0 }
fn one_u32() -> u32 { 1 }
fn unit_quat() -> [f32; 4] { [0.0, 0.0, 0.0, 1.0] }
fn unit_scale() -> [f32; 3] { [1.0, 1.0, 1.0] }

/// An error parsing or validating a manifest.
#[derive(Debug)]
pub enum ManifestError {
    Json(serde_json::Error),
    /// One or more validation problems (each a human-readable message).
    Invalid(Vec<String>),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Json(e) => write!(f, "manifest JSON error: {e}"),
            ManifestError::Invalid(errs) => write!(f, "invalid manifest: {}", errs.join("; ")),
        }
    }
}
impl std::error::Error for ManifestError {}

impl WorldManifest {
    /// Parse and validate a manifest from JSON text.
    pub fn from_json(text: &str) -> Result<Self, ManifestError> {
        let m: WorldManifest = serde_json::from_str(text).map_err(ManifestError::Json)?;
        m.validate()?;
        Ok(m)
    }

    /// Parse and validate a manifest from **either** source form: JSON (the
    /// "DOM") or Thread markup (the "HTML" — see [`markup`]). Auto-detected by
    /// the first non-whitespace byte, so browsers, resolvers, and tools accept
    /// a served `.thread` file exactly as they accept a `world.json`.
    pub fn from_text(text: &str) -> Result<Self, ManifestError> {
        if text.trim_start().starts_with('{') {
            Self::from_json(text)
        } else {
            markup::compile(text).map_err(|e| ManifestError::Invalid(vec![e]))
        }
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// The default arrival point (first spawn, or the origin).
    pub fn default_spawn(&self) -> Spawn {
        self.spawns.first().cloned().unwrap_or(Spawn {
            name: String::new(),
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
        })
    }

    /// Resolve a `#place` anchor to a spawn or a portal position.
    pub fn anchor(&self, name: &str) -> Option<[f32; 3]> {
        if let Some(s) = self.spawns.iter().find(|s| s.name == name) {
            return Some(s.position);
        }
        self.portals.iter().find(|p| p.id == name).map(|p| p.position)
    }

    /// The **effective** interaction for a placement after the `styles` cascade:
    /// an inline `placement.interaction` always wins (like an inline `style=`);
    /// otherwise the highest-specificity matching rule applies, with later rules
    /// breaking ties (CSS source order). Returns `None` if nothing matches.
    pub fn computed_interaction(&self, pl: &Placement) -> Option<Interaction> {
        if pl.interaction.is_some() {
            return pl.interaction.clone();
        }
        let mut best: Option<(u8, &Interaction)> = None;
        for rule in &self.styles {
            let (Some(sel), Some(inter)) = (Selector::parse(&rule.select), &rule.interaction) else {
                continue;
            };
            if sel.matches(pl) && best.is_none_or(|(s, _)| sel.specificity() >= s) {
                best = Some((sel.specificity(), inter));
            }
        }
        best.map(|(_, i)| i.clone())
    }

    /// Validate structural integrity. A **conformant** manifest passes this: the
    /// version is recognized, every reference resolves, and portals address the
    /// Thread. Returns all problems at once.
    pub fn validate(&self) -> Result<(), ManifestError> {
        let mut errs = Vec::new();

        if !self.thread.starts_with("thread/") {
            errs.push(format!("unknown format tag '{}' (expected 'thread/…')", self.thread));
        }
        if self.world.id.trim().is_empty() {
            errs.push("world.id must not be empty".into());
        }

        let asset_ids: std::collections::HashSet<&str> =
            self.assets.iter().map(|a| a.id.as_str()).collect();
        let prefab_ids: std::collections::HashSet<StructuredId> =
            self.prefabs.iter().map(|p| p.id).collect();
        let behavior_ids: std::collections::HashSet<&str> =
            self.behaviors.iter().map(|b| b.id.as_str()).collect();

        for p in &self.prefabs {
            let sources =
                [p.mesh.asset.is_some(), p.mesh.builtin.is_some(), p.mesh.shape.is_some()];
            match sources.iter().filter(|s| **s).count() {
                0 => errs.push(format!(
                    "prefab {} has no mesh source (asset, builtin, or shape)",
                    p.id
                )),
                1 => {
                    if let Some(a) = &p.mesh.asset {
                        if !asset_ids.contains(a.as_str()) {
                            errs.push(format!("prefab {} references unknown asset '{a}'", p.id));
                        }
                    }
                    if let Some(shape) = &p.mesh.shape {
                        if let Err(e) = shape.validate() {
                            errs.push(format!("prefab {} shape: {e}", p.id));
                        }
                    }
                }
                _ => errs.push(format!(
                    "prefab {} sets more than one mesh source (asset/builtin/shape)",
                    p.id
                )),
            }
        }

        // Every placement — including nested children (the tree/DOM model).
        for (i, pl) in self.placements.iter().flat_map(|p| p.iter_tree()).enumerate() {
            if !prefab_ids.contains(&pl.prefab) {
                errs.push(format!("placement[{i}] references unknown prefab {}", pl.prefab));
            }
            if let Some(b) = &pl.behavior {
                if !behavior_ids.contains(b.as_str()) {
                    errs.push(format!("placement[{i}] references unknown behavior '{b}'"));
                }
            }
            if let Some(inter) = &pl.interaction {
                if inter.label.trim().is_empty() {
                    errs.push(format!("placement[{i}] interaction has an empty label"));
                }
                if inter.hits == 0 {
                    errs.push(format!("placement[{i}] interaction.hits must be at least 1"));
                }
            }
            if let Some(a) = &pl.animate {
                if !matches!(a.kind.as_str(), "spin" | "bob" | "path") {
                    errs.push(format!(
                        "placement[{i}] unknown animate kind '{}' (spin|bob|path)",
                        a.kind
                    ));
                }
                if a.kind == "path" && a.points.len() < 2 {
                    errs.push(format!("placement[{i}] animate path needs at least 2 points"));
                }
            }
        }

        for (i, rule) in self.styles.iter().enumerate() {
            if Selector::parse(&rule.select).is_none() {
                errs.push(format!("styles[{i}] has an empty/invalid selector '{}'", rule.select));
            }
            if let Some(inter) = &rule.interaction {
                if inter.label.trim().is_empty() {
                    errs.push(format!("styles[{i}] interaction has an empty label"));
                }
                if inter.hits == 0 {
                    errs.push(format!("styles[{i}] interaction.hits must be at least 1"));
                }
            }
        }

        for b in &self.behaviors {
            // Exactly one code source: a wasm asset, a weft module asset, or
            // a weft package export (`weft_pack` + `weft_export` together).
            let has_pack = b.weft_pack.is_some();
            let sources = usize::from(!b.wasm.is_empty()) + usize::from(b.weft.is_some()) + usize::from(has_pack);
            match sources {
                0 => errs.push(format!(
                    "behavior '{}' names no code (wasm, weft, or weft_pack+weft_export)",
                    b.id
                )),
                1 => {}
                _ => errs.push(format!("behavior '{}' names multiple code sources (pick one)", b.id)),
            }
            if has_pack != b.weft_export.is_some() {
                errs.push(format!(
                    "behavior '{}': weft_pack and weft_export go together",
                    b.id
                ));
            }
            if let Some(w) = &b.weft {
                if !asset_ids.contains(w.as_str()) {
                    errs.push(format!("behavior '{}' references unknown weft asset '{w}'", b.id));
                }
            }
            if let Some(p) = &b.weft_pack {
                if !asset_ids.contains(p.as_str()) {
                    errs.push(format!("behavior '{}' references unknown weft package asset '{p}'", b.id));
                }
            }
            if !b.wasm.is_empty() && !asset_ids.contains(b.wasm.as_str()) {
                errs.push(format!("behavior '{}' references unknown wasm asset '{}'", b.id, b.wasm));
            }
        }

        for p in &self.portals {
            if Locator::parse(&p.to).is_none() {
                errs.push(format!("portal '{}' has an invalid destination Locator '{}'", p.id, p.to));
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(ManifestError::Invalid(errs))
        }
    }
}
