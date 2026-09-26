//! Terra — the Thread's landscape generator.
//!
//! One physically-motivated simulation, read many ways. Almost every engine
//! generates terrain, then biomes, then vegetation, then rocks as four
//! independent systems that never quite agree — forests that ignore the
//! valley, deserts on the wet side of a mountain, boulders with no reason to
//! be there. That disagreement *is* what fake looks like.
//!
//! So this does the opposite: uplift, then erosion, then flow — and the mesh,
//! the rivers, the climate, the plant cover and the stone all fall out of the
//! same run, unable to contradict each other.
//!
//! ```text
//!   uplift ──▶ erode ──▶ flow ──▶ climate ──▶ cover ──▶ scatter
//!   (why the      (what      (rivers,   (rain      (biome    (plants,
//!    land rises)   water      for free)  shadow)    weights)   stone)
//!                  did)
//! ```
//!
//! Pure: no rendering, no I/O, no globals. Both renderers ask this one
//! implementation, so a world cannot look different depending on who opened
//! it — the same rule the builtin primitives live under.

use infinite_manifest::texture::TextureRecipe;
use std::f32::consts::PI;

/// How dramatic the land is. A preset picks the uplift blend and the erosion
/// budget; everything else follows from the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relief {
    /// Wide and gentle — river plains, long sightlines.
    Plains,
    /// Rolling country. The default: readable, walkable, never flat.
    Hills,
    /// Sharp ridgelines, deep valleys, scree. Ridged multifractal dominant.
    Alpine,
    /// Heavily eroded soft rock — mesas, gullies, exposed strata.
    Badlands,
    /// Mostly below sea level, with island chains rising out of it.
    Archipelago,
}

impl Relief {
    /// Parse the manifest's spelling. Unknown names fall back to hills rather
    /// than failing a world — an author's typo should cost them drama, not
    /// their landscape.
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "plains" => Relief::Plains,
            "alpine" => Relief::Alpine,
            "badlands" => Relief::Badlands,
            "archipelago" => Relief::Archipelago,
            _ => Relief::Hills,
        }
    }

    /// (ridged share, vertical scale in metres, erosion droplets per cell).
    fn profile(self) -> (f32, f32, f32) {
        match self {
            Relief::Plains => (0.05, 55.0, 0.30),
            Relief::Hills => (0.32, 260.0, 0.55),
            Relief::Alpine => (0.82, 1500.0, 0.90),
            Relief::Badlands => (0.48, 420.0, 1.60), // erosion IS the look
            Relief::Archipelago => (0.45, 520.0, 0.55),
        }
    }
}

/// A landscape, as a recipe. Seven fields describe a world.
#[derive(Debug, Clone)]
pub struct TerrainRecipe {
    pub seed: u32,
    pub relief: Relief,
    /// Metres above the datum that counts as shoreline.
    pub sea_level: f32,
    /// 0 = equator, 1 = pole. Drives the temperature band.
    pub latitude: f32,
    /// Prevailing wind, normalised on use. Rain falls on the windward side.
    pub wind: [f32; 2],
    /// 0 = arid, 1 = drenched. The moisture the wind starts with.
    pub humidity: f32,
    /// Metres per cell of the simulation grid. Bigger = coarser, faster.
    pub cell_size: f32,
}

impl Default for TerrainRecipe {
    fn default() -> Self {
        Self {
            seed: 1337,
            relief: Relief::Hills,
            sea_level: 0.0,
            latitude: 0.5,
            wind: [1.0, 0.0],
            humidity: 0.6,
            cell_size: 4.0,
        }
    }
}

/// The Whittaker set. Weights, never an id — biomes blend at their edges, and
/// a hard boundary in nature is the exception, not the rule.
pub const BIOMES: [&str; 8] = [
    "water", "beach", "desert", "grassland", "shrubland", "forest", "taiga", "alpine",
];
pub const B_WATER: usize = 0;
pub const B_BEACH: usize = 1;
pub const B_DESERT: usize = 2;
pub const B_GRASSLAND: usize = 3;
pub const B_SHRUBLAND: usize = 4;
pub const B_FOREST: usize = 5;
pub const B_TAIGA: usize = 6;
pub const B_ALPINE: usize = 7;

/// One simulated tile: every field the rest of the system reads.
///
/// All the vectors are `size × size`, row-major, and describe the *same*
/// cells — so a lookup at one index is consistent across every field, which
/// is the whole point of running one simulation.
#[derive(Debug, Clone)]
pub struct Field {
    pub size: usize,
    pub cell_size: f32,
    /// Metres. Post-erosion.
    pub height: Vec<f32>,
    /// Metres of loose material sitting on bedrock — scree, sand, alluvium.
    /// Thermal erosion writes it; the rock scatter reads it.
    pub sediment: Vec<f32>,
    /// How much upstream land drains through each cell. The river map.
    pub flow: Vec<f32>,
    /// 0..1, where 1 is vertical.
    pub slope: Vec<f32>,
    /// °C-ish, unitless but monotonic: latitude minus altitude lapse.
    pub temperature: Vec<f32>,
    /// 0..1 after rain shadow and river proximity.
    pub moisture: Vec<f32>,
    /// Metres of soil over bedrock — the regolith. THIS is the field that
    /// decides where a landscape is green and where it is bare stone. On an
    /// alpine slope every cell shares one climate, so climate cannot draw
    /// that boundary; soil depth can, and does, because plants need
    /// something to root in and soil only rests where the ground will hold it.
    pub soil: Vec<f32>,
    /// Laplacian of the surface, roughly metres of deviation from the local
    /// plane. **Positive is concave** — a hollow, which collects soil, water
    /// and snow. Negative is convex — a shoulder, which sheds all three.
    pub curvature: Vec<f32>,
    /// `size*size*8` — biome weights per cell, summing to 1.
    pub biome: Vec<f32>,
}

