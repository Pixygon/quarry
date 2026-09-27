# Changelog

All notable changes to **Quarry**. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this file is
materialized from the Pixygon Changelog API — edit there, not here.

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


