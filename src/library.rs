//! What the Make door is allowed to offer: the `weft-model` standard library,
//! read out of the package itself rather than typed into a list here.
//!
//! The library knows its own arity and its own types; the only thing it does
//! not carry is what the numbers *mean* to a person. So the shape of a form
//! (how many fields, integer or metre) is read from the package — it cannot
//! drift — and the words beside the fields are ours.

use serde_json::{json, Value};

/// A callable shape and what a person needs to fill it in.
struct Shape {
    export: &'static str,
    label: &'static str,
    kind: &'static str,
    /// One name per parameter, in order. Must match the library's arity or
    /// the entry is dropped — a wrong form is worse than no form.
    names: &'static [&'static str],
    defaults: &'static [f64],
    material: &'static str,
}

/// The carved things worth a door. Combinators (`at`, `join`, `ring_of`…)
/// and the material constructors are the library's *grammar*: they compose
/// models, they are not models, so they are not offered as one-click shapes.
const SHAPES: &[Shape] = &[
    Shape { export: "column", label: "Column", kind: "column",
            names: &["height", "shaft ⌀"], defaults: &[5.2, 0.44], material: "marble" },
    Shape { export: "arch", label: "Arch", kind: "arch",
            names: &["width", "height", "depth"], defaults: &[3.0, 4.0, 0.6], material: "sandstone" },
    Shape { export: "stairs", label: "Stairs", kind: "stairs",
            names: &["steps", "rise", "tread", "width"], defaults: &[12.0, 0.18, 0.28, 1.2], material: "granite" },
    Shape { export: "table", label: "Table", kind: "furniture",
            names: &["width", "depth", "height"], defaults: &[1.6, 0.9, 0.75], material: "wood" },
    Shape { export: "vase", label: "Vase", kind: "vessel",
            names: &["height", "belly r", "neck r"], defaults: &[0.9, 0.3, 0.12], material: "terracotta" },
    Shape { export: "amphora", label: "Amphora", kind: "vessel",
            names: &["height"], defaults: &[1.2], material: "" },
    Shape { export: "bowl", label: "Bowl", kind: "vessel",
            names: &["radius", "wall"], defaults: &[0.5, 0.06], material: "granite" },
    Shape { export: "rock", label: "Rock", kind: "rock",
            names: &["radius", "lumpiness"], defaults: &[0.8, 3.0], material: "granite" },
    Shape { export: "cube", label: "Block", kind: "prop",
            names: &["width", "height", "depth"], defaults: &[1.0, 1.0, 1.0], material: "granite" },
    Shape { export: "cylinder", label: "Cylinder", kind: "prop",
            names: &["radius", "height"], defaults: &[0.4, 1.0], material: "granite" },
    Shape { export: "sphere", label: "Sphere", kind: "prop",
            names: &["radius"], defaults: &[0.5], material: "marble" },
    Shape { export: "capsule", label: "Capsule", kind: "prop",
            names: &["radius", "height"], defaults: &[0.3, 1.2], material: "plaster" },
    Shape { export: "cone", label: "Cone", kind: "prop",
            names: &["base r", "top r", "height"], defaults: &[0.5, 0.0, 1.0], material: "sandstone" },
    Shape { export: "torus", label: "Torus", kind: "prop",
            names: &["ring r", "tube r"], defaults: &[0.5, 0.1], material: "brass" },
];