impl Field {

    /// Cut a square sub-field out of this one, sharing its cell size.
    ///
    /// This is how a landscape is broken into renderable tiles WITHOUT
    /// breaking the simulation into tiles. Erosion is not a function of a
    /// point — it is a sequential simulation over a grid — so two tiles that
    /// simulate the same ground separately erode it into two different shapes
    /// and the traveler finds a cliff where they meet. Overlap margins only
    /// narrow that gap: measured across a real alpine seam it fell from 94 m
    /// to about 10 m as the margin grew, and then stopped falling, because
    /// what remains is the order the droplets were applied in, not the area
    /// they were applied to.
    ///
    /// Simulating once and windowing is seamless by construction, and it is
    /// also CHEAPER than tiling — nine tiles with margins simulate about
    /// twice the cells of one field that covers the same ground, because
    /// every margin is ground simulated twice and thrown away.
    pub fn window(&self, x0: usize, y0: usize, size: usize) -> Field {
        let size = size.min(self.size.saturating_sub(x0)).min(self.size.saturating_sub(y0));
        let mut f = Field {
            size,
            cell_size: self.cell_size,
            height: vec![0.0; size * size],
            sediment: vec![0.0; size * size],
            flow: vec![0.0; size * size],
            slope: vec![0.0; size * size],
            temperature: vec![0.0; size * size],
            moisture: vec![0.0; size * size],
            soil: vec![0.0; size * size],
            curvature: vec![0.0; size * size],
            biome: vec![0.0; size * size * BIOMES.len()],
        };
        for y in 0..size {
            for x in 0..size {
                let src = self.idx(x0 + x, y0 + y);
                let dst = y * size + x;
                f.height[dst] = self.height[src];
                f.sediment[dst] = self.sediment[src];
                f.flow[dst] = self.flow[src];
                f.slope[dst] = self.slope[src];
                f.temperature[dst] = self.temperature[src];
                f.moisture[dst] = self.moisture[src];
                f.soil[dst] = self.soil[src];
                f.curvature[dst] = self.curvature[src];
                let (sb, db) = (src * BIOMES.len(), dst * BIOMES.len());
                f.biome[db..db + BIOMES.len()]
                    .copy_from_slice(&self.biome[sb..sb + BIOMES.len()]);
            }
        }
        f
    }
    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.size + x
    }

    /// Bilinear height at a fractional cell coordinate — the sampler meshing
    /// and scatter both use, so a tree stands exactly on the ground the mesh
    /// draws.
    pub fn height_at(&self, fx: f32, fy: f32) -> f32 {
        bilinear(&self.height, self.size, fx, fy)
    }

    pub fn flow_at(&self, fx: f32, fy: f32) -> f32 {
        bilinear(&self.flow, self.size, fx, fy)
    }

    pub fn sediment_at(&self, fx: f32, fy: f32) -> f32 {
        bilinear(&self.sediment, self.size, fx, fy)
    }

    /// The dominant biome at a cell, for tests and debug colour.
    pub fn dominant(&self, x: usize, y: usize) -> usize {
        let base = self.idx(x, y) * BIOMES.len();
        let mut best = 0;
        for b in 1..BIOMES.len() {
            if self.biome[base + b] > self.biome[base + best] {
                best = b;
            }
        }
        best
    }
}

fn bilinear(v: &[f32], size: usize, fx: f32, fy: f32) -> f32 {
    let max = size as f32 - 1.0;
    let x = fx.clamp(0.0, max);
    let y = fy.clamp(0.0, max);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(size - 1);
    let y1 = (y0 + 1).min(size - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let a = v[y0 * size + x0] * (1.0 - tx) + v[y0 * size + x1] * tx;
    let b = v[y1 * size + x0] * (1.0 - tx) + v[y1 * size + x1] * tx;
    a * (1.0 - ty) + b * ty
}

// ─── noise ────────────────────────────────────────────────────────────────
// Value noise with a hashed lattice: no tables, no allocation, identical on
// every machine. Terrain quality comes from what we *stack* on top of it —
// the ridges, the warp, and above all the erosion — not from the noise basis.

/// A 0..1 hash of a world cell. The droplet seeding needs a value that
/// depends on WHERE a cell is in the world, never on where it sits in the
/// tile currently being simulated.
#[inline]
fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    hash2(x, y, seed) * 0.5 + 0.5
}

