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
- **Make** — a door each for Chisel (a library shape, its args, a material)
  and Grove (a grow recipe, in a form and as JSON, from blank or from
  anything on the shelf). Avatar's door is shown and is honest about being
  shut. Both doors can **derive without publishing**, so you look before the
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
| `POST /publish` | a submission (recipe + words) → derived, then stored |
| `POST /derive` | the same, **kept off the shelf** → served from `/derived/<design>.glb`, swept as it fills |
| `POST /models/<design>/verdict` | `{verdict, note}` → kept on the entry |
| `POST /models/<design>/concept` | `{codex, concept}` → the Codex entity and the image to judge against |

Every `POST` is gated by `authorization: Bearer <QUARRY_TOKEN>` when that
variable is set: each one either costs the store real work or writes to an
entry.

## What an entry carries

- **design** — the hash of the recipe. Same recipe → same design, whoever asks.
- **recipe** — package, export, args, material. Re-derivable at any size or resolution.
- **artifact** — the derived `.glb`: PBR-complete (base colour, normal, metallic,
  roughness, occlusion), with its sha256 and triangle count.
- **facts** — *measured, never claimed*: bounding size, `origin: base|center`,
  facing, part count, materials, collider, and sockets (where things attach).
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
trigger Coolify by uuid. And until a
persistent volume is mounted at `/data`, **every redeploy wipes the store** —
the carved starters reseed themselves, the grown ones do not, so run
`scripts/republish-groves.sh` (with `QUARRY_TOKEN` in the environment) after
each deploy. The design id is the hash of the recipe, so they come back under
the ids they had.
