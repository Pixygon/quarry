//! # arch — the shared architecture math of the Thread
//!
//! One geometry, every builder: the markup `<room>` compiler, the museum
//! generator, and anything else that raises a wall all call THIS module, so
//! a ring of wall segments meets its columns the same way everywhere. Walls
//! looked "off" precisely when three builders each hand-rolled the same
//! trigonometry; the cure is one copy, tested once.
//!
//! **Convention** (matches [`crate::markup`]'s room element): azimuths in
//! degrees, `0` faces −z ("north", away from the default spawn), `180` faces
//! +z (toward arrivals). A point at azimuth `az` and radius `r` sits at
//! `(sin(az)·r, −cos(az)·r)`.

/// Direction of an azimuth: unit `(x, z)`.
pub fn az_dir(az_deg: f32) -> (f32, f32) {
    let a = az_deg.to_radians();
    (a.sin(), -a.cos())
}

/// One straight wall piece of a ring: where it stands, how it turns, how
/// long it is. Height/thickness are the caller's voice.
#[derive(Debug, Clone, Copy)]
pub struct RingSegment {
    pub x: f32,
    pub z: f32,
    pub yaw_deg: f32,
    pub len: f32,
}

/// The gate threshold that actually opens a gate: at least half a segment
/// plus grace, whatever was requested — a gate azimuth landing exactly on a
/// column joint must still clear both straddling segments.
pub fn gate_reach(n_segments: usize, requested_deg: f32) -> f32 {
    requested_deg.max(0.55 * 360.0 / n_segments.max(1) as f32)
}

/// A circular wall of `n` chord segments at radius `r`, with openings cut at
/// each azimuth in `gates` (segments whose midpoint is within
/// [`gate_reach`] degrees of a gate are omitted). Segments overlap joints
/// slightly (`len + 0.2` is the caller's usual scale) so rings read sealed.
pub fn ring_segments(r: f32, n: usize, gates: &[f32], gate_reach_deg: f32) -> Vec<RingSegment> {
    let seg_len = 2.0 * r * (std::f32::consts::PI / n.max(1) as f32).sin();
    (0..n)
        .filter_map(|k| {
            let az_mid = (k as f32 + 0.5) / n as f32 * 360.0;
            if gates.iter().any(|g| ang_dist(az_mid, *g) <= gate_reach_deg) {
                return None;
            }
            let (dx, dz) = az_dir(az_mid);
            Some(RingSegment {
                x: dx * r,
                z: dz * r,
                yaw_deg: -az_mid,
                len: seg_len,
            })
        })
        .collect()
}

/// The ring's column positions — one at every joint between segments.
pub fn ring_joints(r: f32, n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .map(|k| {
            let (dx, dz) = az_dir(k as f32 / n as f32 * 360.0);
            (dx * r, dz * r)
        })
        .collect()
}

/// A corridor radiating from a center at azimuth `az` — the frame every
/// wing-like structure positions and TURNS its pieces in. The yaw helpers
/// are the whole point: wall/beam skew bugs come from hand-deriving these
/// signs per call site. Derive once, test once, reuse everywhere.
///
/// Frame: `dir` runs outward along the corridor; `side` is the lateral axis
/// (positive = to the right when walking outward). Placement yaw convention:
/// a yaw of ψ maps local +X to `(cos ψ, −sin ψ)` and local +Z (a quad's
/// normal) to `(sin ψ, cos ψ)`.
#[derive(Debug, Clone, Copy)]
pub struct Corridor {
    pub az: f32,
}

impl Corridor {
    pub fn new(az: f32) -> Self {
        Corridor { az }
    }

    /// Outward direction of the corridor.
    pub fn dir(&self) -> (f32, f32) {
        az_dir(self.az)
    }

    /// The lateral axis (`side > 0` is this way).
    pub fn side_axis(&self) -> (f32, f32) {
        let a = self.az.to_radians();
        (a.cos(), a.sin())
    }

    /// A point `r` out along the corridor, `side` across it, `lift` up.
    pub fn at(&self, r: f32, side: f32, lift: f32) -> [f32; 3] {
        let (dx, dz) = self.dir();
        let (sx, sz) = self.side_axis();
        [dx * r + sx * side, lift, dz * r + sz * side]
    }

    /// Yaw for an X-long element RUNNING DOWN the corridor (side walls).
    pub fn yaw_along(&self) -> f32 {
        90.0 - self.az
    }

    /// Yaw for an X-long element SPANNING the corridor (lintels, end walls) —
    /// also the yaw for a quad FACING BACK toward the corridor's origin
    /// (gate titles, end paintings: the visitor walks outward into them).
    pub fn yaw_across(&self) -> f32 {
        -self.az
    }

    /// Yaw for a quad on the corridor's `side`, FACING the centerline —
    /// reading boards mounted on a side wall.
    pub fn yaw_face_in(&self, side: f32) -> f32 {
        if side < 0.0 {
            90.0 - self.az
        } else {
            270.0 - self.az
        }
    }
}

/// Minimal angular distance between two azimuths, degrees.
pub fn ang_dist(a: f32, b: f32) -> f32 {
    let mut d = (a - b).abs() % 360.0;
    if d > 180.0 {
        d = 360.0 - d;
    }
    d
}