#[inline]
fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(1_442_695_041));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[inline]
fn smootherstep(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Exposed for the flora scatter, which needs the same lattice the terrain
/// used so its clustering lines up with the land rather than fighting it.
pub fn value_noise_pub(x: f32, y: f32, seed: u32) -> f32 {
    value_noise(x, y, seed)
}

fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let tx = smootherstep(x - xi);
    let ty = smootherstep(y - yi);
    // Saturating casts can land xi on i32::MAX, and the neighbouring lattice
    // point is then one past it. Wrap rather than panic: the lattice is
    // periodic anyway, so wrapping is the mathematically honest answer and a
    // sample is never worth killing a world over.
    let (xi, yi) = (xi as i32, yi as i32);
    let (xi1, yi1) = (xi.wrapping_add(1), yi.wrapping_add(1));
    let a = hash2(xi, yi, seed);
    let b = hash2(xi1, yi, seed);
    let c = hash2(xi, yi1, seed);
    let d = hash2(xi1, yi1, seed);
    let top = a + (b - a) * tx;
    let bot = c + (d - c) * tx;
    top + (bot - top) * ty
}

/// Fractal Brownian motion — the soft, rolling half of a landscape.
fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut f, mut amp, mut sum, mut norm) = (1.0f32, 1.0f32, 0.0f32, 0.0f32);
    for o in 0..octaves {
        sum += value_noise(x * f, y * f, seed.wrapping_add(o * 7919)) * amp;
        norm += amp;
        f *= 2.0;
        amp *= 0.5;
    }
    sum / norm.max(1e-6)
}

/// Ridged multifractal — the sharp half.
///
/// `1 − |n|` folds the noise into creases, and weighting each octave by the
/// last keeps detail on the ridges while leaving the valleys smooth. This is
/// what makes mountains read as *ranges* with connected crests instead of a
/// field of lumps, and plain fbm will never do it however it is tuned.
fn ridged(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut f, mut amp, mut sum, mut norm) = (1.0f32, 1.0f32, 0.0f32, 0.0f32);
    let mut weight = 1.0f32;
    for o in 0..octaves {
        let n = value_noise(x * f, y * f, seed.wrapping_add(o * 6151));
        let mut r = 1.0 - n.abs();
        r *= r;
        r *= weight;
        weight = (r * 2.0).clamp(0.0, 1.0);
        sum += r * amp;
        norm += amp;
        f *= 2.0;
        amp *= 0.5;
    }
    (sum / norm.max(1e-6)) * 2.0 - 1.0
}

/// The uplift field: *why* the land rises, before any water touches it.
///
/// Ridged where the relief wants mountains, fbm underneath for the lowlands,
/// and the whole domain warped so nothing lines up with the sampling grid.
fn uplift(wx: f32, wy: f32, r: &TerrainRecipe) -> f32 {
    let (ridge_share, scale, _) = r.relief.profile();
    // Continental frequency — one "range" per few kilometres.
    let f = 1.0 / 2600.0;
    // Domain warp: sample the field at a position that is itself noisy.
    let wxw = wx + fbm(wx * f * 1.7, wy * f * 1.7, 3, r.seed ^ 0x51ed) * 900.0;
    let wyw = wy + fbm(wx * f * 1.7 + 31.4, wy * f * 1.7 + 17.2, 3, r.seed ^ 0x9e37) * 900.0;

    let soft = fbm(wxw * f, wyw * f, 6, r.seed) * 0.5 + 0.5;
    let sharp = ridged(wxw * f, wyw * f, 6, r.seed ^ 0x2545) * 0.5 + 0.5;
    let mut h = soft * (1.0 - ridge_share) + sharp * ridge_share;

    // A continental mask so the world has coasts instead of an endless plateau.
    let cont = fbm(wxw * f * 0.35, wyw * f * 0.35, 3, r.seed ^ 0x7f4a);
    h += cont * 0.35;

    if r.relief == Relief::Archipelago {
        // Push the datum up so most of the field drowns and only peaks remain.
        h -= 0.26;
    }
    h * scale
}

// ─── the simulation ───────────────────────────────────────────────────────

/// Generate one tile of `size × size` cells whose south-west corner sits at
/// `(origin_x, origin_y)` metres.
///
/// `margin` cells of context are simulated beyond every edge and then trimmed,
/// so a droplet that would have run off the edge still carves the part of its
/// valley that lands inside the tile. That is what makes neighbouring tiles
/// agree without ever having met — the seam problem, solved by overlap rather
/// than by stitching.
pub fn generate(r: &TerrainRecipe, size: usize, origin_x: f32, origin_y: f32) -> Field {
    let margin = (size / 4).clamp(8, 48);
    let big = size + margin * 2;
    let cs = r.cell_size;

    // 1. Uplift.
    let mut height = vec![0.0f32; big * big];
    for y in 0..big {
        for x in 0..big {
            let wx = origin_x + (x as f32 - margin as f32) * cs;
            let wy = origin_y + (y as f32 - margin as f32) * cs;
            height[y * big + x] = uplift(wx, wy, r);
        }
    }

    // 2. Erosion — the step that turns noise into landscape.
    let mut sediment = vec![0.0f32; big * big];
    let (_, _, budget) = r.relief.profile();
    // World cell index of this padded grid's (0, 0), so droplets can be
    // seeded from where they are in the world rather than in the tile.
    let wx0 = (origin_x / cs).round() as i64 - margin as i64;
    let wy0 = (origin_y / cs).round() as i64 - margin as i64;
    hydraulic(&mut height, &mut sediment, big, cs, budget, r.seed, wx0, wy0);
    thermal(&mut height, &mut sediment, big, cs, 12);

    // Loose material settles: a light smoothing of the deposition map. Raw
    // per-droplet deposits are salt-and-pepper and dithered every shoreline.
    let sediment = blur(&sediment, big, 2);

    // 3. Flow — free, and it is the river map.
    let flow = flow_accumulation(&height, big);

    // 4/5. Climate and cover, per cell.
    let flow_soft = blur(&flow, big, 2);
    let slope_full = slopes(&height, big, cs);
    let moisture_full = blur(&moisture(&height, &flow, big, cs, r), big, 4);

    // 4b. Soil. Everything above the bedrock reads from this.
    let curvature_full = curvatures(&height, big, cs);
    let soil_full = regolith(&height, &slope_full, &moisture_full, &flow_soft, big, cs, r);

    // Trim the margin away: everything below is tile-local.
    let mut f = Field {
        size,
        cell_size: cs,
        height: vec![0.0; size * size],
        sediment: vec![0.0; size * size],
        flow: vec![0.0; size * size],
        slope: vec![0.0; size * size],
        temperature: vec![0.0; size * size],
        moisture: vec![0.0; size * size],
        soil: vec![0.0; size * size],
        curvature: vec![0.0; size * size],
        biome: vec![0.0; size * size * BIOMES.len()],
    };
    for y in 0..size {
        for x in 0..size {
            let s = (y + margin) * big + (x + margin);
            let d = y * size + x;
            f.height[d] = height[s];
            f.sediment[d] = sediment[s];
            f.flow[d] = flow[s];
            f.slope[d] = slope_full[s];
            f.moisture[d] = moisture_full[s];
            f.soil[d] = soil_full[s];
            f.curvature[d] = curvature_full[s];
            f.temperature[d] = temperature(height[s], r);
            let w = classify(
                height[s],
                slope_full[s],
                temperature(height[s], r),
                moisture_full[s],
                flow_soft[s],
                r,
            );
            f.biome[d * BIOMES.len()..(d + 1) * BIOMES.len()].copy_from_slice(&w);
        }
    }
    f
}

