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

## Routes

| | |
|---|---|
| `GET /` | index: motto, counts, kinds |
| `GET /models` | every entry, facts included |
| `GET /models/search?kind=column&h=5.2&tol=0.3&style=classical` | **ranked by fit** — for layout engines |
| `GET /models/<design>.json` · `.glb` · `.png` | the entry, the artifact, the preview |
| `POST /publish` | a submission (recipe + words) → derived, then stored |

## What an entry carries

- **design** — the hash of the recipe. Same recipe → same design, whoever asks.
- **recipe** — package, export, args, material. Re-derivable at any size or resolution.
- **artifact** — the derived `.glb`: PBR-complete (base colour, normal, metallic,
  roughness, occlusion), with its sha256 and triangle count.
- **facts** — *measured, never claimed*: bounding size, `origin: base|center`,
  facing, part count, materials, collider, and sockets (where things attach).
  A layout engine cannot use "a handsome weathered column"; it can use these.

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

`PORT` (3000) · `QUARRY_DATA` (./data) · `QUARRY_TOKEN` (optional publish gate).
An empty shelf seeds itself from the built-in `weft-model` library, so the
store is never empty on arrival.

Vendored crates are synced from the engine repo with `scripts/sync-vendor.sh`.
