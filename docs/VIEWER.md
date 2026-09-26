# The Quarry as the viewing room

Agreed with the founder 2026-09-26: the Quarry is where every Chisel, Grove
and Avatar model is opened, previewed and judged, and where new things are
made from any of the three systems. The founder approved the layout in
`viewer-mock.html` ("Yep, that is perfect"). Build that, for real, served by
this binary at quarry.pixygon.io.

## The layout (approved)

Three panes, dark, in the Pixygon palette (`@pixygon/design/tokens.mjs`:
void / panel / surface / edge / star-white, signal-cyan as the one accent;
suppliers get a hue each — chisel sandstone, grove leaf, avatar ember).

- **Shelf** (left): search; supplier chips All / Chisel / Grove / Avatar with
  counts; cards with a turntable thumbnail, title, kind, tris, sockets.
- **Viewing table** (centre): title and kind; **a live 3D viewer** of the glb
  (not the static turntable — `<model-viewer>` or three.js from a CDN is
  fine); LOD0/1/2 switch that really swaps the glb; **Wind** toggle that
  plays the vertex-channel wind (alpha = rigidity today; four channels when
  Grove ships them); **Concept beside**, which splits the table and pulls the
  concept from the Codex entity's gallery (`/v1/codex/entities`, the
  entry's `codex` field once it has one).
- **Ledger** (right): measured facts (size, origin, front, collider, parts,
  materials, tris, LODs, bytes); recipe (package, export/args or the grow
  rules, JSON on demand); sockets (count, kinds, first); **Verdict** —
  "Matches concept" / "Close, fix noted" / "Not it" plus a one-line note,
  **stored on the entry** (`verdicts` on the JSON), so a "Not it" can become
  a task later; provenance (design id, origin, license, author, sha256);
  actions: Download .glb, Copy design id, Republish, Open in Unity.

## Create (new, not in the mock)

A **Make** button on the shelf opens a maker with three doors:

- **Chisel** — pick a library export (the `weft-model` standard library),
  fill its args and material, derive, see it on the table, publish.
- **Grove** — start from a recipe on the shelf or a blank `GrowRecipe`, edit
  the rules in a form (with the JSON beside), a seed field, derive, see it,
  publish. The Grove session is adding species / seed / clock; leave room
  for an age and a season control.
- **Avatar** — nothing publishes here yet; show the door and say so.

Deriving is the existing `POST /publish`, gated by `QUARRY_TOKEN`; a
"derive without storing" route (`POST /derive` → glb + facts, nothing kept)
is the right addition so a person can look before they publish.

## Ground rules

- **Amended 2026-09-26 (founder):** this repo IS a pearl, of the **crate**
  type (`Dyson/templates/crate/README.md`). `pearl ship` is the last step —
  it runs `cargo test`, drafts the changelog, writes the released version
  into `Cargo.toml`, releases on the API, commits and pushes. Commit by path
  when another session shares the checkout. Publishing to crates.io stays a
  human step (`cargo publish -p …`) after the ship.
- Shipping is not deploying. The app does **not** redeploy on push — trigger
  Coolify (`GET <coolify>/api/v1/deploy?uuid=nvsfjj9yuxx2pkzwk4bcwp0v` with
  the token from `[publishing.coolify]` in `~/.config/dyson-swarm/config.toml`,
  read into the environment, never printed).
- **Every redeploy wipes the store** until the founder mounts persistent
  storage at `/data` in Coolify. After a deploy, republish the grove recipes
  from `~/repos/thread-engine/crates/grove/recipes` (`thread grow <r> --publish`,
  `QUARRY_TOKEN` from `[publishing.quarry]`).
- Vendored crates come from `~/repos/thread-engine` via
  `scripts/sync-vendor.sh`. `thread-chisel`, `weft-lang` and
  `thread-manifest` are on crates.io at 0.2.2; once `thread-grove` is
  published too, switch to crates.io dependencies and delete `vendor/`.
- Web rule: crawlers get real HTML. The page is server-rendered by this
  binary (entries baked into the HTML, JS enhances), no empty root div.
- Mock → founder's yes → implement → screenshot against the real build.
  The mock is the yes; screenshot the live page next to it before calling
  the layout done.