/// Curvature: how far each cell sits from the plane its neighbours describe.
///
/// Cheap (one Laplacian) and worth far more than it costs, because almost
/// every surface question is really a curvature question: hollows collect and
/// shoulders shed, and that is what makes the patchwork of soil and bare rock
/// on a mountainside look like a mountainside instead of a gradient.
fn curvatures(h: &[f32], size: usize, cell: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; size * size];
    for y in 1..size - 1 {
        for x in 1..size - 1 {
            let i = y * size + x;
            let lap = h[i - 1] + h[i + 1] + h[i - size] + h[i + size] - 4.0 * h[i];
            // Normalized by cell size so curvature means the same thing at any
            // resolution -- otherwise a coarse tile reads as smooth ground.
            out[i] = lap / cell;
        }
    }
    out
}

/// Regolith — the soil balance.
///
/// Soil is not decoration painted onto a landscape; it is the standing
/// balance of three competing processes, and running them is what produces
/// the shapes the eye recognises:
///
/// 1. **Production.** Bedrock weathers into soil, faster where it is wet and
///    warm. The rate falls as soil thickens, because a soil blanket shields
///    the rock beneath it -- which is why thin soils grow quickly and deep
///    ones barely grow at all.
/// 2. **Creep.** Soil flows downhill, slowly, driven by the slope of the
///    surface it sits on. This is diffusion, and diffusion is what empties
///    the convex shoulders and fills the concave hollows.
/// 3. **Loss.** Above the angle of repose soil simply will not stay, and
///    running water strips what is left in the channels.
///
/// The result is bare stone on ridges, steep faces and scoured channels, with
/// soil banked in the benches and hollows between them. That patchwork is the
/// single most recognisable thing about ground above the treeline.
fn regolith(
    height: &[f32],
    slope: &[f32],
    moisture: &[f32],
    flow: &[f32],
    size: usize,
    cell: f32,
    r: &TerrainRecipe,
) -> Vec<f32> {
    let mut soil = vec![0.0f32; size * size];
    // Metres of soil produced per pass on bare rock in ideal conditions.
    const PRODUCTION: f32 = 0.09;
    // Depth at which the soil blanket has substantially shut production down.
    const SHIELD: f32 = 0.6;
    const CREEP: f32 = 0.28;
    const PASSES: usize = 18;

    for _ in 0..PASSES {
        // 1. Production.
        for i in 0..soil.len() {
            let t = temperature(height[i], r);
            // Weathering wants water and warmth. Frost weathering does work in
            // the cold too, but it SHATTERS rock rather than making soil -- it
            // feeds the scree, not the meadow.
            let warm = smoothstep(-14.0, 8.0, t);
            let wet = 0.25 + 0.75 * moisture[i];
            soil[i] += PRODUCTION * warm * wet * (-soil[i] / SHIELD).exp();
        }

        // 2. Creep: soil diffuses down the surface it lies on.
        let mut moved = soil.clone();
        for y in 1..size - 1 {
            for x in 1..size - 1 {
                let i = y * size + x;
                if soil[i] <= 1e-4 {
                    continue;
                }
                let here = height[i] + soil[i];
                let mut total = 0.0f32;
                let mut drop = [0.0f32; 4];
                for (k, j) in [i - 1, i + 1, i - size, i + size].into_iter().enumerate() {
                    let d = here - (height[j] + soil[j]);
                    if d > 0.0 {
                        drop[k] = d;
                        total += d;
                    }
                }
                if total <= 0.0 {
                    continue;
                }
                // Creep transports a FRACTION of the column, not an absolute
                // depth: the standard law is q = -K * depth * grad(z), so a
                // slope with little soil on it moves little soil. Written as
                // an absolute amount instead, `min(soil[i])` quietly meant
                // "move everything" on any real slope, and the whole
                // mountainside scoured itself to bedrock every pass.
                //
                // Capped at half the column so the diffusion settles rather
                // than oscillating between neighbours.
                let frac = (CREEP * total / cell).min(0.5);
                let budget = soil[i] * frac;
                moved[i] -= budget;
                for (k, j) in [i - 1, i + 1, i - size, i + size].into_iter().enumerate() {
                    if drop[k] > 0.0 {
                        moved[j] += budget * drop[k] / total;
                    }
                }
            }
        }
        soil = moved;

        // 3. Loss: the angle of repose, and the channels.
        for i in 0..soil.len() {
            // What a slope of this steepness can hold at all. Near-vertical
            // rock holds nothing, which is why cliffs are grey everywhere.
            let hold = 2.6 * (1.0 - slope[i].clamp(0.0, 1.0)).powi(3);
            soil[i] = soil[i].min(hold);
            // Running water carries soil away; the more that drains through a
            // cell the less stays in it.
            let wash = (flow[i] / 240.0).clamp(0.0, 0.85);
            soil[i] *= 1.0 - wash;
        }
    }
    soil
}
/// Droplet hydraulic erosion.
///
/// Each droplet is a particle with position, velocity, water and a sediment
/// load. It flows downhill along the interpolated gradient; where the slope is
/// steep it takes more than it carries and cuts, where the ground flattens it
/// drops the surplus. Repeated a few hundred thousand times this produces the
/// dendritic drainage every real landscape has and no noise function contains:
/// V-profile valleys upstream, alluvial fans where the grade eases, ridgelines
/// that connect because the water had to go *around* them.
/// `density` is droplets per cell per pass; `wx0`/`wy0` are the WORLD cell
/// indices of grid position (0, 0).
fn hydraulic(
    h: &mut [f32],
    sed: &mut [f32],
    size: usize,
    cell: f32,
    density: f32,
    seed: u32,
    wx0: i64,
    wy0: i64,
) {
    const INERTIA: f32 = 0.05;
    const CAPACITY: f32 = 4.0;
    const EROSION: f32 = 0.3;
    const DEPOSITION: f32 = 0.3;
    const EVAPORATION: f32 = 0.02;
    const GRAVITY: f32 = 10.0;
    const MAX_STEPS: usize = 64;
    const RADIUS: i32 = 2;

    // Droplets are seeded from their WORLD position, not from a per-tile
    // random stream.
    //
    // This is what makes an unbounded landscape possible at all. Erosion is
    // not a function of a point — it is a simulation over a grid — so two
    // tiles covering the same ground with different droplets erode it into
    // two different shapes, and the traveler walks up to a 30 m cliff where
    // they meet. No amount of overlap margin fixes that, because the margin
    // was simulating the right AREA with the wrong RAIN. Hash the world cell
    // instead and the same ground gets the same weather no matter which tile
    // is asking.
    let whole = density.floor().max(0.0) as usize;
    let extra = density - density.floor();

    for gy in 0..size {
        for gx in 0..size {
            let wx = (wx0 + gx as i64) as i32;
            let wy = (wy0 + gy as i64) as i32;
            // Fractional density: the cell spawns its last droplet only if
            // its own hash says so, which keeps the count right on average
            // without making it depend on the tile.
            let n = whole + usize::from(hash01(wx, wy, seed ^ 0xd10d) < extra);
            for k in 0..n {
                let ks = seed.wrapping_add(k as u32 * 7919);
                let mut px = gx as f32 + hash01(wx, wy, ks ^ 0x1111);
                let mut py = gy as f32 + hash01(wx, wy, ks ^ 0x2222);
                if px >= size as f32 - 1.0 || py >= size as f32 - 1.0 {
                    continue;
                }
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        let mut water = 1.0f32;
        let mut carry = 0.0f32;
        let mut speed = 1.0f32;

        for _ in 0..MAX_STEPS {
            let (gx, gy) = gradient(h, size, px, py);
            // Momentum: a droplet does not turn on a sixpence, which is what
            // keeps valleys running straight instead of scribbling.
            dx = dx * INERTIA - gx * (1.0 - INERTIA);
            dy = dy * INERTIA - gy * (1.0 - INERTIA);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-5 {
                break;
            }
            dx /= len;
            dy /= len;
            let (nx, ny) = (px + dx, py + dy);
            if nx < 1.0 || ny < 1.0 || nx >= size as f32 - 2.0 || ny >= size as f32 - 2.0 {
                break;
            }
            let old_h = bilinear(h, size, px, py);
            let new_h = bilinear(h, size, nx, ny);
            let drop = old_h - new_h;

            let capacity = (drop.max(0.0) * speed * water * CAPACITY).max(0.01);
            if carry > capacity || drop < 0.0 {
                // Uphill or over capacity: put material down. Filling a pit
                // with exactly the step height is what lets lakes and flats
                // form instead of the droplet drilling forever.
                let amount = if drop < 0.0 {
                    (-drop).min(carry)
                } else {
                    (carry - capacity) * DEPOSITION
                };
                carry -= amount;
                deposit(h, sed, size, px, py, amount);
            } else {
                let amount = ((capacity - carry) * EROSION).min(drop.max(0.0));
                carry += amount;
                erode(h, sed, size, px, py, amount, RADIUS);
            }

            speed = (speed * speed + drop * GRAVITY).max(0.0).sqrt();
            water *= 1.0 - EVAPORATION;
            if water < 0.01 {
                break;
            }
            px = nx;
            py = ny;
        }
        let _ = cell;
        }
        }
    }
}

