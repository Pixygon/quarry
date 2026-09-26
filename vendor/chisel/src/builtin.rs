//! The builtin primitives — the geometry a World Manifest's `builtin` names.
//!
//! When a manifest says `"builtin": "cube"`, every browser on the Thread has to
//! produce *the same box*, or the same world is a different place depending on
//! who renders it. That makes these five shapes part of the standard's surface
//! rather than any one engine's convenience, and it is why they live in the
//! mesher instead of inside a renderer: a renderer is a private choice, and
//! this is not.
//!
//! **Every primitive is one metre.** `scale: [1,1,1]` means one metre on each
//! axis for all of them, so a placement's scale reads as metres with no
//! per-shape correction. The cylinder was two metres tall for months — every
//! lamp post and plinth in every world came out double height, and each author
//! quietly compensated in their own direction. Hence
//! [`tests::every_primitive_is_one_metre`], which measures rather than trusts.
//!
//! Winding is **counter-clockwise seen from outside**, matching glTF, so
//! back-face culling keeps the outside visible.

use std::f32::consts::PI;

use crate::MeshData;

/// Geometry for a builtin name, or `None` if the name isn't one.
///
/// `cube` · `sphere` · `cylinder` · `capsule` · `plane` (a flat ground quad in
/// XZ) · `quad` (a vertical, double-sided panel facing ±Z — the signboard).
pub fn mesh(name: &str) -> Option<MeshData> {
    match name {
        "cube" => Some(cube()),
        "sphere" => Some(sphere(24, 16)),
        "cylinder" => Some(cylinder(24)),
        "capsule" => Some(capsule(24, 8)),
        "plane" => Some(plane()),
        "quad" => Some(quad()),
        _ => None,
    }
}

/// Every builtin name, in a stable order — for tools that enumerate them.
pub const NAMES: [&str; 6] = ["cube", "sphere", "cylinder", "capsule", "plane", "quad"];

fn finish(mut m: MeshData) -> MeshData {
    // Planar UVs from local XZ (right for the ground case, harmless elsewhere;
    // the sphere overrides them with an equirectangular map).
    if m.uvs.is_empty() {
        m.uvs = m.positions.iter().map(|p| [p[0] + 0.5, p[2] + 0.5]).collect();
    }
    if m.tangents.is_empty() {
        m.tangents = vec![[1.0, 0.0, 0.0, 1.0]; m.positions.len()];
    }
    if m.colors.is_empty() {
        m.colors = vec![[1.0, 1.0, 1.0, 1.0]; m.positions.len()];
    }
    m
}

/// A 1 × 1 × 1 box centred on the origin.
pub fn cube() -> MeshData {
    let mut m = MeshData::default();
    // normal, then the face's four corners in CCW order seen from outside
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([1.0, 0.0, 0.0], [[1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, -1.0]]),
        (
            [-1.0, 0.0, 0.0],
            [[-1.0, -1.0, 1.0], [-1.0, -1.0, -1.0], [-1.0, 1.0, -1.0], [-1.0, 1.0, 1.0]],
        ),
        ([0.0, 1.0, 0.0], [[-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]]),
        (
            [0.0, -1.0, 0.0],
            [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, -1.0, -1.0], [-1.0, -1.0, -1.0]],
        ),
        ([0.0, 0.0, 1.0], [[-1.0, -1.0, 1.0], [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, -1.0, 1.0]]),
        (
            [0.0, 0.0, -1.0],
            [[1.0, -1.0, -1.0], [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0], [-1.0, -1.0, -1.0]],
        ),
    ];
    for (normal, corners) in &faces {
        let base = m.positions.len() as u32;
        for c in corners {
            // ±1 corners, halved to the unit box
            m.positions.push([c[0] * 0.5, c[1] * 0.5, c[2] * 0.5]);
            m.normals.push(*normal);
        }
        m.indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    finish(m)
}

/// A ⌀1 sphere, equirectangularly mapped — a textured sphere IS a globe.
pub fn sphere(segments: u32, rings: u32) -> MeshData {
    let mut m = MeshData::default();
    let r = 0.5;
    for ring in 0..=rings {
        let phi = PI * ring as f32 / rings as f32;
        let y = r * phi.cos();
        let ring_r = r * phi.sin();
        for seg in 0..=segments {
            let theta = 2.0 * PI * seg as f32 / segments as f32;
            let (x, z) = (ring_r * theta.cos(), ring_r * theta.sin());
            let len = (x * x + y * y + z * z).sqrt().max(1e-6);
            m.positions.push([x, y, z]);
            m.normals.push([x / len, y / len, z / len]);
            m.uvs.push([
                0.5 + z.atan2(x) / std::f32::consts::TAU,
                0.5 - (y / len).asin() / PI,
            ]);
        }
    }
    for ring in 0..rings {
        for seg in 0..segments {
            let cur = ring * (segments + 1) + seg;
            let next = cur + segments + 1;
            // Rings run top→bottom, so `next` is the ring below.
            m.indices.extend_from_slice(&[cur, cur + 1, next, cur + 1, next + 1, next]);
        }
    }
    finish(m)
}

