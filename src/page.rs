//! The viewing room, rendered by the binary that owns the models.
//!
//! The shelf, the ledger and every fact are written into the HTML here, so a
//! crawler, an answer engine or a browser with the network half-up gets the
//! Quarry's actual contents — titles, sizes, materials, triangle counts —
//! and not an empty root div with a promise. JavaScript then enhances: it
//! turns the static turntable into a live 3D viewer, swaps LODs, plays the
//! wind, and opens the Make door. Nothing it adds is needed to *read* the
//! store; everything it adds is needed to *judge* a model.

use crate::entry::Entry;

const CSS: &str = include_str!("web/app.css");
const JS: &str = include_str!("web/app.js");

pub fn css() -> &'static str {
    CSS
}
pub fn js() -> &'static str {
    JS
}

/// A short stamp that changes when the asset changes — enough to let the
/// CSS and JS be cached hard and still never be stale after a deploy.
pub fn version() -> String {
    let mut n: u64 = 1469598103934665603;
    for b in CSS.as_bytes().iter().chain(JS.as_bytes()) {
        n ^= *b as u64;
        n = n.wrapping_mul(1099511628211);
    }
    format!("{n:016x}")[..8].to_string()
}

pub fn render(entries: &[Entry], current: Option<&Entry>, locked: bool) -> String {
    let v = version();
    let cur = current.or_else(|| entries.first());
    let count = entries.len();
    let n = |s: &str| entries.iter().filter(|e| supplier(e) == s).count();
    let (n_chisel, n_grove, n_avatar) = (n("chisel"), n("grove"), n("avatar"));

    // The head describes the URL that was asked for, not whichever model the
    // table happens to open on: `/` is the store, `/m/<design>` is a model.
    // Getting this wrong points every canonical at the alphabetically first
    // entry, and the store disappears from the index behind an amphora.
    let title = match current {
        Some(e) => format!("{} — The Quarry", e.title),
        None => "The Quarry — the Thread's model store".to_string(),
    };
    let description = match current {
        Some(e) => format!(
            "{} — {}, {} triangles, {} m. One of {count} models in the Quarry, each derived by the store from its own recipe.",
            e.title, e.kind, thousands(e.artifact.tris), dims(e.facts.size)
        ),
        None => format!(
            "The Thread's model store: you publish a recipe, not a file, and the Quarry runs the recipe itself — so an entry cannot misrepresent its artifact. {count} models on the shelf, carved by Chisel and grown by Grove, each with measured facts: size, origin, collider, sockets."
        ),
    };
    let canonical = match current {
        Some(e) => format!("https://quarry.pixygon.io/m/{}", e.design),
        None => "https://quarry.pixygon.io/".to_string(),
    };
    let og_image = cur.map(|e| format!("https://quarry.pixygon.io{}", e.preview)).unwrap_or_default();

    let mut h = String::with_capacity(96 * 1024);
    h.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    h.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    h.push_str(&format!("<title>{}</title>\n", esc(&title)));
    h.push_str(&format!("<meta name=\"description\" content=\"{}\">\n", esc(&description)));
    h.push_str(&format!("<link rel=\"canonical\" href=\"{}\">\n", esc(&canonical)));
    h.push_str("<meta property=\"og:type\" content=\"website\">\n");
    h.push_str(&format!("<meta property=\"og:title\" content=\"{}\">\n", esc(&title)));
    h.push_str(&format!("<meta property=\"og:description\" content=\"{}\">\n", esc(&description)));
    h.push_str(&format!("<meta property=\"og:url\" content=\"{}\">\n", esc(&canonical)));
    if !og_image.is_empty() {
        h.push_str(&format!("<meta property=\"og:image\" content=\"{}\">\n", esc(&og_image)));
        h.push_str("<meta name=\"twitter:card\" content=\"summary_large_image\">\n");
    }
    h.push_str("<meta name=\"theme-color\" content=\"#0e0f12\">\n");
    h.push_str("<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>\n");
    h.push_str("<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,600&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Mono:wght@400;500&display=swap\">\n");
    h.push_str(&format!("<link rel=\"stylesheet\" href=\"/app.css?v={v}\">\n"));
    h.push_str(&format!("<script type=\"application/ld+json\">{}</script>\n", ld_json(entries)));
    h.push_str("</head>\n<body>\n");

    // ── header ──────────────────────────────────────────────────────────
    h.push_str("<header><h1>The Quarry</h1><span class=\"motto\">where you go for stone that is already cut</span>\n");
    h.push_str("<p class=\"lede\">The Thread's model store. You publish a recipe, not a file: the Quarry runs the recipe itself, so an entry cannot misrepresent its artifact.</p>\n");
    h.push_str("<div class=\"right\">");
    h.push_str(&format!(
        "<div class=\"counts\"><span><b id=\"c-all\">{count}</b> models</span><span><b id=\"c-chisel\">{n_chisel}</b> chisel</span><span><b id=\"c-grove\">{n_grove}</b> grove</span><span><b id=\"c-avatar\">{n_avatar}</b> avatar</span></div>"
    ));
    if locked {
        h.push_str("<span class=\"key locked\" id=\"key\" hidden>key <b>—</b></span>");
    }
    h.push_str("<button class=\"make\" id=\"make-open\">Make</button>");
    h.push_str("</div></header>\n");

    h.push_str("<main>\n");

    // ── shelf ───────────────────────────────────────────────────────────
    h.push_str("<aside class=\"shelf\" aria-label=\"Shelf\">\n <div class=\"tools\">\n");
    h.push_str("  <input id=\"q\" type=\"search\" placeholder=\"Search title, kind, tag…\" aria-label=\"Search the shelf\">\n");
    h.push_str("  <div class=\"chips\" id=\"sup\">\n");
    h.push_str(&format!("   <button class=\"chip\" data-sup=\"all\" aria-pressed=\"true\">All <span class=\"n\" id=\"n-all\">{count}</span></button>\n"));
    h.push_str(&format!("   <button class=\"chip\" data-sup=\"chisel\"><i style=\"background:var(--chisel)\"></i>Chisel <span class=\"n\" id=\"n-chisel\">{n_chisel}</span></button>\n"));
    h.push_str(&format!("   <button class=\"chip\" data-sup=\"grove\"><i style=\"background:var(--grove)\"></i>Grove <span class=\"n\" id=\"n-grove\">{n_grove}</span></button>\n"));
    let avatar_attr = if n_avatar == 0 { " disabled title=\"Avatar publishes here next\"" } else { "" };
    h.push_str(&format!("   <button class=\"chip\" data-sup=\"avatar\"{avatar_attr}><i style=\"background:var(--avatar)\"></i>Avatar <span class=\"n\" id=\"n-avatar\">{n_avatar}</span></button>\n"));
    h.push_str("  </div>\n </div>\n <div class=\"cards\" id=\"cards\" role=\"list\">\n");
    for e in entries {
        h.push_str(&card(e, cur.is_some_and(|c| c.design == e.design)));
    }
    if entries.is_empty() {
        h.push_str("  <p class=\"note\">The shelf is empty. Publish a recipe and it fills itself.</p>\n");
    }
    h.push_str(" </div>\n</aside>\n");

    // ── viewing table ───────────────────────────────────────────────────
    h.push_str("<section class=\"table\" aria-label=\"Viewing table\">\n");
    h.push_str(&format!(
        " <div class=\"bar\"><h2 id=\"t-title\">{}</h2><span class=\"kind\" id=\"t-kind\">{}</span>\n",
        esc(cur.map(|e| e.title.as_str()).unwrap_or("—")),
        esc(&cur.map(subtitle).unwrap_or_default())
    ));
    let lods = cur.map(|e| e.artifact.lods.len()).unwrap_or(0);
    h.push_str("  <div class=\"seg\" id=\"lod\" aria-label=\"Level of detail\">");
    for i in 0..3usize {
        let dis = if i > 0 && lods < i { " disabled" } else { "" };
        let pressed = i == 0;
        h.push_str(&format!("<button data-lod=\"{i}\" aria-pressed=\"{pressed}\"{dis}>LOD{i}</button>"));
    }
    h.push_str("</div>\n");
    h.push_str("  <div class=\"right\"><button id=\"judge\" aria-pressed=\"false\">Concept beside</button><button id=\"bones\" aria-pressed=\"false\" hidden>Bones</button><button id=\"wind\" aria-pressed=\"false\">Wind</button></div>\n </div>\n");

    h.push_str(" <div class=\"stage\" id=\"stage\"><div class=\"pane\" id=\"vpane\">");
    if let Some(e) = cur {
        h.push_str(&format!(
            "<img class=\"still\" id=\"turn\" src=\"{}\" alt=\"{} — three-view turntable\" width=\"1152\" height=\"384\">",
            esc(&e.preview), esc(&e.title)
        ));
    }
    h.push_str("<span class=\"cap\" id=\"cap\">turntable · lod0</span><span class=\"hint\">drag to turn · scroll to zoom</span></div>");
    h.push_str("<div class=\"pane cpane\" id=\"cpane\" hidden></div></div>\n");
    h.push_str(&format!(" <div class=\"foot\" id=\"foot\">{}</div>\n</section>\n", cur.map(foot).unwrap_or_default()));

    // ── ledger ──────────────────────────────────────────────────────────
    h.push_str("<aside class=\"ledger\" aria-label=\"Ledger\">\n");
    h.push_str(&format!(" <section><h3>Measured, never claimed</h3><dl id=\"facts\">{}</dl></section>\n", cur.map(facts_dl).unwrap_or_default()));
    h.push_str(&format!(
        " <section><h3>Recipe</h3><dl id=\"recipe\">{}</dl><details><summary>Show the recipe</summary><pre id=\"recipe-json\">{}</pre></details></section>\n",
        cur.map(recipe_dl).unwrap_or_default(),
        esc(&cur.map(recipe_json).unwrap_or_default())
    ));
    h.push_str(&format!(" <section><h3>Sockets</h3><dl id=\"sockets\">{}</dl></section>\n", cur.map(sockets_dl).unwrap_or_default()));
    let (life_hidden, life_dl) = match cur.and_then(|e| e.facts.life.as_ref()) {
        Some(l) => ("", life_dl(l, cur.unwrap())),
        None => (" hidden", String::new()),
    };
    h.push_str(&format!(" <section id=\"life\"{life_hidden}><h3>This moment</h3><dl id=\"life-dl\">{life_dl}</dl></section>\n"));
    let (item_hidden, item_dl, checks) = match cur.and_then(|e| e.facts.item.as_ref()) {
        Some(i) => ("", item_dl(i), checks_ul(i)),
        None => (" hidden", String::new(), String::new()),
    };
    h.push_str(&format!(" <section id=\"item\"{item_hidden}><h3>The item</h3><dl id=\"item-dl\">{item_dl}</dl><ul class=\"checks\" id=\"checks\">{checks}</ul></section>\n"));
    h.push_str(" <section><h3>Verdict</h3><div class=\"verdict\" id=\"verdict\"><button data-v=\"yes\">Matches concept</button><button data-v=\"close\">Close, fix noted</button><button data-v=\"no\">Not it</button></div>");
    h.push_str("<textarea id=\"note\" placeholder=\"What is off, in one line. Saved with the design.\"></textarea>");
    h.push_str(&format!("<div class=\"verdict-log\" id=\"verdict-log\">{}</div></section>\n", cur.map(verdict_log).unwrap_or_default()));
    h.push_str(&format!(" <section><h3>Provenance</h3><dl id=\"prov\">{}</dl></section>\n", cur.map(prov_dl).unwrap_or_default()));
    h.push_str(" <section><h3>Actions</h3><div class=\"actions\"><a class=\"primary\" id=\"a-glb\" download href=\"");
    h.push_str(&esc(cur.map(|e| e.artifact.url.as_str()).unwrap_or("#")));
    h.push_str("\">Download .glb</a>");
    h.push_str("<button id=\"a-id\">Copy design id</button><button id=\"a-again\">Republish</button><button id=\"a-unity\">Open in Unity</button></div></section>\n");
    h.push_str("</aside>\n</main>\n");

    h.push_str(&maker());

    // ── the data the enhancement reads: the same entries, once ──────────
    h.push_str("<script type=\"application/json\" id=\"models\">");
    h.push_str(&in_script(&serde_json::to_value(entries).unwrap_or_else(|_| serde_json::json!([]))));
    h.push_str("</script>\n");
    h.push_str(&format!(
        "<script type=\"application/json\" id=\"boot\">{{\"current\":{},\"gated\":{locked}}}</script>\n",
        serde_json::to_string(&cur.map(|e| e.design.clone())).unwrap_or_else(|_| "null".into())
    ));
    h.push_str("<script type=\"importmap\">{\"imports\":{\"three\":\"https://cdn.jsdelivr.net/npm/three@0.169.0/build/three.module.js\",\"three/addons/\":\"https://cdn.jsdelivr.net/npm/three@0.169.0/examples/jsm/\"}}</script>\n");
    h.push_str(&format!("<script type=\"module\" src=\"/app.js?v={v}\"></script>\n"));
    h.push_str("</body>\n</html>\n");
    h
}