fn gradient(h: &[f32], size: usize, px: f32, py: f32) -> (f32, f32) {
    let x = px.floor() as usize;
    let y = py.floor() as usize;
    let x1 = (x + 1).min(size - 1);
    let y1 = (y + 1).min(size - 1);
    let tx = px - x as f32;
    let ty = py - y as f32;
    let h00 = h[y * size + x];
    let h10 = h[y * size + x1];
    let h01 = h[y1 * size + x];
    let h11 = h[y1 * size + x1];
    (
        (h10 - h00) * (1.0 - ty) + (h11 - h01) * ty,
        (h01 - h00) * (1.0 - tx) + (h11 - h10) * tx,
    )
}

fn deposit(h: &mut [f32], sed: &mut [f32], size: usize, px: f32, py: f32, amount: f32) {
    let x = px.floor() as usize;
    let y = py.floor() as usize;
    let x1 = (x + 1).min(size - 1);
    let y1 = (y + 1).min(size - 1);
    let tx = px - x as f32;
    let ty = py - y as f32;
    for (i, w) in [
        (y * size + x, (1.0 - tx) * (1.0 - ty)),
        (y * size + x1, tx * (1.0 - ty)),
        (y1 * size + x, (1.0 - tx) * ty),
        (y1 * size + x1, tx * ty),
    ] {
        h[i] += amount * w;
        sed[i] += amount * w;
    }
}

