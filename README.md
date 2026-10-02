# The Quarry — the Thread's model store

Where you go for stone that is already cut.

An asset store with one difference that changes what it can promise: **you
publish a recipe, not a file.** The Quarry runs the recipe itself — evaluates
the [Weft](https://github.com/Pixygon/thread-spec) program, carves the mesh,
bakes the PBR maps, writes the `.glb`, renders the preview — so an entry
cannot misrepresent its artifact. There is nothing to compare and nothing to
trust: the file *is* the program's output.

That also makes every model **re-derivable**: the CDN is an optimisation, not
a dependency. Lose the Quarry and every world rebuilds from recipes.

## The viewing room

It is also the room the models are **looked at** in. `quarry.pixygon.io` in a
browser is the one page every Chisel, Grove and Avatar model is opened,
turned, judged and made in:

- **Shelf** — search, a chip per supplier, a card per design.
- **Viewing table** — the actual `.glb` on a turntable you can turn, an
  LOD0/1/2 switch that really swaps the file the engine would swap, a **Wind**
  toggle that plays the wind the grower wrote into the vertex-colour alpha
  (1 = rigid, so carved stone stands still — and says so), and **Concept
  beside**, which splits the table and puts the Codex concept next to the
  mesh.
- **Ledger** — the measured facts, the recipe, the sockets, the provenance,
  and a **verdict** (*matches · close, fix noted · not it*) with a one-line
  note, kept **on the entry**, so a "not it" survives the browser that said it.
- **Make** — a door each for Chisel (a library shape, its args, a material,
  and whether to stand it on the ground; carved things get LOD1/LOD2 from
  the same recipe at coarser meshing)
  and Grove (a plant: its species in a form and as JSON, from blank or from
  anything on the shelf, plus the **seed**, the **clock** — age in seasons,
  season in the year — and whether it is **withered** — and something from
  the shelf to **hang** at its sockets: a lantern at every fruit socket. The
  placements are facts, the artifact stays the bare plant an importer
  instances onto, and the table instances them the same way). **Avatar**'s door
  takes a *manifested GLB* — the Portable Item Convention's own unit of
  exchange, one file that is the whole item — and does not derive it: it
  reads the file the way every consumer will, holds it to the convention
  rule by rule (schema-1 manifest, a real slot, textures embedded,
  metallic-roughness, a named skin for bodies and garments, metres, feet on
  the ground), measures it, and keeps the bytes untouched. The checklist is
  what the door shows, so an exporter learns exactly what is wrong. Both doors can **derive without publishing**, so you look before the
  shelf gains anything.

The page is rendered by this binary with every entry already in the HTML —
a crawler and an answer engine get the store's actual contents, and the
JavaScript only adds the parts a page of HTML cannot do.

## Routes

| | |
|---|---|
| `GET /` | the viewing room (HTML), or the index JSON for anything that did not ask for HTML |
| `GET /index.json` | index: motto, counts, kinds |
| `GET /m/<design>` | the viewing room, opened on one model |
| `GET /models` | every entry, facts included |
| `GET /models/search?kind=column&h=5.2&tol=0.3&style=classical` | **ranked by fit** — for layout engines |
| `GET /models/<design>.json` · `.glb` · `.png` | the entry, the artifact, the preview |
| `GET /library` | what the Make door may offer: the shapes with the arity the library itself declares, its materials, the grower's blank recipe |
| `POST /publish` | a submission (recipe + words) → derived, then stored. A body of `content-type: model/gltf-binary` is an avatar item: checked, measured, stored as it came; `?title=&tags=&style=&codex=` may add words the manifest lacks |
| `POST /derive` | the same, **kept off the shelf** → served from `/derived/<design>.glb`, swept as it fills |
| `POST /models/<design>/verdict` | `{verdict, note}` → kept on the entry |
| `POST /models/<design>/concept` | `{codex, concept}` → the Codex entity and the image to judge against |

Every `POST` is gated by `authorization: Bearer <QUARRY_TOKEN>` when that
variable is set: each one either costs the store real work or writes to an
entry.

## What an entry carries

- **design** — the hash of the recipe. Same recipe → same design, whoever asks.
  An avatar item has no recipe but the file, so its design is the hash of the
  bytes — the same file is the same design from anyone.
  The hash is over the recipe's *canonical* form: numbers rounded past f32
  noise, and — for a grown thing — every field the grower would have filled
  in by default stripped out, inside nested recipes too. A rule Grove gains
  tomorrow does not rename a tree nobody re-tuned.
- **recipe** — package, export, args, material. Re-derivable at any size or resolution.
- **artifact** — the derived `.glb`: PBR-complete (base colour, normal, metallic,
  roughness, occlusion), with its sha256 and triangle count.
- **facts** — *measured, never claimed*: bounding size, `origin: base|center`,
  facing, part count, materials, collider, and sockets (where things attach,
  typed: `tip`, `bloom`, `fruit`, `cut`). A grown thing also carries its
  **life** at this moment — stage, phase of the year, maturity, branch count.
  A layout engine cannot use "a handsome weathered column"; it can use these.
- **verdicts** — what people said when they looked at it beside its concept,
  oldest first. A judgement about a design belongs with the design.
- **codex / concept** — the Codex entity it answers to, and the gallery image
  to judge it against. The url is carried on the entry because most of the
  Codex is sealed and a browser arrives anonymous.

## Publish

```bash
curl -X POST https://quarry.pixygon.io/publish -H 'content-type: application/json' -d '{
  "title": "Doric column, 5.2 m", "kind": "column", "style": "classical",
  "tags": ["column","classical","load-bearing"],
  "export": "column", "args": [5.2, 0.44], "material": "marble" }'
```

or, from the Thread toolchain: `thread model --lib column --args '[5.2,0.44]'
--material marble --publish`.

## Running it

`PORT` (3000) · `QUARRY_DATA` (./data) · `QUARRY_TOKEN` (optional gate on
every write). An empty shelf seeds itself from the built-in `weft-model`
library, so the store is never empty on arrival.

Vendored crates are synced from the engine repo with `scripts/sync-vendor.sh`.

**Shipping.** The Quarry is a pearl of the crate type: `pearl ship` is the
last step (cargo test → changelog → the released version into `Cargo.toml` →
release on the API → commit and push). Publishing to crates.io is a separate
human step.

**Deploying.** Shipping is not deploying: a push does not redeploy, so
trigger Coolify by uuid. The store lives on persistent storage at `/data`
(since 2026-10-02) and survives a redeploy. On a fresh store the carved
starters reseed themselves; `scripts/republish-groves.sh` (with
`QUARRY_TOKEN` in the environment) puts back the grown ones, the lantern orb
and the lit tree — the design id is the hash of the recipe, so they come
back under the ids they had.
