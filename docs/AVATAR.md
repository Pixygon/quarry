# Avatar in the Quarry — the roadmap

Written 2026-09-27, after the Avatar door opened (Quarry 0.7.0). This is
the brief for the session that takes it further. Read `src/avatar.rs`,
`pixygon-packages/@pixygon/avatar/CONVENTION.md` and
`com.pixygon.avatar/README.md` first; they are the state of play.

## Where it stands

- **`thread-avatar` is a data model, not a maker.** `AvatarSlot` (26),
  `AvatarSpec` (slot → partId + body height), `AvatarRaceMode` (biology
  defaults + a morph weight), `AvatarCatalog`, and an `AvatarRenderer` seam
  that something else implements. Nothing in Rust turns a spec into a mesh.
  `loom` carries a spec in a passport. So an avatar part is not derived the
  way stone is carved or wood is grown.
- **The unit of exchange is a manifested GLB** — one file that is the whole
  item: mesh, materials, embedded textures, and `asset.extras.pixygonItem`
  (schema 1: partId, kind, slot, title, lore, codexSlug, stats, placement).
  Metres, Y-up, +Z; origin at grip / worn anchor / feet; PBR
  metallic-roughness; bodies and garments **skinned to the shared skeleton
  by bone name**, rigid props snap to a named bone. Unity has the baker, the
  uploader (`PixygonPartSync` → `/v1/avatar/assets`) and `SkinnedRebinder`;
  the web has `@pixygon/avatar/parts`.
- **The door is the convention's checker.** `POST /derive` or `/publish`
  with a GLB body: ten rules ticked or crossed (`glb · manifest · slot · id
  · embedded · pbr · triangles · metres · origin · skinned|attach`), facts
  measured off the file, bytes stored untouched, design = sha256 of them,
  words from the manifest. The crosses are the bug report an exporter never
  had. `Bones` on the table shows the rig; the manifest's `codexSlug` feeds
  Concept beside.
- **Two absences shape everything below.** The live library
  (`GET /v1/avatar/assets`) holds **nothing**, and **no canonical bone-name
  list exists anywhere** — the convention says "by bone name" and never
  says which names. The checker records joint names; it cannot judge them.

## The idea

A dressed avatar should keep the Quarry's promise the way a tree does:
**publish a recipe, the store derives the artifact.** The recipe is an
`AvatarSpec` — slot → partId, a body height, a race — and the artifact is
one dressed, skinned GLB, content-addressed by the spec and the designs it
names. Parts are entries; a spec is an entry whose recipe points at other
entries. Nothing here is possible until a body with the shared skeleton
exists, so the order is the order of what has to exist first.

## Build order

1. **The shared skeleton, named.** One canonical bone list — Hips, Spine,
   Chest, Neck, Head, Shoulder/UpperArm/LowerArm/Hand ×2, UpperLeg/LowerLeg/
   Foot/Toes ×2, and the snap bones props use — in `infinite-avatar`
   (`skeleton.rs`), mirrored in `com.pixygon.avatar` and
   `@pixygon/avatar`. Then the Quarry's `skinned` rule becomes real: which
   required bones are missing, which extra ones are there, whether a
   `boneMap` fix could close the gap. This is one afternoon and it unblocks
   everything; do it before any body is exported, or the first body defines
   the skeleton by accident.
2. **The first body, through the door.** Veilwalkers' `PixygonBodySync`
   exports a rigged body as kind `body`; drop it in the Avatar door before
   anything else. Every cross it earns is a fix in the exporter, not in the
   Quarry. When it adheres, publish it: the shelf has a body, the Avatar
   chip has a body, the `Bones` toggle shows the skeleton everyone binds to.
   Also the first garment and the first prop — three files, three kinds.
3. **Dressing on the table (viewer only).** With a body on the shelf, the
   table can dress it: pick a body, pick parts, and three.js does what
   `SkinnedRebinder` and `bindPartToRig` do — rebind skinned parts to the
   body's bones by name, parent rigid ones to their `snapBone`, apply
   `AvatarAsset.fix` (scale → rotation → position → boneMap) before every
   bind, `placement` offsets on equip. The spec that describes the outcome
   is shown as JSON beside the table. Nothing is stored yet; this is the
   Studio's job done where the parts live, and it proves the skeleton.
4. **The Quarry derives the dressed body.** `package: "avatar-spec"`: the
   spec in, one skinned GLB out — merge the body and its parts in Rust (the
   `gltf` crate is already a dependency; skins joined by bone name, one
   skeleton, materials and textures carried across, the manifest of the
   body kept and the worn parts listed in extras). Design id = hash of the
   spec + the part designs. Republish a part → republish the specs that
   wear it, and they keep their ids. This is the step that makes an avatar
   a Quarry model in the full sense.
5. **The library bridge.** `/v1/avatar/assets` is the Studio's library; the
   Quarry is the checker and the viewer. Either the API mirrors adhering
   entries from the Quarry (partId as the key, `url` the Quarry's artifact),
   or `PixygonPartSync` uploads to the Quarry first and the Quarry pushes
   what passes. Decide with the founder; do not run two sources of truth.
6. **Races on the table.** `AvatarRaceMode` applies biology defaults; the
   morph weight (`< 1` = proto-forms, Caul) needs a body mesher or blend
   shapes in the body GLB. Blend shapes are the honest first step: the body
   carries `morphTargets`, the race sets weights, the Quarry shows them.
7. **Actors.** `com.pixygon.actor` (Humanoids = Actor + Avatar) and
   procedural animation are the game's, not the store's. The Quarry's part
   is done when a spec derives to a dressed, correctly skinned GLB that
   glTFast loads and the actor can drive.

## Ground rules

- **Never mutate meaning.** The stored bytes are the maker's bytes. Fixes
  are data (`AvatarAsset.fix`) applied on view and on bind, never written
  back.
- **partId keys everything.** Never a name, never a database `_id`, never a
  Quarry design id — the design is the *file's* identity, the partId is the
  *item's*.
- **The checker's crosses are bug reports, not blockers.** A failing file
  can still be derived and looked at; it cannot dress a body.
- **Test with files the test owns.** `src/avatar.rs` builds its fixtures by
  carving with Chisel and injecting the manifest into the JSON chunk; do the
  same for a body — a capsule with a two-bone skin is enough to prove a
  rebind.
- Same rules as the rest of the Quarry: commit by path, `pearl ship` last,
  Coolify by uuid, republish the groves after the wipe until `/data`
  persists.