/// Remove material over a small disc rather than a single cell — a
/// single-cell cut leaves needle artefacts that read as noise, not erosion.
fn erode(h: &mut [f32], sed: &mut [f32], size: usize, px: f32, py: f32, amount: f32, radius: i32) {
    let cx = px.round() as i32;
    let cy = py.round() as i32;
    let mut total = 0.0f32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= radius as f32 {
                total += 1.0 - d / (radius as f32 + 1e-6);
            }
        }
    }
    if total <= 0.0 {
        return;
    }
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let (x, y) = (cx + dx, cy + dy);
            if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
                continue;
            }
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d > radius as f32 {
                continue;
            }
            let w = (1.0 - d / (radius as f32 + 1e-6)) / total;
            let i = y as usize * size + x as usize;
            let take = amount * w;
            h[i] -= take;
            sed[i] = (sed[i] - take).max(0.0);
        }
    }
}

/// Thermal erosion: anything steeper than the talus angle slides.
///
/// This is what puts scree at the foot of a cliff and stops mountains from
/// being infinitely sharp. The sediment it moves is also the map the rock
/// scatter reads later — a boulder field with a reason to exist.
fn thermal(h: &mut [f32], sed: &mut [f32], size: usize, cell: f32, iterations: usize) {
    // ~34°, the repose angle of loose rock.
    let talus = 0.7 * cell;
    for _ in 0..iterations {
        let src = h.to_vec();
        for y in 1..size - 1 {
            for x in 1..size - 1 {
                let i = y * size + x;
                let me = src[i];
                let mut total = 0.0f32;
                let mut diffs = [0.0f32; 4];
                for (k, n) in [i - 1, i + 1, i - size, i + size].iter().enumerate() {
                    let d = me - src[*n];
                    if d > talus {
                        diffs[k] = d - talus;
                        total += d - talus;
                    }
                }
                if total <= 0.0 {
                    continue;
                }
                // Damped by 0.25 to stop the cell oscillating with its
                // neighbours across iterations. It used to also cap at
                // `me * 0.5` — the cell's own ALTITUDE — which is nonsense for
                // anything below the datum: a cell at −40 m got a negative
                // budget, so it pumped material UP toward zero and pushed its
                // neighbours down, tearing every seabed into ±90 m spikes and
                // driving the sediment map negative. How much rock can slide
                // has nothing to do with how high above sea level it sits.
                let move_total = total * 0.25;
                h[i] -= move_total;
                for (k, n) in [i - 1, i + 1, i - size, i + size].iter().enumerate() {
                    if diffs[k] > 0.0 {
                        let share = move_total * (diffs[k] / total);
                        h[*n] += share;
                        sed[*n] += share;
                    }
                }
            }
        }
    }
}

/// Flow accumulation: how much upstream land drains through each cell.
///
/// Cells are visited from high to low, each pushing its accumulated water into
/// its lowest neighbour (D8). One sort and one pass — and the result is the
/// river network, which also feeds moisture and riverbed stone. Free, because
/// the erosion already shaped the surface it runs on.
fn flow_accumulation(h: &[f32], size: usize) -> Vec<f32> {
    let mut order: Vec<u32> = (0..(size * size) as u32).collect();
    order.sort_unstable_by(|a, b| h[*b as usize].total_cmp(&h[*a as usize]));
    let mut flow = vec![1.0f32; size * size];
    for &i in &order {
        let i = i as usize;
        let (x, y) = (i % size, i / size);
        if x == 0 || y == 0 || x + 1 == size || y + 1 == size {
            continue;
        }
        let mut lowest = i;
        for n in [i - 1, i + 1, i - size, i + size, i - size - 1, i - size + 1, i + size - 1, i + size + 1] {
            if h[n] < h[lowest] {
                lowest = n;
            }
        }
        if lowest != i {
            let f = flow[i];
            flow[lowest] += f;
        }
    }
    flow
}