/// A ⌀1 × 1 m cylinder, centred on the origin.
pub fn cylinder(segments: u32) -> MeshData {
    let mut m = MeshData::default();
    let (r, h) = (0.5f32, 0.5f32);
    let mut cap = |m: &mut MeshData, y: f32, ny: f32| {
        let centre = m.positions.len() as u32;
        m.positions.push([0.0, y, 0.0]);
        m.normals.push([0.0, ny, 0.0]);
        for seg in 0..=segments {
            let t = std::f32::consts::TAU * seg as f32 / segments as f32;
            m.positions.push([r * t.cos(), y, r * t.sin()]);
            m.normals.push([0.0, ny, 0.0]);
        }
        for seg in 0..segments {
            let (a, b) = (centre + 1 + seg, centre + 2 + seg);
            if ny > 0.0 {
                m.indices.extend_from_slice(&[centre, b, a]);
            } else {
                m.indices.extend_from_slice(&[centre, a, b]);
            }
        }
    };
    cap(&mut m, h, 1.0);
    cap(&mut m, -h, -1.0);
    // The side wall, with its own vertices so the cap normals stay hard.
    let base = m.positions.len() as u32;
    for seg in 0..=segments {
        let t = std::f32::consts::TAU * seg as f32 / segments as f32;
        let (c, s) = (t.cos(), t.sin());
        for y in [h, -h] {
            m.positions.push([r * c, y, r * s]);
            m.normals.push([c, 0.0, s]);
        }
    }
    for seg in 0..segments {
        let (tl, bl) = (base + seg * 2, base + seg * 2 + 1);
        let (tr, br) = (tl + 2, bl + 2);
        // The quad tl→tr→br→bl is counter-clockwise seen from outside the
        // wall; the other diagonal winds it inside-out, which renders as a
        // cylinder you can see straight through from the front.
        m.indices.extend_from_slice(&[tl, tr, br, tl, br, bl]);
    }
    finish(m)
}

/// A 1 m tall capsule, ⌀0.25 — the stand-in body.
///
/// One metre like the rest, which is a **change**: it used to be three units
/// tall (⌀1 with a 2 m barrel), so a manifest had to scale by height ÷ 3 to
/// get a person. That is precisely the trap the cylinder was, and one
/// primitive that means something different by `scale` than its neighbours is
/// worse than no primitive at all — an author has to remember which. Now a
/// capsule scaled to 1.69 is 1.69 m tall, like a cube scaled to 1.69 is 1.69 m
/// wide.
pub fn capsule(segments: u32, rings: u32) -> MeshData {
    let mut m = MeshData::default();
    let r = 0.125f32;
    // total height = barrel + two hemispherical caps = 1 m
    let half_barrel = (1.0 - 2.0 * r) / 2.0;
    // A sphere cut in half and pulled apart: rings 0..=rings walk the top cap,
    // then the same again for the bottom, each offset to its end of the barrel.
    for (half, lift) in [(0u32, half_barrel), (1, -half_barrel)] {
        for ring in 0..=rings {
            let phi = (PI / 2.0) * ring as f32 / rings as f32 + if half == 1 { PI / 2.0 } else { 0.0 };
            let (y, ring_r) = (r * phi.cos(), r * phi.sin());
            for seg in 0..=segments {
                let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
                let (x, z) = (ring_r * theta.cos(), ring_r * theta.sin());
                let len = (x * x + y * y + z * z).sqrt().max(1e-6);
                m.positions.push([x, y + lift, z]);
                m.normals.push([x / len, y / len, z / len]);
            }
        }
    }
    let per_half = (rings + 1) * (segments + 1);
    for half in 0..2u32 {
        for ring in 0..rings {
            for seg in 0..segments {
                let cur = half * per_half + ring * (segments + 1) + seg;
                let next = cur + segments + 1;
                m.indices.extend_from_slice(&[cur, cur + 1, next, cur + 1, next + 1, next]);
            }
        }
    }
    // The barrel, joining the two caps' equators.
    let (top_eq, bot_eq) = (rings * (segments + 1), per_half);
    for seg in 0..segments {
        let (tl, tr) = (top_eq + seg, top_eq + seg + 1);
        let (bl, br) = (bot_eq + seg, bot_eq + seg + 1);
        m.indices.extend_from_slice(&[tl, tr, br, tl, br, bl]);
    }
    finish(m)
}

/// A 1 × 1 ground quad in XZ, facing +Y.
pub fn plane() -> MeshData {
    let mut m = MeshData::default();
    for (x, z) in [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)] {
        m.positions.push([x, 0.0, z]);
        m.normals.push([0.0, 1.0, 0.0]);
    }
    m.indices.extend_from_slice(&[0, 2, 1, 1, 2, 3]);
    finish(m)
}

/// A 1 × 1 **vertical, double-sided** panel facing ±Z — the signboard,
/// painting and poster primitive. Double-sided because a sign you can walk
/// behind and lose is a worse sign; the back face mirrors its U so the text
/// reads the right way round from both sides.
pub fn quad() -> MeshData {
    let mut m = MeshData::default();
    let v = |m: &mut MeshData, x: f32, y: f32, nz: f32, u: f32, vv: f32| {
        m.positions.push([x, y, 0.0]);
        m.normals.push([0.0, 0.0, nz]);
        m.uvs.push([u, vv]);
    };
    // front (+Z)
    v(&mut m, -0.5, -0.5, 1.0, 0.0, 1.0);
    v(&mut m, 0.5, -0.5, 1.0, 1.0, 1.0);
    v(&mut m, 0.5, 0.5, 1.0, 1.0, 0.0);
    v(&mut m, -0.5, 0.5, 1.0, 0.0, 0.0);
    // back (−Z), mirrored winding + mirrored U
    v(&mut m, 0.5, -0.5, -1.0, 0.0, 1.0);
    v(&mut m, -0.5, -0.5, -1.0, 1.0, 1.0);
    v(&mut m, -0.5, 0.5, -1.0, 1.0, 0.0);
    v(&mut m, 0.5, 0.5, -1.0, 0.0, 0.0);
    m.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    finish(m)
}