fn supplier(e: &Entry) -> &str {
    if e.supplier.is_empty() {
        crate::entry::supplier_of(&e.recipe.package)
    } else {
        &e.supplier
    }
}

fn card(e: &Entry, current: bool) -> String {
    format!(
        "  <a class=\"card\" role=\"listitem\" href=\"/m/{d}\" data-d=\"{d}\" aria-current=\"{current}\"><span class=\"thumb\"><img src=\"{png}\" alt=\"\" loading=\"lazy\" width=\"192\" height=\"64\"></span><span><span class=\"t\"><i class=\"sup {sup}\"></i><span>{title}</span></span><span class=\"m\">{kind} · {tris} tris · {sockets} sockets</span></span></a>\n",
        d = esc(&e.design),
        png = esc(&e.preview),
        sup = supplier(e),
        title = esc(&e.title),
        kind = esc(&e.kind),
        tris = thousands(e.artifact.tris),
        sockets = e.facts.sockets.len(),
    )
}

fn subtitle(e: &Entry) -> String {
    let mut s = e.kind.clone();
    if !e.style.is_empty() {
        s.push_str(" · ");
        s.push_str(&e.style);
    }
    s.push_str(" · ");
    s.push_str(supplier(e));
    s
}

fn foot(e: &Entry) -> String {
    let lods = if e.artifact.lods.is_empty() {
        "no coarser lods".to_string()
    } else {
        e.artifact
            .lods
            .iter()
            .enumerate()
            .map(|(i, u)| format!("lod{} <b>{}</b>", i + 1, esc(u.rsplit('/').next().unwrap_or(u))))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    format!(
        "<span>design <b>{}</b></span><span>artifact <b>{}</b></span><span>{lods}</span>",
        esc(&e.design),
        esc(&e.artifact.url)
    )
}

fn facts_dl(e: &Entry) -> String {
    let f = &e.facts;
    format!(
        "<dt>Size</dt><dd>{} m</dd><dt>Origin</dt><dd>{}</dd><dt>Front</dt><dd>{}</dd><dt>Collider</dt><dd>{}</dd><dt>Parts</dt><dd>{} · {}</dd><dt>Triangles</dt><dd>{}</dd><dt>LODs</dt><dd>{}</dd><dt>Bytes</dt><dd>{}</dd>",
        dims(f.size),
        esc(&f.origin),
        esc(&f.front),
        esc(&f.collider),
        f.parts,
        esc(&f.materials.join(", ")),
        thousands(e.artifact.tris),
        if e.artifact.lods.is_empty() { "none".to_string() } else { format!("{} coarser", e.artifact.lods.len()) },
        thousands(e.artifact.bytes),
    )
}

fn recipe_dl(e: &Entry) -> String {
    let r = &e.recipe;
    if r.package == "grove" {
        let rules = r.recipe.as_ref().and_then(|v| v.as_object()).map(|o| o.len()).unwrap_or(0);
        let seed = r
            .recipe
            .as_ref()
            .and_then(|v| v.get("seed"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "default".into());
        format!("<dt>Package</dt><dd>grove</dd><dt>Grown from</dt><dd>{rules} rules · seed {}</dd>", esc(&seed))
    } else {
        let args = r.args.iter().map(num).collect::<Vec<_>>().join(", ");
        format!(
            "<dt>Package</dt><dd>{}</dd><dt>Export</dt><dd>{}({})</dd><dt>Material</dt><dd>{}</dd>",
            esc(&r.package),
            esc(&r.export),
            esc(&args),
            esc(if r.material.is_empty() { "its own" } else { &r.material })
        )
    }
}

fn recipe_json(e: &Entry) -> String {
    let v = e.recipe.recipe.clone().unwrap_or_else(|| {
        serde_json::json!({ "export": e.recipe.export, "args": e.recipe.args, "material": e.recipe.material })
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

fn item_dl(i: &crate::entry::Item) -> String {
    format!(
        "<dt>Kind</dt><dd>{} · {}</dd><dt>partId</dt><dd>{}</dd><dt>Attach</dt><dd>{}{}</dd><dt>Textures</dt><dd>{}</dd><dt>Convention</dt><dd>{}</dd>",
        esc(&i.kind),
        esc(&i.slot),
        i.id,
        esc(&i.attach),
        if i.bones.is_empty() { String::new() } else { format!(" · {} joints", i.bones.len()) },
        i.textures,
        if i.failed == 0 { "adheres".to_string() } else { format!("{} of {} rules failed", i.failed, i.checks.len()) }
    )
}

/// Every rule, ticked or crossed — the reason this door exists.
fn checks_ul(i: &crate::entry::Item) -> String {
    i.checks
        .iter()
        .map(|c| format!("<li class=\"{}\"><b>{}</b> {}</li>", if c.ok { "ok" } else { "bad" }, esc(&c.rule), esc(&c.note)))
        .collect()
}

fn life_dl(l: &crate::entry::Life, e: &Entry) -> String {
    let withered = e.recipe.recipe.as_ref().and_then(|r| r.get("withered")).and_then(|v| v.as_bool()).unwrap_or(false);
    format!(
        "<dt>Stage</dt><dd>{}</dd><dt>Year</dt><dd>{}</dd><dt>Maturity</dt><dd>{:.0} %</dd><dt>Branches</dt><dd>{}</dd>{}",
        esc(&l.stage),
        esc(&l.phase),
        l.maturity * 100.0,
        thousands(l.branches),
        if withered { "<dt>State</dt><dd>withered</dd>" } else { "" }
    )
}

fn sockets_dl(e: &Entry) -> String {
    let s = &e.facts.sockets;
    if s.is_empty() {
        return "<dt>Count</dt><dd>none — nothing attaches here</dd>".into();
    }
    // Sockets by kind — "124 tip · 28 fruit" says what can hang here, which
    // is what a socket is for.
    let mut by_kind: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for x in s {
        *by_kind.entry(x.kind.as_str()).or_default() += 1;
    }
    let kinds: Vec<String> = by_kind.iter().map(|(k, n)| format!("{n} {k}")).collect();
    let first = &s[0];
    format!(
        "<dt>Count</dt><dd>{}</dd><dt>Kinds</dt><dd>{}</dd><dt>First</dt><dd>{} @ {}</dd>",
        s.len(),
        esc(&kinds.join(" · ")),
        esc(&first.name),
        first.at.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(", ")
    )
}

fn verdict_log(e: &Entry) -> String {
    e.verdicts
        .iter()
        .rev()
        .take(4)
        .map(|v| {
            format!(
                "<span class=\"{}\"><b>{}</b>{}</span>",
                esc(&v.verdict),
                esc(word(&v.verdict)),
                if v.note.is_empty() { String::new() } else { format!(" — {}", esc(&v.note)) }
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn word(v: &str) -> &str {
    match v {
        "yes" => "matches",
        "close" => "close",
        "no" => "not it",
        other => other,
    }
}

fn prov_dl(e: &Entry) -> String {
    let codex = if e.codex.is_empty() { "—".to_string() } else { esc(&e.codex) };
    format!(
        "<dt>Design</dt><dd class=\"id\">{}</dd><dt>Origin</dt><dd>{}</dd><dt>Codex</dt><dd>{codex}</dd><dt>License</dt><dd>{}</dd><dt>Author</dt><dd>{}</dd><dt>sha256</dt><dd class=\"id\">{}…</dd>",
        esc(&e.design),
        esc(&e.origin),
        esc(&e.license),
        esc(if e.author.is_empty() { "—" } else { &e.author }),
        esc(&e.artifact.sha256.chars().take(20).collect::<String>())
    )
}

/// The Make door: three doors, one of which is honest about not being open.
fn maker() -> String {
    r##"<dialog class="maker" id="maker" aria-label="Make a model">
 <div class="head"><h2>Make</h2><span class="kind">the recipe is the thing — derive it, look at it, then publish</span><button class="x" id="make-close">Close</button></div>
 <div class="doors" id="doors">
  <button data-door="chisel" aria-pressed="true"><i style="background:var(--chisel)"></i>Chisel</button>
  <button data-door="grove"><i style="background:var(--grove)"></i>Grove</button>
  <button data-door="avatar"><i style="background:var(--avatar)"></i>Avatar</button>
 </div>
 <div class="door" id="door-chisel">
  <div class="shapes" id="shapes"></div>
  <div class="grid" id="chisel-args"></div>
  <h4>Words</h4>
  <div class="grid">
   <div class="field"><label for="c-title">Title</label><input id="c-title" placeholder="Doric column, 5.2 m"></div>
   <div class="field"><label for="c-kind">Kind</label><input id="c-kind" placeholder="column"></div>
   <div class="field"><label for="c-style">Style</label><input id="c-style" placeholder="classical"></div>
   <div class="field"><label for="c-material">Material</label><select id="c-material"></select></div>
   <div class="field"><label for="c-tags">Tags</label><input id="c-tags" placeholder="column, classical"></div>
   <div class="field"><label for="c-codex">Codex slug</label><input id="c-codex" placeholder="lantern-desert"></div>
  </div>
 </div>
 <div class="door" id="door-grove" hidden>
  <div class="grid">
   <div class="field"><label for="g-from">Start from</label><select id="g-from"></select></div>
   <div class="field"><label for="g-seed">Seed</label><input id="g-seed" type="number" step="1" min="0"></div>
   <div class="field"><label for="g-age">Age, seasons</label><input id="g-age" type="number" step="1" min="0" placeholder="blank = grown"></div>
   <div class="field"><label for="g-season">Season</label><select id="g-season"></select></div>
   <div class="field"><label for="g-withered">Withered</label><input id="g-withered" type="checkbox"></div>
  </div>
  <div class="two">
   <div><h4>Rules</h4><div class="grid" id="grove-args"></div></div>
   <div><h4>The recipe itself</h4><textarea class="json" id="g-json" spellcheck="false"></textarea><p class="note">The form writes into this; edit it directly for bark, leaves and colour. The form wins on the next field you touch.</p></div>
  </div>
  <h4>Words</h4>
  <div class="grid">
   <div class="field"><label for="g-title">Title</label><input id="g-title" placeholder="Oak"></div>
   <div class="field"><label for="g-style">Style</label><input id="g-style" placeholder="temperate"></div>
   <div class="field"><label for="g-tags">Tags</label><input id="g-tags" placeholder="tree, oak, broadleaf"></div>
   <div class="field"><label for="g-codex">Codex slug</label><input id="g-codex" placeholder="lantern-desert"></div>
  </div>
 </div>
 <div class="door" id="door-avatar" hidden>
  <p class="note" style="margin:0 0 10px">An avatar part is not derived here — it is made in Unity, or anywhere, and arrives as <b>one GLB that is the whole item</b>: mesh, materials, textures, and its meaning at <code>asset.extras.pixygonItem</code>. The Quarry reads it the way every consumer will, holds it to the Portable Item Convention, measures it, and keeps the bytes exactly as they came.</p>
  <label class="drop" for="a-file"><input id="a-file" type="file" accept=".glb,model/gltf-binary"><span id="a-drop-text">Drop a manifested .glb here, or choose one</span></label>
  <div class="grid" style="margin-top:12px">
   <div class="field"><label for="a-title">Title <span style="text-transform:none;letter-spacing:0">(blank = the manifest's)</span></label><input id="a-title"></div>
   <div class="field"><label for="a-style">Style</label><input id="a-style" placeholder="lantern-desert"></div>
   <div class="field"><label for="a-tags">Tags</label><input id="a-tags" placeholder="cloak, veilwalkers"></div>
   <div class="field"><label for="a-codex">Codex slug <span style="text-transform:none;letter-spacing:0">(blank = the manifest's)</span></label><input id="a-codex"></div>
  </div>
  <ul class="checks" id="a-checks"></ul>
 </div>
 <div class="foot">
  <button id="m-derive">Derive &amp; look</button>
  <button id="m-publish" class="primary">Publish to the shelf</button>
  <span class="msg" id="m-msg"></span>
 </div>
</dialog>
"##
    .to_string()
}

/// Structured data for the answer engines: the store as a list of 3D models,
/// each with the facts the Quarry measured.
fn ld_json(entries: &[Entry]) -> String {
    let items: Vec<serde_json::Value> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            serde_json::json!({
                "@type": "ListItem",
                "position": i + 1,
                "item": {
                    "@type": "3DModel",
                    "name": e.title,
                    "description": if e.description.is_empty() {
                        format!("{}, {} — derived by the Quarry from its recipe.", e.kind, dims(e.facts.size))
                    } else { e.description.clone() },
                    "url": format!("https://quarry.pixygon.io/m/{}", e.design),
                    "contentUrl": format!("https://quarry.pixygon.io{}", e.artifact.url),
                    "encodingFormat": "model/gltf-binary",
                    "image": format!("https://quarry.pixygon.io{}", e.preview),
                    "license": e.license,
                    "keywords": e.tags.join(", "),
                }
            })
        })
        .collect();
    let doc = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "CollectionPage",
        "name": "The Quarry",
        "url": "https://quarry.pixygon.io/",
        "description": "The Thread's model store: publish a recipe, the store derives the artifact.",
        "mainEntity": { "@type": "ItemList", "numberOfItems": entries.len(), "itemListElement": items },
    });
    in_script(&doc)
}

/// JSON going inside a `<script>` element. Escaping only `</` is the usual
/// advice and it is not enough: `<!--<script` puts the tokenizer into the
/// script-data-double-escaped state, where the element's own closing tag
/// stops closing it and the rest of the document is swallowed. Escaping
/// every `<` as `\u003c` costs nothing, parses identically, and leaves no
/// sequence HTML can react to.
fn in_script(v: &serde_json::Value) -> String {
    v.to_string().replace('<', "\\u003c")
}

fn dims(s: [f32; 3]) -> String {
    s.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" × ")
}

fn num(v: &serde_json::Value) -> String {
    match v.as_f64() {
        Some(f) if (f - f.round()).abs() < 1e-6 => format!("{}", f.round() as i64),
        Some(f) => format!("{}", (f * 10_000.0).round() / 10_000.0),
        None => v.to_string(),
    }
}

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Artifact, Facts, Recipe, Socket, Verdict};

    fn entry(design: &str, title: &str) -> Entry {
        Entry {
            design: design.into(),
            title: title.into(),
            description: String::new(),
            tags: vec!["column".into(), "classical".into()],
            kind: "column".into(),
            style: "classical".into(),
            recipe: Recipe {
                package: "weft-model".into(),
                export: "column".into(),
                args: vec![serde_json::json!(5.2), serde_json::json!(0.44)],
                material: "marble".into(),
                recipe: None,
            },
            artifact: Artifact {
                url: format!("/models/{design}.glb"),
                bytes: 460_860,
                tris: 2504,
                sha256: "d0bf8301c4b48aa18a7b48946403ee1f691cc831a2bbe7c169adedd850570495".into(),
                lods: Vec::new(),
            },
            facts: Facts {
                size: [0.91, 5.2, 0.91],
                origin: "base".into(),
                front: "+z".into(),
                parts: 1,
                materials: vec!["marble".into()],
                collider: "cylinder".into(),
                sockets: vec![Socket { name: "cap".into(), at: [0.0, 5.2, 0.0], kind: "capital".into(), size: [0.9; 3] }],
                life: None,
                item: None,
            },
            preview: format!("/models/{design}.png"),
            license: "CC0-1.0".into(),
            author: "did:pixygon:quarry".into(),
            origin: "seed".into(),
            supplier: "chisel".into(),
            codex: String::new(),
            concept: String::new(),
            verdicts: Vec::new(),
            derived_at: 0,
        }
    }

    /// The whole point of rendering here: a crawler, an answer engine or a
    /// browser with JavaScript off gets the store's CONTENTS. If this ever
    /// passes only because the JSON blob is in the page, it has failed.
    #[test]
    fn a_crawler_gets_the_contents_not_a_div() {
        let entries = vec![entry("5e41fec7b0ce3898", "Doric column, 5.2 m")];
        let html = render(&entries, None, false);
        let markup = html.split("<script type=\"application/json\"").next().unwrap();
        assert!(markup.contains("Doric column, 5.2 m"), "the title is in the markup");
        assert!(markup.contains("href=\"/m/5e41fec7b0ce3898\""), "the card is a real link");
        assert!(markup.contains("0.91 × 5.20 × 0.91 m"), "the measured size is in the markup");
        assert!(markup.contains("cylinder"), "the collider is in the markup");
        assert!(markup.contains("2,504"), "the triangle count is in the markup");
        assert!(markup.contains("column(5.2, 0.44)"), "the recipe is in the markup");
        assert!(markup.contains("capital"), "the socket kinds are in the markup");
    }

    /// The head describes the url that was asked for. Pointing every
    /// canonical at the alphabetically first entry hides the store behind an
    /// amphora.
    #[test]
    fn the_head_describes_the_url_that_was_asked_for() {
        let entries = vec![entry("aaa1", "Amphora"), entry("bbb2", "Column")];
        let index = render(&entries, None, false);
        assert!(index.contains("<link rel=\"canonical\" href=\"https://quarry.pixygon.io/\">"));
        assert!(index.contains("<title>The Quarry — the Thread&#39;s model store</title>"));

        let one = render(&entries, entries.get(1), false);
        assert!(one.contains("<link rel=\"canonical\" href=\"https://quarry.pixygon.io/m/bbb2\">"));
        assert!(one.contains("<title>Column — The Quarry</title>"));
        assert!(one.contains("og:image\" content=\"https://quarry.pixygon.io/models/bbb2.png"));
    }

    /// A title is words a publisher sent. They are escaped everywhere, and
    /// the entries blob cannot close the script tag it lives in.
    #[test]
    fn a_publishers_words_cannot_break_out() {
        let mut e = entry("c0ffee", "</script><img src=x onerror=alert(1)>");
        e.description = "\"quoted\" & <angled>".into();
        let html = render(&[e], None, false);
        let markup = html.split("<script type=\"application/json\"").next().unwrap();
        assert!(!markup.contains("<img src=x"), "unescaped markup reached the page");
        assert!(!html.contains("</script><img"), "the json blob closed its own tag");
        // Nothing HTML reacts to survives inside a script element, and the
        // blob is still valid JSON: the parser undoes \u003c for us.
        for id in ["application/ld+json\">", "application/json\" id=\"models\">"] {
            let blob = html.split(id).nth(1).unwrap().split("</script>").next().unwrap();
            assert!(!blob.contains('<'), "a raw < survived inside a script element");
        }
        let blob = html
            .split("<script type=\"application/json\" id=\"models\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let back: Vec<Entry> = serde_json::from_str(blob).expect("the blob round-trips");
        assert_eq!(back[0].title, "</script><img src=x onerror=alert(1)>");
    }

    /// Verdicts are rendered for a reader, not only for the script.
    #[test]
    fn a_verdict_is_on_the_page() {
        let mut e = entry("d00d", "Column");
        e.verdicts.push(Verdict { verdict: "no".into(), note: "too slender".into(), by: String::new(), at: 1 });
        let html = render(&[e.clone()], Some(&e), false);
        assert!(html.contains("not it"));
        assert!(html.contains("too slender"));
    }

    /// Cache-busting has to actually change when the asset does.
    #[test]
    fn the_asset_version_is_eight_hex_digits() {
        let v = version();
        assert_eq!(v.len(), 8, "{v}");
        assert!(v.chars().all(|c| c.is_ascii_hexdigit()), "{v}");
    }

    #[test]
    fn an_empty_shelf_says_so() {
        let html = render(&[], None, false);
        assert!(html.contains("The shelf is empty"));
        assert!(html.contains("<title>The Quarry"));
    }
}