/// Separable box blur, `radius` cells. Cheap, and enough to stop a spiky
/// field from dithering a classification boundary.
fn blur(v: &[f32], size: usize, radius: usize) -> Vec<f32> {
    let mut tmp = vec![0.0f32; v.len()];
    let mut out = vec![0.0f32; v.len()];
    let r = radius as isize;
    for y in 0..size {
        for x in 0..size {
            let (mut sum, mut n) = (0.0f32, 0.0f32);
            for d in -r..=r {
                let xx = x as isize + d;
                if xx >= 0 && (xx as usize) < size {
                    sum += v[y * size + xx as usize];
                    n += 1.0;
                }
            }
            tmp[y * size + x] = sum / n;
        }
    }
    for y in 0..size {
        for x in 0..size {
            let (mut sum, mut n) = (0.0f32, 0.0f32);
            for d in -r..=r {
                let yy = y as isize + d;
                if yy >= 0 && (yy as usize) < size {
                    sum += tmp[yy as usize * size + x];
                    n += 1.0;
                }
            }
            out[y * size + x] = sum / n;
        }
    }
    out
}

fn slopes(h: &[f32], size: usize, cell: f32) -> Vec<f32> {
    let mut s = vec![0.0f32; size * size];
    for y in 0..size {
        for x in 0..size {
            let xm = x.saturating_sub(1);
            let xp = (x + 1).min(size - 1);
            let ym = y.saturating_sub(1);
            let yp = (y + 1).min(size - 1);
            let dx = (h[y * size + xp] - h[y * size + xm]) / (2.0 * cell);
            let dy = (h[yp * size + x] - h[ym * size + x]) / (2.0 * cell);
            // tan → 0..1, where 1 is a wall.
            s[y * size + x] = ((dx * dx + dy * dy).sqrt()).atan() / (PI * 0.5);
        }
    }
    s
}

fn temperature(height: f32, r: &TerrainRecipe) -> f32 {
    // Latitude band, then the lapse rate: ~6.5 °C per kilometre climbed.
    let base = 27.0 - r.latitude.clamp(0.0, 1.0) * 52.0;
    base - (height - r.sea_level).max(0.0) * 0.0065
}

/// Moisture: base humidity, minus what the wind lost climbing, plus what the
/// rivers give back.
///
/// The march is the interesting half. Stepping upwind from each cell and
/// shedding moisture wherever the land *rose* puts deserts behind mountains
/// and rainforest on the windward slope — the orographic rain shadow, which is
/// the single most recognisable climate pattern on Earth and is almost free
/// once the height field exists.
fn moisture(h: &[f32], flow: &[f32], size: usize, cell: f32, r: &TerrainRecipe) -> Vec<f32> {
    let wl = (r.wind[0] * r.wind[0] + r.wind[1] * r.wind[1]).sqrt().max(1e-5);
    let (wx, wy) = (r.wind[0] / wl, r.wind[1] / wl);
    let steps = 24;
    let stride = 3.0;

    let mut m = vec![0.0f32; size * size];
    for y in 0..size {
        for x in 0..size {
            let i = y * size + x;
            let start = r.humidity.clamp(0.0, 1.0);
            let mut climbed = 0.0f32;
            let mut prev = bilinear(h, size, x as f32, y as f32);
            // Walk UPWIND, so we accumulate what the air lost getting here.
            for s in 1..=steps {
                let sx = x as f32 - wx * s as f32 * stride;
                let sy = y as f32 - wy * s as f32 * stride;
                if sx < 0.0 || sy < 0.0 || sx >= size as f32 || sy >= size as f32 {
                    break;
                }
                let hh = bilinear(h, size, sx, sy);
                // Air climbing sheds rain; descending air is dry and stays dry.
                climbed += (prev - hh).max(0.0);
                prev = hh;
            }
            // Saturating: the first ridge takes most of the rain, and the
            // tenth cannot take what is no longer there. A linear subtraction
            // drove whole regions to exactly zero and made the map binary.
            let moisture = start * (-climbed / 900.0).exp();
            // Rivers water their own banks.
            let river = (flow[i] / 900.0).clamp(0.0, 1.0).sqrt() * 0.35;
            // Ocean is wet.
            let sea = if h[i] < r.sea_level { 0.4 } else { 0.0 };
            let _ = cell;
            m[i] = (moisture + river + sea).clamp(0.0, 1.0);
        }
    }
    m
}

