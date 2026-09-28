#!/usr/bin/env bash
# Every Coolify redeploy wipes the store (no persistent mount at /data yet),
# so the grown things have to be put back. The chisel starters reseed
# themselves; the groves are recipes that live in thread-engine.
#
#   scripts/republish-groves.sh [quarry-url]
#
# QUARRY_TOKEN must be in the environment (from [publishing.quarry] in
# ~/.config/dyson-swarm/config.toml). It is never printed.
set -euo pipefail
URL="${1:-https://quarry.pixygon.io}"
RECIPES="${GROVE_RECIPES:-$HOME/repos/thread-engine/crates/grove/recipes}"
: "${QUARRY_TOKEN:?QUARRY_TOKEN is not in the environment}"

# The Lantern Desert entries answer to a Codex entity; the gallery image is
# public even though the entity is sealed, so the entry carries the url and
# the viewing room can put the concept beside the model for anyone.
LD_CONCEPT="https://pixygontech.b-cdn.net/system/textures/88ad7fc2-20b2-40ea-b408-7088b711f66a.jpg"

post() { # recipe title style tags codex concept
  python3 -c "
import json,sys
recipe=json.load(open(sys.argv[1]))
print(json.dumps({'title':sys.argv[2],'kind':'tree','style':sys.argv[3],
  'tags':[t for t in sys.argv[4].split(',') if t],'package':'grove','recipe':recipe,
  'origin':'grown','license':'CC0-1.0','codex':sys.argv[5],'concept':sys.argv[6]}))
" "$1" "$2" "$3" "$4" "$5" "$6" \
  | curl -sS -X POST "$URL/publish" -H 'content-type: application/json' \
      -H "authorization: Bearer $QUARRY_TOKEN" --data-binary @- \
  | python3 -c "
import json,sys
e=json.load(sys.stdin) if sys.stdin.isatty() is False else {}
print('  ', e.get('design','?'), e.get('title','?'), e.get('artifact',{}).get('tris','?'), 'tris,',
      len(e.get('facts',{}).get('sockets',[])), 'sockets') if 'design' in e else print('  REFUSED', e)
"
}

# A lantern to hang. Carved, so its id is the hash of its recipe and the lit
# tree below can name it before it exists on a fresh store.
echo "the lantern orb"
curl -sS -X POST "$URL/publish" -H 'content-type: application/json' -H "authorization: Bearer $QUARRY_TOKEN" \
  -d '{"title":"Lantern orb","kind":"prop","package":"weft-model","export":"sphere","args":[0.14],"material":"brass","rest":true,"tags":["lantern","prop"]}' \
  | python3 -c "import json,sys; e=json.load(sys.stdin); print('  ', e['design'], e['title'])"

post_lit() { # recipe title style tags codex concept hang-design
  python3 -c "
import json,sys
recipe=json.load(open(sys.argv[1]))
print(json.dumps({'title':sys.argv[2],'kind':'tree','style':sys.argv[3],
  'tags':[t for t in sys.argv[4].split(',') if t],'package':'grove','recipe':recipe,
  'origin':'grown','license':'CC0-1.0','codex':sys.argv[5],'concept':sys.argv[6],
  'hang':{'design':sys.argv[7],'kind':'fruit','count':0,'scale':1.0,'drop':0.12,'spin':True}}))
" "$1" "$2" "$3" "$4" "$5" "$6" "$7" \
  | curl -sS -X POST "$URL/publish" -H 'content-type: application/json' -H "authorization: Bearer $QUARRY_TOKEN" --data-binary @- \
  | python3 -c "
import json,sys
e=json.load(sys.stdin)
h=e.get('facts',{}).get('hung') or {}
print('  ', e.get('design','?'), e.get('title','?'), '· hung', h.get('count','?'), h.get('kind','')) if 'design' in e else print('  REFUSED', e)
"
}

echo "republishing the groves to $URL"
post "$RECIPES/oak.grow.json"           "Oak"                     "temperate"      "tree,oak,broadleaf,grown" "" ""
post "$RECIPES/lantern-tree.grow.json"  "lantern tree"            ""               "tree,grown" "lantern-desert" "$LD_CONCEPT"
post "$RECIPES/withered-tree.grow.json" "Withered lantern tree"   "lantern-desert" "tree,withered,aetherfall,lantern-desert,grown" "lantern-desert" "$LD_CONCEPT"
post_lit "$RECIPES/lantern-tree.grow.json" "lantern tree, lit"    "lantern-desert" "tree,lantern-desert,grown,lit" "lantern-desert" "$LD_CONCEPT" "039a0e8f8fcf7d51"
echo "done"
