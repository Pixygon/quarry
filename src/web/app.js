// The Quarry's viewing room — the enhancement layer.
//
// Everything a person needs to READ the store is already in the HTML the
// binary sent. This file adds the part a page of HTML cannot do: it puts the
// actual .glb on a turntable you can turn, swaps the LOD the engine would
// swap, plays the wind out of the vertex channel the grower wrote it into,
// pulls the concept up beside it, and opens the door to make a new one.
//
// Wind is the grower's convention and not an invention of this page: vertex
// colour alpha is RIGIDITY (1 = rigid, the root; lower = sways, the tips), so
// a model with no alpha channel — everything chisel carves — stands still,
// which is correct.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';

const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];
const json = (id) => { try { return JSON.parse(document.getElementById(id).textContent); } catch { return null; } };

let MODELS = json('models') || [];
const BOOT = json('boot') || {};
const GATED = !!BOOT.gated;

const state = {
  sup: 'all',
  q: '',
  design: BOOT.current || (MODELS[0] && MODELS[0].design) || null,
  lod: 0,
  wind: false,
  judge: false,
  preview: null,   // an unshelved derive sitting on the table
  bones: false,
};

const fmt = (n) => Number(n).toLocaleString('en-US');
const dims = (s) => s.map((v) => v.toFixed(2)).join(' × ');
const supplierOf = (m) => m.supplier || (m.recipe.package === 'grove' ? 'grove' : m.recipe.package === 'avatar' ? 'avatar' : 'chisel');
const current = () => state.preview || MODELS.find((m) => m.design === state.design) || MODELS[0] || null;

// ── the key ────────────────────────────────────────────────────────────────
// Publishing costs the store real work, so it is gated. The key lives in this
// browser and is sent as a bearer token; it is never in the page the server
// renders, and "Forget" means forget.

const KEY = 'quarry.key';
const key = () => { try { return localStorage.getItem(KEY) || ''; } catch { return ''; } };
function setKey(v) {
  try { v ? localStorage.setItem(KEY, v) : localStorage.removeItem(KEY); } catch { /* private window */ }
  paintKey();
}
function paintKey() {
  const el = $('#key');
  if (!el) return;
  el.hidden = false;
  const has = !!key();
  el.classList.toggle('locked', !has);
  el.innerHTML = has ? 'key <b>held</b> <button id="key-x">Forget</button>' : '<button id="key-in">Unlock to publish</button>';
  const x = $('#key-x'), i = $('#key-in');
  if (x) x.onclick = () => setKey('');
  if (i) i.onclick = askKey;
}
function askKey() {
  const v = prompt('Publish key (QUARRY_TOKEN). Stored in this browser only.');
  if (v !== null) setKey(v.trim());
  return !!key();
}

async function send(path, body, method = 'POST') {
  const headers = { 'content-type': 'application/json' };
  if (key()) headers.authorization = 'Bearer ' + key();
  const r = await fetch(path, { method, headers, body: JSON.stringify(body) });
  const text = await r.text();
  if (!r.ok) {
    if (r.status === 401) throw new Error('the store refused the key — unlock again');
    throw new Error(text.slice(0, 300) || ('HTTP ' + r.status));
  }
  try { return JSON.parse(text); } catch { return {}; }
}

function toast(msg, bad) {
  const t = document.createElement('div');
  t.className = 'toast' + (bad ? ' err' : '');
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), bad ? 6000 : 2600);
}

// ── shelf ──────────────────────────────────────────────────────────────────

function shelf() {
  const list = MODELS.filter((m) =>
    (state.sup === 'all' || supplierOf(m) === state.sup) &&
    (!state.q || (m.title + ' ' + m.kind + ' ' + (m.style || '') + ' ' + m.tags.join(' ')).toLowerCase().includes(state.q)));
  $('#cards').innerHTML = list.map((m) => `<a class="card" role="listitem" href="/m/${m.design}" data-d="${m.design}" aria-current="${m.design === state.design}"><span class="thumb"><img src="${m.preview}" alt="" loading="lazy" width="192" height="64"></span><span><span class="t"><i class="sup ${supplierOf(m)}"></i><span>${esc(m.title)}</span></span><span class="m">${esc(m.kind)} · ${fmt(m.artifact.tris)} tris · ${m.facts.sockets.length} sockets</span></span></a>`).join('')
    || '<p class="note">Nothing on the shelf matches that.</p>';
  for (const a of $$('#cards .card')) {
    a.onclick = (e) => { if (e.metaKey || e.ctrlKey || e.shiftKey) return; e.preventDefault(); select(a.dataset.d, true); };
  }
  counts();
}

