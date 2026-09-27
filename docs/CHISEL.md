# Chisel, seen from the table — findings for the engine session

2026-09-27. Every shape the Make door offers was derived at its defaults and
put on the turntable next to its measured facts (`scripts` are not needed:
`POST /derive` for each `/library` shape). What the Quarry could fix, it
fixed; what is Chisel's own is listed here, and the first one already has a
verdict on the live shelf.

## Fixed on the Quarry's side (0.8.0)

- **Carved things have LODs now.** LOD1 and LOD2 are built from the same
  recipe at half and a quarter of the material's meshing `resolution` and
  bake `size`. A sphere goes from 21 648 triangles to something a forest of
  them can afford; the LOD switch on the table really swaps them. The
  design id does not change — LODs are derived, not declared.
- **`rest`.** The library centres its primitives (cube, cylinder, sphere,
  capsule, cone, torus — and the bowl), so a placed one sits half
  underground. A submission may say `rest: true` and the carving is stood
  on the ground; it is part of the recipe (in the hash only when set, so no
  existing design moved). The door ticks it by default for the centred
  shapes.

## Chisel's own — engine repo, `crates/chisel` and `crates/weft/model_lib`

1. **`bowl(r, wall)` is not a bowl.** It comes out as a sphere with a
   shredded ring where the hollow should be: the cut that hollows it shares
   one grid with the shape, the same failure that shredded the 5 cm disc and
   the withered tree in the desert pass. *Stone bowl* (`d94d6155bb7bf4f7`)
   on the live shelf carries the verdict **not it** with this note. Either
   carve the hollow on its own grid, or lathe a profile the way `vase` does.
2. **`rock(r, lumpiness)` is a textured ball.** At lumpiness 3 the
   silhouette is a sphere; the "rock" is entirely the granite texture. A
   rock needs displacement in the SDF (noise on the radius), not on the
   normal map.
3. **Material scale lives in UV space, not metres.** Granite cells are 30 cm
   across on a 1 m cube and grain on a 10 cm one; the stairs look right by
   luck of size. `uv_scale` is per material and the default is 0.5. Either
   materials declare a physical scale (cells per metre) and the mesher sets
   `uv_scale` from the part's size, or the door has to ask a person for it.
4. **The vertex payload is the bytes.** Every vertex is written with
   position, normal, uv, tangent, colour and uv2 — 72 bytes — whether the
   part uses them or not; a 37 000-triangle bowl is 6 MB before its textures.
   Write tangents only with a normal map, colours only when not all white,
   uv2 only when something is in it. Half the store's bytes are this.
5. **`cone(0.5, 0, 1)` measures 0.97 m.** The SDF meshing shrinks a sharp
   apex by a cell; fine for a cone, a problem for anything whose height is a
   fact a layout engine will trust. Worth a note in the facts, or a
   resolution bump near sharp features.
6. **No sockets on carved things.** A column has a capital and a base; an
   arch has two feet and a keystone. Grove measures sockets at tips; the
   library's models could declare theirs (`sockets` in the export's return)
   and the Quarry would carry them as facts, and a layout engine could put a
   thing on a column instead of near it.

## What the door still lacks

- Composition. `at`, `join`, `ring_of`, `row_of`, `blend`, `cut` are the
  library's grammar and the door cannot say them. A "balustrade" is
  `ring_of(baluster, 12, 2.0)`; today that is a hand-written submission.
  A second tab in the Chisel door for a Weft expression, evaluated by the
  same `eval_model_or_part`, would make every combinator reachable.
- Materials with knobs. `tint`, `weathered`, `glow` take arguments; the
  door offers only the bare constructors.
