//! Building models: carve every part, bake every material, hand back
//! something a renderer or an exporter can use directly.
//!
//! This is the join between the [format](infinite_manifest::model) an agent
//! writes (or a Weft program computes) and the geometry a machine draws. It
//! is deliberately the *only* place that decides defaults, so a model made
//! by any route — hand-written JSON, a Weft program, a generator — comes out
//! PBR-complete: base color, normal, metallic, roughness, occlusion.

use infinite_manifest::model::{Model, PartMaterial};
pub use infinite_manifest::model::Finish;

use crate::texture::Baked;
use crate::{MeshData, MeshOptions, UvMode};

/// One carved, textured part.
pub struct BuiltPart {
    pub name: String,
    pub mesh: MeshData,
    /// The baked PBR set. `None` only when the part declared no recipe.
    pub baked: Option<Baked>,
    pub color: [f32; 4],
    pub emissive: f32,
    /// Render both faces: leaves, cloth, thin cards. Solid parts are false.
    pub double_sided: bool,
    /// The physical finish (preset already resolved): gloss, cloth, glass…
    pub finish: Finish,
}

/// A finished model: named parts, each with geometry and materials.
pub struct Built {
    pub name: String,
    pub parts: Vec<BuiltPart>,
}

impl Built {
    pub fn triangles(&self) -> usize {
        self.parts.iter().map(|p| p.mesh.indices.len() / 3).sum()
    }
    pub fn vertices(&self) -> usize {
        self.parts.iter().map(|p| p.mesh.positions.len()).sum()
    }
    /// World bounds over every part (for framing a camera).
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for part in &self.parts {
            for p in &part.mesh.positions {
                for c in 0..3 {
                    min[c] = min[c].min(p[c]);
                    max[c] = max[c].max(p[c]);
                }
            }
        }
        if min[0] > max[0] {
            return ([0.0; 3], [1.0; 3]);
        }
        (min, max)
    }
}

/// Carve + bake a model.
pub fn build(model: &Model) -> Result<Built, String> {
    model.validate()?;
    let parts = model
        .resolve()?
        .into_iter()
        .map(|rp| {
            let m: &PartMaterial = &rp.material;
            let finish = m.finish.resolved();
            let mut mesh = mesh_components(
                &rp.shape,
                MeshOptions {
                    resolution: m.resolution,
                    uv: UvMode::parse(&m.uv),
                    uv_scale: if m.uv_scale > 0.0 { m.uv_scale } else { 0.5 },
                },
            );
            // The curvature wear the mesher bakes into vertex colour, scaled:
            // 1 keeps it (carved stone), 0 is factory-clean (lacquer, plastic).
            if let Some(w) = finish.weathering {
                let w = w.clamp(0.0, 1.0);
                for c in &mut mesh.colors {
                    for ch in c.iter_mut().take(3) {
                        *ch = 1.0 - w * (1.0 - *ch);
                    }
                }
            }
            BuiltPart {
                name: rp.name,
                mesh,
                baked: m.texture.as_ref().map(crate::texture::bake),
                color: m.color,
                emissive: m.emissive,
                finish, double_sided: false,
            }
        })
        .collect();
    Ok(Built {
        name: model.name.clone(),
        parts,
    })
}

/// Mesh a shape, splitting plain unions into their own grids first.
///
/// This matters more than it sounds. A carve is sampled over one grid across
/// the whole shape's bounds, so a staircase four metres long gets centimetre
/// cells and mushy treads. But the steps of a staircase are a *union* — they
/// don't melt into each other — so each can be carved on its own tight grid
/// and the results concatenated. Crisper geometry, less of it, and faster:
/// twelve small grids are far cheaper than one big one. Blends, cuts and
/// intersections still mesh together, because there the parts genuinely
/// interact.
fn mesh_components(shape: &infinite_manifest::shape::Shape, opts: MeshOptions) -> MeshData {
    use infinite_manifest::shape::Shape;
    let splittable = match shape {
        Shape::Group(g) => {
            (g.op == "union") && g.at == [0.0; 3] && g.rot == 0.0 && g.parts.len() > 1
        }
        _ => false,
    };
    if !splittable {
        return crate::mesh_with(shape, opts);
    }
    let Shape::Group(g) = shape else {
        unreachable!()
    };
    let mut out = MeshData::default();
    for part in &g.parts {
        let piece = mesh_components(part, opts);
        let base = out.positions.len() as u32;
        out.positions.extend(piece.positions);
        out.normals.extend(piece.normals);
        out.uvs.extend(piece.uvs);
        out.tangents.extend(piece.tangents);
        out.colors.extend(piece.colors);
        out.indices.extend(piece.indices.iter().map(|i| i + base));
    }
    out
}

/// Export a built model as one self-contained binary glTF — every part a
/// mesh, every material complete.
pub fn export_glb(built: &Built) -> Result<Vec<u8>, String> {
    let meshes: Vec<crate::gltf::SceneMesh> = built
        .parts
        .iter()
        .map(|p| crate::gltf::SceneMesh {
            name: p.name.clone(),
            mesh: &p.mesh,
            baked: p.baked.as_ref(),
            base_color: p.color,
            emissive: p.emissive,
            finish: p.finish.clone(), double_sided: p.double_sided,
        })
        .collect();
    let nodes: Vec<crate::gltf::SceneNode> = built
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| crate::gltf::SceneNode {
            name: p.name.clone(),
            mesh: i,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
        })
        .collect();
    crate::gltf::write_glb_scene(&meshes, &nodes)
}