function counts() {
  const by = (s) => MODELS.filter((m) => supplierOf(m) === s).length;
  const set = (id, n) => { const e = $(id); if (e) e.textContent = n; };
  set('#c-all', MODELS.length); set('#n-all', MODELS.length);
  for (const s of ['chisel', 'grove', 'avatar']) { set('#c-' + s, by(s)); set('#n-' + s, by(s)); }
  const av = $('#sup [data-sup="avatar"]');
  if (av) av.disabled = by('avatar') === 0;
}

function select(design, push) {
  state.preview = null;
  state.design = design;
  state.lod = 0;
  shelf();
  show();
  if (push) history.pushState({ design }, '', '/m/' + design);
}

// ── ledger + table ─────────────────────────────────────────────────────────

function show() {
  const m = current();
  if (!m) return;
  const lods = m.artifact.lods || [];
  const unshelved = !!state.preview;

  $('#t-title').textContent = m.title;
  $('#t-kind').textContent = [m.kind, m.style, supplierOf(m)].filter(Boolean).join(' · ') + (unshelved ? ' · not published' : '');
  for (const b of $$('#lod button')) {
    const i = +b.dataset.lod;
    b.disabled = i > 0 && !lods[i - 1];
    b.setAttribute('aria-pressed', String(i === state.lod));
  }

  const f = m.facts;
  $('#facts').innerHTML = `<dt>Size</dt><dd>${dims(f.size)} m</dd><dt>Origin</dt><dd>${f.origin}</dd><dt>Front</dt><dd>${f.front}</dd><dt>Collider</dt><dd>${f.collider}</dd><dt>Parts</dt><dd>${f.parts} · ${esc(f.materials.join(', '))}</dd><dt>Triangles</dt><dd>${fmt(m.artifact.tris)}</dd><dt>LODs</dt><dd>${lods.length ? lods.length + ' coarser' : 'none'}</dd><dt>Bytes</dt><dd>${fmt(m.artifact.bytes)}</dd>`;

  const r = m.recipe;
  $('#recipe').innerHTML = r.package === 'grove'
    ? `<dt>Package</dt><dd>grove</dd><dt>Grown from</dt><dd>${Object.keys(r.recipe || {}).length} rules · seed ${(r.recipe || {}).seed ?? 'default'}</dd>`
    : `<dt>Package</dt><dd>${esc(r.package)}</dd><dt>Export</dt><dd>${esc(r.export)}(${(r.args || []).map(trim).join(', ')})</dd><dt>Material</dt><dd>${esc(r.material || 'its own')}</dd>`;
  $('#recipe-json').textContent = JSON.stringify(r.recipe || { export: r.export, args: r.args, material: r.material }, null, 1);

  const sk = f.sockets || [];
  const byKind = {};
  for (const x of sk) byKind[x.kind] = (byKind[x.kind] || 0) + 1;
  $('#sockets').innerHTML = sk.length
    ? `<dt>Count</dt><dd>${sk.length}</dd><dt>Kinds</dt><dd>${esc(Object.entries(byKind).map(([k, n]) => `${n} ${k}`).join(' · ') || '—')}</dd><dt>First</dt><dd>${esc(sk[0].name)} @ ${sk[0].at.map((v) => v.toFixed(2)).join(', ')}</dd>`
    : '<dt>Count</dt><dd>none — nothing attaches here</dd>';
  const item = f.item;
  $('#item').hidden = !item;
  if (item) {
    $('#item-dl').innerHTML = `<dt>Kind</dt><dd>${esc(item.kind)} · ${esc(item.slot)}</dd><dt>partId</dt><dd>${item.id}</dd><dt>Attach</dt><dd>${esc(item.attach)}${(item.bones || []).length ? ' · ' + item.bones.length + ' joints' : ''}</dd><dt>Textures</dt><dd>${item.textures}</dd><dt>Convention</dt><dd>${item.failed === 0 ? 'adheres' : item.failed + ' of ' + item.checks.length + ' rules failed'}</dd>`;
    $('#checks').innerHTML = checksHtml(item.checks);
  }
  const life = f.life;
  $('#life').hidden = !life;
  if (life) $('#life-dl').innerHTML = `<dt>Stage</dt><dd>${esc(life.stage)}</dd><dt>Year</dt><dd>${esc(life.phase)}</dd><dt>Maturity</dt><dd>${(life.maturity * 100).toFixed(0)} %</dd><dt>Branches</dt><dd>${fmt(life.branches)}</dd>` +
    ((r.recipe || {}).withered ? '<dt>State</dt><dd>withered</dd>' : '');

  const last = (m.verdicts || [])[(m.verdicts || []).length - 1];
  for (const b of $$('#verdict button')) {
    b.setAttribute('aria-pressed', String(!!last && b.dataset.v === last.verdict));
    b.disabled = unshelved;
  }
  // Only when the design changes: switching LOD re-renders the ledger, and
  // a half-typed note must survive that.
  if (state.notedFor !== m.design) {
    $('#note').value = last ? last.note : '';
    state.notedFor = m.design;
  }
  $('#note').disabled = unshelved;
  $('#verdict-log').innerHTML = (m.verdicts || []).slice(-4).reverse().map((v) =>
    `<span class="${v.verdict}"><b>${word(v.verdict)}</b>${v.note ? ' — ' + esc(v.note) : ''} <span>${when(v.at)}</span></span>`).join('');

  $('#prov').innerHTML = `<dt>Design</dt><dd class="id">${m.design}</dd><dt>Origin</dt><dd>${esc(m.origin)}</dd><dt>Codex</dt><dd>${esc(m.codex || '—')}</dd><dt>License</dt><dd>${esc(m.license)}</dd><dt>Author</dt><dd>${esc(m.author || '—')}</dd><dt>sha256</dt><dd class="id">${m.artifact.sha256.slice(0, 20)}…</dd>`;

  $('#foot').innerHTML = `<span>design <b>${m.design}</b></span><span>artifact <b>${m.artifact.url}</b></span><span>${lods.map((u, i) => 'lod' + (i + 1) + ' <b>' + u.split('/').pop() + '</b>').join(' · ') || 'no coarser lods'}</span>` + (unshelved ? '<span><b>not on the shelf</b> — derived to look at</span>' : '');
  $('#a-glb').href = lodUrl(m, state.lod);
  $('#a-again').disabled = unshelved;

  const still = $('#turn');
  if (still) { still.src = m.preview; still.alt = m.title + ' — three-view turntable'; }
  load(m);
  concept(m);
}