/// Whittaker classification → blended weights.
fn classify(
    height: f32,
    slope: f32,
    temp: f32,
    moist: f32,
    flow: f32,
    r: &TerrainRecipe,
) -> [f32; 8] {
    let mut w = [0.0f32; 8];

    // Water: below the datum, or a river channel big enough to be one.
    let depth = r.sea_level - height;
    let sea = smoothstep(-3.0, 5.0, depth);
    let river = smoothstep(2200.0, 9000.0, flow) * (1.0 - smoothstep(0.0, 0.25, slope));
    w[B_WATER] = sea.max(river);

    // Beach: the metre or two above the waterline, and only where it's flat.
    w[B_BEACH] = smoothstep(-9.0, -1.0, depth) * (1.0 - sea) * (1.0 - smoothstep(0.10, 0.30, slope));

    // Bare rock wherever it is too steep to hold soil — this is what makes
    // cliffs read as cliffs rather than as vertical lawn.
    let bare = smoothstep(0.42, 0.68, slope);
    let cold = 1.0 - smoothstep(-8.0, 2.0, temp);
    w[B_ALPINE] = bare.max(cold * smoothstep(0.0, 0.3, slope + 0.15));

    // The Whittaker body: temperature × moisture.
    let warm = smoothstep(-1.0, 15.0, temp);
    let wet = moist;
    let land = (1.0 - w[B_WATER]).max(0.0) * (1.0 - w[B_ALPINE]).max(0.0);
    w[B_DESERT] = land * warm * (1.0 - smoothstep(0.16, 0.42, wet));
    w[B_GRASSLAND] = land * warm * bump(wet, 0.30, 0.60) * (1.0 - smoothstep(0.55, 0.85, wet));
    w[B_SHRUBLAND] = land * bump(wet, 0.22, 0.50) * (1.0 - warm * 0.4);
    w[B_FOREST] = land * warm * smoothstep(0.48, 0.78, wet);
    w[B_TAIGA] = land * (1.0 - warm) * smoothstep(0.30, 0.62, wet);

    // Normalise — weights, always summing to one.
    let sum: f32 = w.iter().sum();
    if sum > 1e-5 {
        for v in w.iter_mut() {
            *v /= sum;
        }
    } else {
        w[B_GRASSLAND] = 1.0;
    }
    w
}

#[inline]
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    if (b - a).abs() < 1e-6 {
        return if x >= b { 1.0 } else { 0.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A soft band: 1 in the middle of `[a,b]`, falling off outside it.
#[inline]
fn bump(x: f32, a: f32, b: f32) -> f32 {
    smoothstep(a - 0.15, a + 0.05, x) * (1.0 - smoothstep(b - 0.05, b + 0.20, x))
}

// ─── preview ──────────────────────────────────────────────────────────────

/// Debug colour for a cell: biome tint, shaded by the sun and darkened where
/// water runs. Not the renderer's material — this exists so a landscape can be
/// judged as a PNG in a second, without a browser or a GPU.
pub fn preview_rgb(f: &Field, x: usize, y: usize, r: &TerrainRecipe) -> [u8; 3] {
    let sed_bare = (f.sediment[f.idx(x, y)] / 6.0).clamp(0.0, 0.55);
    const TINT: [[f32; 3]; 8] = [
        [0.13, 0.30, 0.48], // water
        [0.80, 0.74, 0.55], // beach
        [0.78, 0.68, 0.45], // desert
        [0.52, 0.63, 0.30], // grassland
        [0.56, 0.53, 0.31], // shrubland
        [0.17, 0.34, 0.16], // forest
        [0.20, 0.31, 0.26], // taiga
        [0.55, 0.54, 0.52], // alpine rock
    ];
    let base = f.idx(x, y) * BIOMES.len();
    let mut c = [0.0f32; 3];
    for b in 0..BIOMES.len() {
        let w = f.biome[base + b];
        for k in 0..3 {
            c[k] += TINT[b][k] * w;
        }
    }
    // Loose scree and washed sediment read as bare ground, not lawn.
    for k in 0..3 {
        c[k] = c[k] * (1.0 - sed_bare) + [0.62, 0.56, 0.45][k] * sed_bare;
    }

    // Hillshade from the height gradient — a sun in the north-west, which is
    // the convention every map reader already has in their eye.
    let i = f.idx(x, y);
    let xm = x.saturating_sub(1);
    let xp = (x + 1).min(f.size - 1);
    let ym = y.saturating_sub(1);
    let yp = (y + 1).min(f.size - 1);
    let dx = (f.height[y * f.size + xp] - f.height[y * f.size + xm]) / (2.0 * f.cell_size);
    let dy = (f.height[yp * f.size + x] - f.height[ym * f.size + x]) / (2.0 * f.cell_size);
    let n = [-dx, 1.0, -dy];
    let nl = (n[0] * n[0] + 1.0 + n[2] * n[2]).sqrt();
    let l = [-0.5, 0.75, -0.43];
    let shade = ((n[0] * l[0] + n[1] * l[1] + n[2] * l[2]) / nl).clamp(0.15, 1.0);

    // Snow above the freezing line, on anything that isn't a wall.
    let snow = if f.temperature[i] < -2.0 && f.slope[i] < 0.55 { 0.75 } else { 0.0 };
    for k in 0..3 {
        c[k] = c[k] * (1.0 - snow) + 0.92 * snow;
        c[k] *= shade;
    }
    // Deep water reads deeper.
    if f.height[i] < r.sea_level {
        let d = ((r.sea_level - f.height[i]) / 120.0).clamp(0.0, 1.0);
        for k in 0..3 {
            c[k] *= 1.0 - d * 0.35;
        }
    }
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

/// Render a field to an RGB8 buffer for eyeballing.
pub fn preview(f: &Field, r: &TerrainRecipe) -> Vec<u8> {
    let mut px = Vec::with_capacity(f.size * f.size * 3);
    for y in 0..f.size {
        for x in 0..f.size {
            px.extend_from_slice(&preview_rgb(f, x, y, r));
        }
    }
    px
}
