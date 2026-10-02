# Changelog

All notable changes to **Quarry**. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this file is
materialized from the Pixygon Changelog API — edit there, not here.

## [0.11.1] — 2026-10-02

_The store now survives redeploys instead of being wiped each time._

### Changed
- The republish-groves script is now only needed to populate a fresh store, rather than being a required step after every single deploy.

### Fixed
- The store is now backed by persistent storage and survives redeploys. Previously, every redeploy wiped the store, losing grown designs, verdicts, and other data that wasn't a reseeded starter; now that data stays put across deploys.


## [0.11.0] — 2026-10-02

### Added
- The Chisel door in the Maker now has two ways to build: pick a library shape as before, or switch to "Model" mode and write (or paste) Chisel's own raw model format directly — carving steps in order, each a shape in a mode, with the materials the parts wear. A hint below the editor lists the available primitives, modes, and fields.
- When starting a model from scratch, you can now begin from a ready-made blank or from any of your previously published Chisel models, with the editor pre-filled and the title, kind, style, tags and codex carried over.

### Improved
- The recipe summary for Chisel items (in both the item page and the Maker preview) now shows the number of carving steps and materials used, instead of generic export arguments, making authored models easier to understand at a glance.

### Fixed
- Hidden elements in the Maker now reliably disappear regardless of their normal display style, fixing a case where the material selector could remain visible while editing a raw model.


## [0.10.0] — 2026-10-02

### Added
- Added support for submitting a model directly in Chisel's own format (carving steps and materials), as an alternative to the existing Weft-model library/arguments format. This lets agents author models the way they naturally write them, while still deduplicating identical designs regardless of how defaults are spelled out.
- Models can now carry a 'finish' — clearcoat, sheen/cloth, and glass/transmission — which is exported as standard glTF material extensions (clearcoat, sheen, transmission, volume, IOR). This means lacquered, fabric, and glass-like surfaces now render correctly in three.js, Unity glTFast, Blender and other standard viewers instead of looking flat.
- Added an impostor generator that bakes eight turntable views of a model into a single billboard atlas, which can stand in for the full mesh at a distance for much cheaper rendering of scenes with many repeated objects (e.g. forests).

### Improved
- Single primitive shapes (boxes, cylinders, cones, capsules, spheres, ellipsoids, and tori) are now meshed with exact analytic geometry instead of the general surface-nets carving pipeline. This produces far fewer triangles with crisp, exact rounding and normals for simple parts, while more complex carved shapes are unaffected.
- Exported glTF files now support up to three extra UV channels (TEXCOORD_1 through TEXCOORD_3) where needed, and base-color textures with transparent holes (e.g. leaf cards, impostor atlases) are now correctly marked as alpha-cutout materials so renderers don't fill in the holes.


## [0.9.0] — 2026-09-28

### Added
- Grove trees can now hang another design from the shelf at their sockets — for example, lanterns hung at every fruit socket on a lantern tree. The hung items are stored as placement data, not baked into the tree's geometry, so an importer (or Quarry's own turntable) instances the real design at each socket while the base plant model stays lightweight.
- The design page now shows a 'Hung' section listing what is hung, how many sockets it occupies, and the position of the first placement, so you can inspect a tree's lanterns (or other hung items) without opening the recipe JSON.
- The 3D preview for grove designs now renders hung items (like lanterns) placed on the tree in the live viewer, matching exactly what an importer would see.
- The publishing form for grove recipes now has a 'Hang at the sockets' section, letting you pick any published design, choose which socket kind to attach it to (fruit, tip, or bloom), and control count, scale, drop, and spin — all without editing raw JSON.


## [0.8.0] — 2026-09-27

### Added
- Carved shapes made with Chisel now automatically get two lower-detail levels (LOD1/LOD2), generated from the same recipe at coarser meshing. Distant objects render with dramatically fewer triangles while keeping the same design identity, and LOD switching on the turntable now actually swaps geometry.
- Chisel submissions can now be told to "rest on the ground" — for library shapes that are centred by default (block, cylinder, sphere, capsule, cone, torus, bowl), this stands the carving on its base instead of half-burying it when placed. The Make door pre-checks this option automatically for shapes that need it, and it's part of the recipe so existing designs are unaffected.
- Documentation added covering the Avatar system's current state and roadmap, and findings on Chisel's primitive shapes (including a known issue with the bowl shape's hollow rendering incorrectly, now marked as "not it" on the live shelf).


## [0.7.0] — 2026-09-27

### Added
- Quarry can now accept avatar items directly: post a manifested GLB (a Unity-exported file carrying the Portable Item Convention's manifest) to /publish and it is checked, measured, and stored byte-for-byte — no re-export, no data loss. The same file always produces the same design id, no matter who submits it.
- Avatar items are validated against every rule of the Portable Item Convention — manifest schema, valid slot, embedded textures, metallic-roughness materials, correctly-scaled metres, ground-aligned origin, and named skin joints for bodies and garments. Each rule's pass/fail result is recorded with the entry so exporters can see exactly what needs fixing.

### Changed
- Publishing an avatar item auto-fills its title, description, kind, tags, and linked Codex concept straight from the item's own manifest, so you no longer need to supply that information separately unless you want to override it.
- The upload size limit is now higher (64MB instead of 8MB) specifically for GLB uploads, since avatar items with embedded textures are much larger than ordinary recipes.
- The Make page's Avatar door now shows a live checklist of convention rules against an uploaded GLB instead of just being marked as unavailable.


## [0.6.0] — 2026-09-27

### Added
- Grove's door now models a plant's whole life, not just its species: you can set a seed, an age in seasons, and where in the year it stands (bud, leaf, bloom, fruit, seed drop, bare), plus mark it as withered. The entry page shows a new 'This moment' panel with stage, year, maturity and branch count for anything grown this way.

### Changed
- Sockets are now typed (tip, bloom, fruit, cut) and the entry page and maker UI group them by kind with counts (e.g. '124 tip · 28 fruit') instead of just listing unique kind names.
- The Grove maker form now exposes editable Seed, Age, Season, and Withered controls in place of the previous disabled placeholders, so age and season can actually be set when creating or editing a grown design.

### Improved
- Design hashing for grown recipes is now more stable: floating-point noise is rounded away everywhere (including inside nested species rules like leaves, bloom, fruit and wither curves), and fields left at their default no longer change a design's identity — even when a new rule is added later. This means republishing an existing tree, or upgrading Grove, won't silently fork or orphan existing designs.


## [0.5.1] — 2026-09-26

### Changed
- Documentation now clarifies the release process: this repo ships via `pearl ship` (tests, changelog, version bump, release, commit and push), while publishing to crates.io remains a separate manual step, and deploying is a separate step from shipping.

### Security
- Fixed a page-rendering issue where a specially crafted title or description in a published design could break out of the embedded data script and inject markup into the page. All embedded JSON is now escaped so it can no longer terminate its containing script tag.