const lodUrl = (m, lod) => (lod === 0 ? m.artifact.url : (m.artifact.lods || [])[lod - 1] || m.artifact.url);
const word = (v) => ({ yes: 'matches', close: 'close', no: 'not it' })[v] || v;
const checksHtml = (checks) => (checks || []).map((c) => `<li class="${c.ok ? 'ok' : 'bad'}"><b>${esc(c.rule)}</b> ${esc(c.note)}</li>`).join('');
const trim = (n) => (typeof n === 'number' ? +n.toFixed(4) : n);
function when(sec) {
  if (!sec) return '';
  const d = Math.floor((Date.now() / 1000 - sec) / 86400);
  return d <= 0 ? 'today' : d === 1 ? 'yesterday' : d + ' days ago';
}
function esc(s) {
  return String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

// ── the live viewer ────────────────────────────────────────────────────────

const pane = $('#vpane');
const uniforms = { uTime: { value: 0 }, uWind: { value: 0 }, uAmp: { value: 0.25 } };
let renderer, scene, camera, controls, model, loader, ground, loaded, skeleton;
let token = 0;   // one per load, so a slow glb never replaces a newer one

function viewer() {
  const canvas = document.createElement('canvas');
  pane.insertBefore(canvas, pane.firstChild);
  renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  // A bright studio blows a pale, smooth material to white — the lantern
  // tree is blue crystal and must still read as blue here, or the table is
  // no use for judging against a concept.
  renderer.toneMappingExposure = 0.85;

  scene = new THREE.Scene();
  const pmrem = new THREE.PMREMGenerator(renderer);
  scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  scene.environmentIntensity = 0.55;

  camera = new THREE.PerspectiveCamera(38, 1, 0.01, 500);
  controls = new OrbitControls(camera, canvas);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.minDistance = 0.2;
  controls.maxDistance = 200;

  const key = new THREE.DirectionalLight(0xfff4e2, 1.5);
  key.position.set(3, 5, 4);
  scene.add(key);
  const rim = new THREE.DirectionalLight(0x9fd8ff, 0.55);
  rim.position.set(-4, 2, -3);
  scene.add(rim);
  scene.add(new THREE.HemisphereLight(0x6e7892, 0x14161b, 0.45));

  // A ring on the ground plane: the only honest way to see at a glance
  // whether `origin: base` is true of the mesh and how big the thing is.
  ground = new THREE.PolarGridHelper(1, 8, 4, 64, 0x353945, 0x23262f);
  ground.material.transparent = true;
  ground.material.opacity = 0.5;
  scene.add(ground);

  loader = new GLTFLoader();
  new ResizeObserver(resize).observe(pane);
  resize();
  renderer.setAnimationLoop((t) => {
    uniforms.uTime.value = t / 1000;
    uniforms.uWind.value += ((state.wind ? 1 : 0) - uniforms.uWind.value) * 0.05;
    controls.update();
    renderer.render(scene, camera);
  });
}

function resize() {
  if (!renderer) return;
  const w = pane.clientWidth || 1, h = pane.clientHeight || 1;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}

function say(msg, bad) {
  let el = pane.querySelector('.load');
  if (!el) { el = document.createElement('span'); el.className = 'load'; pane.appendChild(el); }
  el.className = 'load' + (bad ? ' err' : '');
  el.textContent = msg || '';
  el.hidden = !msg;
}

function load(m) {
  if (!renderer) { try { viewer(); } catch (e) { say('no WebGL here — the turntable stands in', true); return; } }
  const url = lodUrl(m, state.lod);
  // Writing a verdict re-renders the ledger; it must not re-download a 3 MB
  // tree to tell you what you just said about it.
  if (url === loaded && model) return;
  loaded = url;
  const mine = ++token;
  $('#cap').textContent = `${m.title.toLowerCase()} · lod${state.lod}`;
  pane.classList.remove('live');
  say('loading ' + url.split('/').pop());
  loader.load(url, (gltf) => {
    if (mine !== token) return;
    if (model) { scene.remove(model); dispose(model); }
    if (skeleton) { scene.remove(skeleton); skeleton = null; }
    model = gltf.scene;
    // A skinned item shows its rig on request: the joints a rebind by name
    // has to find are the thing to look at.
    let rigged = null;
    model.traverse((o) => { if (o.isSkinnedMesh && !rigged) rigged = o; });
    if (rigged) {
      skeleton = new THREE.SkeletonHelper(model);
      skeleton.visible = state.bones;
      scene.add(skeleton);
    }
    $('#bones').hidden = !rigged;
    let swayable = 0;
    model.traverse((o) => {
      if (!o.isMesh) return;
      swayable += rigidity(o.geometry) ? 1 : 0;
      for (const mat of [].concat(o.material)) patch(mat);
    });
    scene.add(model);
    frame(model, m);
    pane.classList.add('live');
    say('');
    $('#cap').textContent = `${m.title.toLowerCase()} · lod${state.lod}` + (swayable ? '' : ' · no wind channel');
    $('#wind').disabled = swayable === 0;
    $('#wind').title = swayable ? 'Play the wind written into the vertex alpha' : 'This model carries no wind channel — carved stone does not sway';
  }, undefined, (err) => {
    if (mine !== token) return;
    loaded = null;
    say('could not open ' + url + ' — ' + (err && err.message ? err.message : 'the artifact did not load'), true);
  });
}

// Copy the grower's rigidity out of vertex-colour alpha into an attribute of
// its own, so the shader does not depend on three deciding to declare the
// colour attribute as a vec4. EVERY geometry gets one, filled with 1.0 where
// there is no wind channel: an attribute a shader reads and a geometry does
// not carry reads as zero in WebGL, which is maximum sway — carved stone
// flapping in the breeze. Rigid is the safe default and the true one.
function rigidity(geo) {
  const have = geo.getAttribute('aRigidity');
  if (have) return !!geo.userData.windy;
  const c = geo.getAttribute('color');
  const n = geo.getAttribute('position').count;
  const a = new Float32Array(n).fill(1);
  let moves = false;
  if (c && c.itemSize === 4 && c.count === n) {
    for (let i = 0; i < n; i++) { a[i] = c.getW(i); if (a[i] < 0.999) moves = true; }
  }
  geo.setAttribute('aRigidity', new THREE.BufferAttribute(a, 1));
  geo.userData.windy = moves;
  return moves;
}

function patch(mat) {
  if (!mat || mat.userData.windy) return;
  mat.userData.windy = true;
  mat.onBeforeCompile = (shader) => {
    shader.uniforms.uTime = uniforms.uTime;
    shader.uniforms.uWind = uniforms.uWind;
    shader.uniforms.uAmp = uniforms.uAmp;
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', `#include <common>
        attribute float aRigidity;
        uniform float uTime; uniform float uWind; uniform float uAmp;`)
      .replace('#include <begin_vertex>', `#include <begin_vertex>
        float sway = (1.0 - aRigidity) * uWind * uAmp;
        float ph = position.x * 0.7 + position.z * 0.5;
        transformed.x += sin(uTime * 1.7 + ph + position.y * 0.35) * sway;
        transformed.z += sin(uTime * 1.3 + ph * 1.4 + 1.7) * sway * 0.7;
        transformed.y -= abs(sin(uTime * 1.7 + ph)) * sway * 0.18;`);
  };
  mat.needsUpdate = true;
}

function frame(obj, m) {
  const box = new THREE.Box3().setFromObject(obj);
  const size = box.getSize(new THREE.Vector3());
  const mid = box.getCenter(new THREE.Vector3());
  const r = Math.max(size.x, size.y, size.z) || 1;
  // A 15 m tree must travel further than a 0.5 m bowl for the same wind.
  uniforms.uAmp.value = Math.max(0.04, size.y * 0.045);
  ground.scale.setScalar(Math.max(r * 0.8, 0.3));
  ground.position.y = Math.min(box.min.y, 0);
  camera.position.set(r * 1.15, mid.y + r * 0.42, r * 1.7);
  controls.target.set(0, mid.y, 0);
  controls.minDistance = r * 0.25;
  controls.maxDistance = r * 12;
  camera.near = r / 400;
  camera.far = r * 60;
  camera.updateProjectionMatrix();
  controls.update();
  void m;
}

function dispose(root) {
  root.traverse((o) => {
    if (!o.isMesh) return;
    o.geometry.dispose();
    for (const mat of [].concat(o.material)) {
      for (const k of Object.keys(mat)) { const v = mat[k]; if (v && v.isTexture) v.dispose(); }
      mat.dispose();
    }
  });
}

// ── concept beside ─────────────────────────────────────────────────────────
// Most of the Codex is sealed and this page arrives anonymous, so the entry
// carries the gallery URL it is answering to. The live Codex is tried first
// anyway, in case the entity is public and its gallery has moved on.

const CODEX = 'https://api.pixygon.io/v1/codex/entities/';
const seen = new Map();

async function concept(m) {
  const cp = $('#cpane');
  $('#stage').classList.toggle('split', state.judge);
  cp.hidden = !state.judge;
  if (!state.judge) return;
  let url = m.concept || '';
  if (m.codex && !seen.has(m.codex)) {
    seen.set(m.codex, null);
    try {
      const r = await fetch(CODEX + encodeURIComponent(m.codex));
      if (r.ok) {
        const e = await r.json();
        const g = (e.gallery || [])[0];
        seen.set(m.codex, (g && g.url) || e.imageUrl || null);
      }
    } catch { /* sealed, offline, or CORS — the stored url still works */ }
  }
  url = seen.get(m.codex) || url;
  cp.innerHTML = url
    ? `<img src="${esc(url)}" alt="Concept for ${esc(m.title)}"><span class="cap">concept · ${esc(m.codex || 'attached')}</span>`
    : `<div class="empty">No concept for this design yet. Attach the Codex entity and its image and it appears here, every time you open it.</div>`;
  const shelved = MODELS.some((x) => x.design === m.design);
  if (shelved) {
    const at = document.createElement('div');
    at.className = 'attach';
    at.innerHTML = `<input id="cx-slug" placeholder="codex slug" value="${esc(m.codex || '')}"><input id="cx-url" placeholder="concept image url" value="${esc(m.concept || '')}"><button id="cx-save">Attach</button>`;
    cp.appendChild(at);
    $('#cx-save').onclick = async () => {
      if (GATED && !key() && !askKey()) return;
      try {
        const e = await send(`/models/${m.design}/concept`, { codex: $('#cx-slug').value.trim(), concept: $('#cx-url').value.trim() });
        replace(e);
        seen.delete(e.codex);
        show();
        toast('concept attached');
      } catch (err) { toast(err.message, true); }
    };
  }
}

function replace(e) {
  const i = MODELS.findIndex((m) => m.design === e.design);
  if (i >= 0) MODELS[i] = e; else MODELS.push(e);
  MODELS.sort((a, b) => a.title.localeCompare(b.title));
}

// ── controls ───────────────────────────────────────────────────────────────

for (const b of $$('#sup .chip')) b.onclick = () => {
  state.sup = b.dataset.sup;
  for (const x of $$('#sup .chip')) x.setAttribute('aria-pressed', String(x === b));
  shelf();
};
$('#q').oninput = (e) => { state.q = e.target.value.toLowerCase(); shelf(); };
for (const b of $$('#lod button')) b.onclick = () => { state.lod = +b.dataset.lod; show(); };
$('#judge').onclick = () => {
  state.judge = $('#judge').getAttribute('aria-pressed') !== 'true';
  $('#judge').setAttribute('aria-pressed', String(state.judge));
  concept(current());
  setTimeout(resize, 60);
};
$('#bones').onclick = () => {
  state.bones = $('#bones').getAttribute('aria-pressed') !== 'true';
  $('#bones').setAttribute('aria-pressed', String(state.bones));
  if (skeleton) skeleton.visible = state.bones;
};
$('#wind').onclick = () => {
  state.wind = $('#wind').getAttribute('aria-pressed') !== 'true';
  $('#wind').setAttribute('aria-pressed', String(state.wind));
};
addEventListener('popstate', () => {
  const d = location.pathname.startsWith('/m/') ? location.pathname.slice(3) : (MODELS[0] || {}).design;
  if (d) select(d, false);
});

for (const b of $$('#verdict button')) b.onclick = async () => {
  const m = current();
  if (!m || state.preview) return;
  if (GATED && !key() && !askKey()) return;
  try {
    const e = await send(`/models/${m.design}/verdict`, { verdict: b.dataset.v, note: $('#note').value.trim() });
    replace(e);
    show();
    toast('verdict kept with the design');
  } catch (err) { toast(err.message, true); }
};

$('#a-id').onclick = () => { navigator.clipboard.writeText(current().design); toast('design id copied'); };
$('#a-unity').onclick = () => {
  navigator.clipboard.writeText(location.origin + current().artifact.url);
  toast('glb url copied — Unity: Pixygon → Quarry → Import');
};
$('#a-again').onclick = async () => {
  const m = current();
  if (GATED && !key() && !askKey()) return;
  try {
    const e = await send('/publish', submissionOf(m));
    replace(e);
    // Same recipe, same design id, same url — and possibly a different mesh,
    // because the grower moved. Drop the cached artifact or the table shows
    // yesterday's tree.
    loaded = null;
    show();
    toast('re-derived from the recipe — ' + fmt(e.artifact.tris) + ' tris');
  } catch (err) { toast(err.message, true); }
};

function submissionOf(m) {
  if (m.recipe.package === 'avatar') {
    return { package: 'avatar', source: m.design, title: m.title, tags: m.tags, style: m.style, codex: m.codex, concept: m.concept };
  }
  return {
    title: m.title, description: m.description, tags: m.tags, kind: m.kind, style: m.style,
    package: m.recipe.package, export: m.recipe.export, args: m.recipe.args,
    recipe: m.recipe.recipe, material: m.recipe.material,
    license: m.license, author: m.author, origin: m.origin, codex: m.codex, concept: m.concept,
    rest: !!m.recipe.rest,
  };
}

// ── the Make door ──────────────────────────────────────────────────────────

let LIB = null, shape = null, door = 'chisel';

$('#make-open').onclick = async () => {
  $('#maker').showModal();
  if (!LIB) {
    try { LIB = await (await fetch('/library')).json(); } catch { msg('the library did not answer', true); return; }
    chiselDoor();
    groveDoor();
  }
};
$('#make-close').onclick = () => $('#maker').close();
for (const b of $$('#doors button')) b.onclick = () => {
  door = b.dataset.door;
  for (const x of $$('#doors button')) x.setAttribute('aria-pressed', String(x === b));
  for (const d of ['chisel', 'grove', 'avatar']) $('#door-' + d).hidden = d !== door;
  msg('');
};

function msg(t, bad) {
  const el = $('#m-msg');
  el.textContent = t || '';
  el.className = 'msg' + (bad ? ' err' : t ? ' ok' : '');
}

function chiselDoor() {
  $('#shapes').innerHTML = LIB.models.map((s) => `<button data-e="${s.export}">${s.label}</button>`).join('');
  $('#c-material').innerHTML = '<option value="">its own</option>' + LIB.materials.map((m) => `<option>${m}</option>`).join('');
  for (const b of $$('#shapes button')) b.onclick = () => pickShape(b.dataset.e);
  pickShape(LIB.models[0].export);
}

function pickShape(name) {
  shape = LIB.models.find((s) => s.export === name);
  for (const b of $$('#shapes button')) b.setAttribute('aria-pressed', String(b.dataset.e === name));
  $('#chisel-args').innerHTML = shape.args.map((a, i) =>
    `<div class="field"><label for="ca-${i}">${esc(a.name)}</label><input id="ca-${i}" type="number" step="${a.type === 'int' ? 1 : 0.01}" value="${a.default}"></div>`).join('');
  $('#c-title').value = shape.label;
  $('#c-kind').value = shape.kind;
  $('#c-material').value = shape.material;
  $('#c-tags').value = shape.kind;
  // The library centres its primitives; a placed thing stands on something.
  $('#c-rest').checked = !!shape.centred;
}

function chiselSubmission() {
  const args = shape.args.map((a, i) => {
    const v = parseFloat($('#ca-' + i).value);
    return Number.isFinite(v) ? v : a.default;
  });
  return {
    title: $('#c-title').value.trim() || shape.label,
    kind: $('#c-kind').value.trim() || shape.kind,
    style: $('#c-style').value.trim(),
    tags: $('#c-tags').value.split(',').map((t) => t.trim()).filter(Boolean),
    package: 'weft-model', export: shape.export, args,
    material: $('#c-material').value,
    codex: $('#c-codex').value.trim(),
    origin: 'authored',
    rest: $('#c-rest').checked,
  };
}

// Grove's form is built from the grower's own default recipe — a rule added
// upstream shows up here the day it lands — but the ORDER is ours: a tree is
// read trunk-first, and serde hands the keys over alphabetically. Any key
// the grower has that is not named below lives in the JSON beside the form.
// `seed` is deliberately absent: it has its own field above.
const GROVE_HELP = {
  name: 'name', height: 'trunk to 1st fork (m)', trunk_radius: 'trunk radius (m)',
  flare: 'root flare', lean: 'lean °', trunk_curve: 'trunk bend °',
  levels: 'branch generations', forks: 'forks / level', branches: 'laterals / level',
  angle: 'branch angle °', length: 'child length ×', child_radius: 'child radius ×',
  curve: 'branch bend °', gravity: 'gravity', wobble: 'wobble', taper: 'taper',
  sprout_from: 'sprout from', sway: 'sway', emissive: 'glow',
  seasons_to_grown: 'seasons to grown', sprout_size: 'sprout size ×', seasons_of_life: 'seasons of life',
  evergreen: 'evergreen', sides: 'ring sides', segments: 'segments', lods: 'lod detail ×',
};
// The planting's own fields — the individual, the moment, what happened to
// it — have their own controls above the rules and are kept out of the grid.
const PLANTING = ['seed', 'age', 'season', 'withered', 'cut', 'taken', 'clock', 'state'];

// f32 through f64 prints 0.550000011920929. Show the number a person typed,
// and only write it back when they actually touch the field — rounding an
// untouched value would fork the design id off the recipe it came from.
const tidy = (v) => (typeof v === 'number' ? +v.toPrecision(7) : v);
let recipe = {};

function groveDoor() {
  const grown = MODELS.filter((m) => m.recipe.package === 'grove');
  $('#g-from').innerHTML = '<option value="">a blank recipe</option>' + grown.map((m) => `<option value="${m.design}">${esc(m.title)}</option>`).join('');
  $('#g-from').onchange = () => {
    const m = grown.find((x) => x.design === $('#g-from').value);
    recipe = structuredClone(m ? (m.recipe.recipe || {}) : LIB.grove.blank);
    if (m) { $('#g-title').value = m.title; $('#g-style').value = m.style || ''; $('#g-tags').value = m.tags.join(', '); $('#g-codex').value = m.codex || ''; }
    groveForm();
  };
  recipe = structuredClone(LIB.grove.blank);
  groveForm();
  $('#g-json').oninput = () => {
    try { recipe = JSON.parse($('#g-json').value); msg(''); groveForm(true); }
    catch (e) { msg('the recipe is not valid JSON yet', true); }
  };
  $('#g-seed').oninput = () => { recipe.seed = Math.max(0, parseInt($('#g-seed').value, 10) || 0); groveForm(); };
  // The clock. Age blank = the whole potential plant, the grower's meaning
  // of "no age"; season wraps 0..1 through bud, leaf, bloom, fruit, seed
  // drop and bare.
  $('#g-age').oninput = () => {
    const v = parseFloat($('#g-age').value);
    if (Number.isFinite(v) && v >= 0) recipe.age = v; else delete recipe.age;
    groveForm();
  };
  $('#g-season').onchange = () => { recipe.season = parseFloat($('#g-season').value); groveForm(); };
  $('#g-withered').onchange = () => { if ($('#g-withered').checked) recipe.withered = true; else delete recipe.withered; groveForm(); };
}

const SEASONS = [[0.05, 'bud'], [0.2, 'leaf'], [0.35, 'bloom'], [0.55, 'fruit'], [0.75, 'seed drop'], [0.92, 'bare']];

function groveForm(fromJson) {
  const blank = LIB.grove.blank;
  const keys = Object.keys(GROVE_HELP).filter((k) => k in blank && !PLANTING.includes(k));
  $('#grove-args').innerHTML = keys.map((k) => {
    const v = recipe[k] ?? blank[k];
    const arr = Array.isArray(v);
    if (typeof v === 'boolean') {
      return `<div class="field"><label for="ga-${k}">${esc(GROVE_HELP[k])}</label><input id="ga-${k}" data-k="${k}" type="checkbox" ${v ? 'checked' : ''}></div>`;
    }
    const val = arr ? v.map(tidy).join(', ') : tidy(v);
    return `<div class="field"><label for="ga-${k}">${esc(GROVE_HELP[k])}</label><input id="ga-${k}" data-k="${k}" data-arr="${arr}" ${arr || typeof v === 'string' ? '' : 'type="number" step="any"'} value="${esc(val)}"></div>`;
  }).join('');
  for (const i of $$('#grove-args input')) i.oninput = () => {
    const k = i.dataset.k;
    if (i.type === 'checkbox') {
      recipe[k] = i.checked;
    } else if (i.dataset.arr === 'true') {
      recipe[k] = i.value.split(',').map((x) => parseFloat(x)).filter((x) => Number.isFinite(x));
    } else if (typeof (recipe[k] ?? LIB.grove.blank[k]) === 'string') {
      recipe[k] = i.value;
    } else {
      const n = parseFloat(i.value);
      if (Number.isFinite(n)) recipe[k] = n;
    }
    $('#g-json').value = JSON.stringify(recipe, null, 1);
  };
  $('#g-seed').value = recipe.seed ?? blank.seed ?? 1;
  $('#g-age').value = recipe.age ?? '';
  const season = recipe.season ?? blank.season ?? 0.35;
  $('#g-season').innerHTML = SEASONS.map(([v, n]) => `<option value="${v}">${n}</option>`).join('') +
    (SEASONS.some(([v]) => Math.abs(v - season) < 0.001) ? '' : `<option value="${season}">${season.toFixed(2)} (as written)</option>`);
  $('#g-season').value = String(SEASONS.find(([v]) => Math.abs(v - season) < 0.001)?.[0] ?? season);
  $('#g-withered').checked = !!recipe.withered;
  if (!fromJson) $('#g-json').value = JSON.stringify(recipe, null, 1);
}

function groveSubmission() {
  return {
    title: $('#g-title').value.trim() || recipe.name || 'A tree',
    kind: 'tree',
    style: $('#g-style').value.trim(),
    tags: $('#g-tags').value.split(',').map((t) => t.trim()).filter(Boolean),
    package: 'grove', recipe,
    codex: $('#g-codex').value.trim(),
    origin: 'grown',
  };
}

const submission = () => (door === 'grove' ? groveSubmission() : chiselSubmission());

// The Avatar door sends a file, not a recipe: the GLB is the body and the
// words ride in the query, because the manifest inside already carries most
// of them.
let glbFile = null;
const drop = $('.drop');
$('#a-file').onchange = () => pickFile($('#a-file').files[0]);
drop.ondragover = (e) => { e.preventDefault(); drop.classList.add('over'); };
drop.ondragleave = () => drop.classList.remove('over');
drop.ondrop = (e) => { e.preventDefault(); drop.classList.remove('over'); pickFile(e.dataTransfer.files[0]); };
function pickFile(f) {
  glbFile = f || null;
  $('#a-drop-text').textContent = f ? `${f.name} · ${(f.size / 1024 / 1024).toFixed(2)} MB` : 'Drop a manifested .glb here, or choose one';
  $('#a-checks').innerHTML = '';
  msg('');
}
async function sendGlb(path) {
  if (!glbFile) throw new Error('choose a .glb first');
  const q = new URLSearchParams();
  for (const [k, id] of [['title', '#a-title'], ['style', '#a-style'], ['tags', '#a-tags'], ['codex', '#a-codex']]) {
    const v = $(id).value.trim();
    if (v) q.set(k, v);
  }
  const headers = { 'content-type': 'model/gltf-binary' };
  if (key()) headers.authorization = 'Bearer ' + key();
  const r = await fetch(path + '?' + q, { method: 'POST', headers, body: glbFile });
  const text = await r.text();
  if (!r.ok) {
    if (r.status === 401) throw new Error('the store refused the key — unlock again');
    throw new Error(text.slice(0, 300) || ('HTTP ' + r.status));
  }
  return JSON.parse(text);
}

async function run(path, then) {
  if (GATED && !key() && !askKey()) return;
  const foot = $('.maker .foot');
  foot.setAttribute('aria-busy', 'true');
  msg(door === 'avatar' ? 'the store is reading the file…' : 'the store is running the recipe…');
  try {
    const e = door === 'avatar' ? await sendGlb(path) : await send(path, submission());
    if (door === 'avatar' && e.facts && e.facts.item) $('#a-checks').innerHTML = checksHtml(e.facts.item.checks);
    then(e);
  } catch (err) {
    msg(err.message, true);
  } finally {
    foot.removeAttribute('aria-busy');
  }
}

$('#m-derive').onclick = () => run('/derive', (e) => {
  state.preview = e;
  state.lod = 0;
  const item = e.facts.item;
  msg(item
    ? (item.failed === 0 ? `adheres — ${item.checks.length} rules kept. ` : `${item.failed} of ${item.checks.length} rules failed — see below. `) + fmt(e.artifact.tris) + ' tris, ' + dims(e.facts.size) + ' m.'
    : 'derived — ' + fmt(e.artifact.tris) + ' tris, ' + dims(e.facts.size) + ' m. Nothing is on the shelf yet.');
  show();
  // A failed check is the point of looking: keep the list in front of you.
  if (!item || item.failed === 0) $('#maker').close();
  toast('on the table, not on the shelf');
});

$('#m-publish').onclick = () => run('/publish', (e) => {
  replace(e);
  state.preview = null;
  state.design = e.design;
  state.lod = 0;
  msg('published as ' + e.design);
  shelf();
  show();
  history.pushState({ design: e.design }, '', '/m/' + e.design);
  $('#maker').close();
  toast('on the shelf — ' + e.title);
});

// ── go ─────────────────────────────────────────────────────────────────────

if (GATED) paintKey();
shelf();
show();
