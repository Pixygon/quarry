//! Hang things at sockets — fruit, lanterns, leaf clusters, props.
//!
//! A grown plant ends in sockets (position, direction, radius, and the branch
//! each tip belongs to). Hanging is the first composition: pick some of those
//! tips and place one thing at each, so a lantern tree carries its lanterns
//! exactly where its branches end and the fruit is a separate, reusable model
//! (Trellis-made, carved, or grown) instanced once per tip.
//!
//! Which tips carry one is **the tip's own business**, not the list's: every
//! socket scores itself from its branch id, and the lowest scores win. A pool
//! that gains or loses tips — a sapling that has not sprouted them yet, a
//! branch cut off, a fruit picked — leaves every other tip's answer alone.
//! That is what makes picking and regrowth state instead of a re-roll.
use serde::{Deserialize, Serialize};

use crate::grow::{Socket, SocketKind};
use crate::rand::unit;

/// Slots on a tip's branch id: whether it is chosen, and how the thing sits.
const SLOT_CHOSEN: u32 = 0;
const SLOT_SCALE: u32 = 1;
const SLOT_YAW: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HangRecipe {
    /// Which sockets to hang at. A lantern hangs at `fruit`, a garland at
    /// `tip` — and out of season there are no fruit sockets, so nothing hangs.
    pub kind: SocketKind,
    /// How many sockets get one (0 = every eligible socket).
    pub count: u32,
    /// Only tips at this branching generation or deeper.
    pub min_level: u32,
    /// Uniform scale of the hung thing.
    pub scale: f32,
    /// ± fraction of random scale per instance.
    pub scale_jitter: f32,
    /// Stem: metres between the tip and the top of the hung thing.
    pub drop: f32,
    /// Random yaw per instance (a hanging thing turns freely).
    pub spin: bool,
    pub seed: u32,
}

impl Default for HangRecipe {
    fn default() -> Self {
        Self { kind: SocketKind::Tip, count: 0, min_level: 0, scale: 1.0, scale_jitter: 0.15, drop: 0.15, spin: true, seed: 1 }
    }
}

/// One placed instance (TRS, glTF conventions; rotation is `[x y z w]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub socket: String,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Place one thing per chosen socket, hanging straight down from the tip.
/// `top_y` is the hung model's own top (its bounds max Y), so its top sits
/// `drop` metres under the tip whatever its origin is.
pub fn hang(sockets: &[Socket], top_y: f32, r: &HangRecipe) -> Vec<Placement> {
    let mut pool: Vec<(f32, &Socket)> = sockets
        .iter()
        .filter(|s| s.kind == r.kind && s.level >= r.min_level)
        .map(|s| (unit(r.seed, s.branch, SLOT_CHOSEN), s))
        .collect();
    // Lowest score first; the name breaks ties so the order is never the
    // order the sockets happened to arrive in.
    pool.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    if r.count > 0 {
        pool.truncate(r.count as usize);
    }
    pool.iter()
        .map(|(_, s)| {
            let sc = r.scale * (1.0 + (unit(r.seed, s.branch, SLOT_SCALE) * 2.0 - 1.0) * r.scale_jitter);
            let yaw = if r.spin { unit(r.seed, s.branch, SLOT_YAW) * std::f32::consts::TAU } else { 0.0 };
            let (sy, cy) = (yaw * 0.5).sin_cos();
            Placement {
                socket: s.name.clone(),
                translation: [s.position[0], s.position[1] - r.drop - top_y * sc, s.position[2]],
                rotation: [0.0, sy, 0.0, cy],
                scale: [sc, sc, sc],
            }
        })
        .collect()
}