/// The library, read out loud: every shape with the arity the package
/// actually declares, and every material it can dress a bare part in.
pub fn catalog() -> Value {
    let package: Value = serde_json::from_str(&chisel::weft_model::standard_library())
        .unwrap_or_else(|_| json!({}));
    let exports = package["exports"].as_object().cloned().unwrap_or_default();
    let defs = package["defs"].clone();
    let params_of = |name: &str| -> Option<Vec<String>> {
        let hash = exports.get(name)?.as_str()?;
        Some(
            defs[hash]["params"]
                .as_array()?
                .iter()
                .map(|t| match t.as_str() {
                    Some("Int") => "int".to_string(),
                    Some("Fix") => "number".to_string(),
                    Some(other) => other.to_lowercase(),
                    None => "value".to_string(),
                })
                .collect(),
        )
    };

    let mut models = Vec::new();
    for s in SHAPES {
        let Some(types) = params_of(s.export) else { continue };
        // The library moved and we did not: say nothing rather than draw a
        // form that will be refused.
        if types.len() != s.names.len() {
            eprintln!("library: '{}' takes {} args, the door offers {} — hidden", s.export, types.len(), s.names.len());
            continue;
        }
        models.push(json!({
            "export": s.export,
            "label": s.label,
            "kind": s.kind,
            "material": s.material,
            "args": s.names.iter().zip(types).zip(s.defaults)
                .map(|((name, ty), d)| json!({ "name": name, "type": ty, "default": d }))
                .collect::<Vec<_>>(),
        }));
    }

    // A material is an export that asks for nothing and hands back a full
    // PBR set — the library's own definition, so a new one appears here the
    // day it is added upstream.
    let mut materials: Vec<String> = exports
        .iter()
        .filter(|(_, h)| {
            let d = &defs[h.as_str().unwrap_or_default()];
            d["params"].as_array().is_some_and(|p| p.is_empty())
                && d["ret"].get("Record").is_some_and(|r| r.get("color").is_some())
        })
        .map(|(name, _)| name.clone())
        .collect();
    materials.sort();

    // Grove's door is not a list of shapes — a tree is one recipe with many
    // rules. The blank is the grower's own default, serialised here, so the
    // form gains a field the day the grower gains a rule.
    let blank = serde_json::to_value(grove::GrowRecipe::default()).unwrap_or_else(|_| json!({}));

    json!({
        "package": "weft-model",
        "models": models,
        "materials": materials,
        "grove": { "blank": blank },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The door's forms are shaped by the library's own declared arity. If a
    /// shape's arg list and the library disagree the shape is hidden rather
    /// than drawn wrong — so an empty or shrinking catalog is the signal that
    /// `weft-model` moved under us.
    #[test]
    fn every_offered_shape_matches_the_library() {
        let c = catalog();
        let models = c["models"].as_array().expect("models");
        assert_eq!(
            models.len(),
            SHAPES.len(),
            "the library no longer agrees with the door — check which shape was dropped"
        );
        let column = models
            .iter()
            .find(|m| m["export"] == "column")
            .expect("the library still carves columns");
        let args = column["args"].as_array().unwrap();
        assert_eq!(args.len(), 2, "column(height, diameter)");
        assert_eq!(args[0]["name"], "height");
        assert_eq!(args[0]["type"], "number");
        assert_eq!(models.iter().find(|m| m["export"] == "stairs").unwrap()["args"][0]["type"], "int",
                   "a stair count is a whole number, and the library says so");
    }

    /// A material is an export that asks for nothing and returns a full PBR
    /// set — found, not listed, so a new one upstream appears by itself.
    #[test]
    fn the_materials_come_from_the_library() {
        let c = catalog();
        let mats: Vec<&str> = c["materials"].as_array().unwrap().iter().map(|m| m.as_str().unwrap()).collect();
        for expected in ["marble", "granite", "terracotta", "wood"] {
            assert!(mats.contains(&expected), "{expected} is missing from {mats:?}");
        }
        assert!(!mats.contains(&"column"), "a shape is not a material");
        assert!(!mats.contains(&"tint"), "a material *modifier* takes arguments");
    }

    /// Grove's form is built from the grower's own blank.
    #[test]
    fn the_grove_door_starts_from_the_growers_default() {
        let c = catalog();
        let blank = c["grove"]["blank"].as_object().expect("a blank recipe");
        for rule in ["seed", "height", "trunk_radius", "forks", "levels"] {
            assert!(blank.contains_key(rule), "{rule} is missing from the blank");
        }
    }
}
