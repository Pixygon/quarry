#!/usr/bin/env bash
# Vendor the four crates the Quarry needs to DERIVE a model from its recipe:
# the id scheme, the manifest/model format, the Weft language (with its
# modeling library), and chisel (carve, bake, export, preview). They are
# copied rather than depended on because Pixygon/Infinite is private — the
# same reason wpm vendors weft. Re-run after changing any of them.
set -euo pipefail
SRC="${1:-$HOME/repos/Infinite}"
DST="$(cd "$(dirname "$0")/.." && pwd)/vendor"
for c in thread-id infinite-manifest weft chisel; do
  rm -rf "$DST/$c"
  mkdir -p "$DST/$c"
  cp -r "$SRC/crates/$c/src" "$DST/$c/src"
  cp "$SRC/crates/$c/Cargo.toml" "$DST/$c/Cargo.toml"
done
# Workspace-inherited fields and path deps must become concrete.
python3 - "$DST" <<'PY'
import re, sys, pathlib
dst = pathlib.Path(sys.argv[1])
versions = {
    "serde": '{ version = "1", features = ["derive"] }',
    "serde_json": '"1"',
    "sha2": '"0.10"',
    "image": '{ version = "0.25", default-features = false, features = ["png"] }',
}
paths = {
    "thread-id": '{ path = "../thread-id" }',
    "infinite-manifest": '{ path = "../infinite-manifest" }',
    "weft": '{ path = "../weft" }',
    "chisel": '{ path = "../chisel" }',
}
for crate in ["thread-id", "infinite-manifest", "weft", "chisel"]:
    p = dst / crate / "Cargo.toml"
    out, in_dev = [], False
    for line in p.read_text().splitlines():
        s = line.strip()
        if s.startswith("["):
            in_dev = s.startswith("[dev-dependencies]")
        if in_dev and not s.startswith("["):
            continue                      # tests don't ship
        if "workspace = true" in line or "path = " in line:
            name = s.split("=")[0].split(".")[0].strip()
            if name in versions:
                out.append(f"{name} = {versions[name]}")
                continue
            if name in paths:
                out.append(f"{name} = {paths[name]}")
                continue
            for field in ["version", "edition", "authors", "license", "repository", "description"]:
                if s.startswith(field):
                    fixed = {"version": '"0.1.0"', "edition": '"2021"',
                             "authors": '["Pixygon"]', "license": '"MIT OR Apache-2.0"',
                             "repository": '"https://github.com/Pixygon/quarry"',
                             "description": '"vendored for the Quarry"'}[field]
                    out.append(f"{field} = {fixed}")
                    break
            else:
                continue
            continue
        out.append(line)
    p.write_text("\n".join(out) + "\n")
    # Vendored copies carry no tests (their home repo runs those).
    for f in (dst / crate / "src").rglob("*.rs"):
        t = f.read_text()
        i = t.find("#[cfg(test)]")
        if i > 0:
            f.write_text(t[:i].rstrip() + "\n")
# Two crates are published under neutral names; keep the aliases the
# workspace uses so `use infinite_manifest::…` / `use thread_id::…` compile.
for crate, old, new in [
    ("chisel", 'infinite-manifest = { path = "../infinite-manifest" }',
     'infinite-manifest = { package = "thread-manifest", path = "../infinite-manifest" }'),
    ("infinite-manifest", 'thread-id = { path = "../thread-id" }',
     'thread-id = { package = "thread-structured-id", path = "../thread-id" }'),
]:
    f = dst / crate / "Cargo.toml"
    f.write_text(f.read_text().replace(old, new))
print("vendored:", ", ".join(sorted(p.name for p in dst.iterdir())))
PY
