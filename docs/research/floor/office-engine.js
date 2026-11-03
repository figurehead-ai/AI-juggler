/* The Floor: a procedural pixel office (research prototype, not app code).
 *
 * One engine draws the office from AI Juggler's model: each App gets a zone of
 * desk bays, each live session a desk and a person, subagents are helper bots,
 * pools meet in meeting rooms, This Mac is the server room. Everything is
 * generated from a seed, drawn once into caches and reused:
 *   - the floor (floors, walls, rooms, desks) is baked into one canvas per floor;
 *   - every person is built from parts into one sprite atlas, once per staff slot;
 *   - text cards are cached per desk and redrawn only when their data changes.
 * Frames are drawn on demand. A low-rate ticker runs only while something
 * animated is on screen, the page is visible and motion is allowed.
 */
(function (G) {
  'use strict';

  const TILE = 16;
  const LOBBY_W = 9, BAY_W = 8, BAY_H = 8, BX = 8, BY = 3, TOP = 10;
  const W = 1 + LOBBY_W + BX * BAY_W + 1, H = TOP + BY * BAY_H + 1;
  const OX = 1 + LOBBY_W, OY = TOP;
  const BAYS_PER_FLOOR = BX * BY;
  // Desk top-left tiles inside a bay: two rows of two.
  const DESK_SPOTS = [[1, 2], [4, 2], [1, 5], [4, 5]];
  const FW = 14, FH = 22; // sprite frame incl. 1 px outline margin

  // ---------- seeded randomness ----------
  function hash32(s) {
    let h = 2166136261 >>> 0;
    for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
    return h >>> 0;
  }
  function mulberry(seed) {
    let a = seed >>> 0;
    return function () {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  const rngFor = (...parts) => mulberry(hash32(parts.join('|')));
  const pick = (r, a) => a[(r() * a.length) | 0];

  // ---------- colour ----------
  function hslHex(h, s, l) {
    s /= 100; l /= 100;
    const k = n => (n + h / 30) % 12, a = s * Math.min(l, 1 - l);
    const f = n => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
    return '#' + [f(0), f(8), f(4)].map(x => Math.round(x * 255).toString(16).padStart(2, '0')).join('');
  }
  function shade(hex, f) {
    const n = parseInt(hex.slice(1), 16);
    const c = [n >> 16, (n >> 8) & 255, n & 255].map(v => Math.max(0, Math.min(255, Math.round(f > 1 ? v + (255 - v) * (f - 1) : v * f))));
    return '#' + c.map(v => v.toString(16).padStart(2, '0')).join('');
  }

  const P = {
    outline: '#1d1726', eye: '#2a2230', white: '#f4f1ea', mouth: '#7a2e35',
    wallFace: '#d8cfbf', wallFaceDark: '#c2b8a6', wallTop: '#433a4f', wallTrim: '#a89c88',
    glass: '#9fcbe6', glassHi: '#d4ecf8', frame: '#ece6d8',
    carpet: '#5a6378', corridor: '#8c909b', corridorDark: '#838793',
    lobbyA: '#a77b52', lobbyB: '#9c714a', lobbyLine: '#86613f',
    kitchenA: '#e6e0d2', kitchenB: '#d2cab9',
    serverA: '#3c4150', serverLine: '#4b5163',
    meeting: '#5e5470', meetingLine: '#564c67',
    office: '#8a6a4c', officeLine: '#7d6046', archive: '#6d7062', archiveLine: '#646757',
    metal: '#c9cfdb', metalDark: '#8d94a3', black: '#262533', keyb: '#cfd3dc',
    leaf: '#3f9b55', leafDark: '#2b7240', leafHi: '#6cc77c', pot: '#b4643a', potDark: '#8d4b2a',
    wilt: '#a39250', wiltDark: '#7a6c3a',
    paper: '#f2efe6', paperLine: '#c9c3b5', cork: '#c08a52', corkDark: '#9c6c3c',
    trash: '#2c2f36', trashHi: '#4a505c', box: '#b98b57', boxDark: '#93693d',
    fire1: '#ffe08a', fire2: '#ff9a3d', fire3: '#e5452f',
    sky: '#bfe3f7', skyNight: '#1d2547', cloud: '#f4fbff', star: '#e9f0ff',
  };
  const DESK_STYLES = [
    { top: '#cf9f68', hi: '#e2b67f', front: '#9b6f43' },
    { top: '#e9e7e1', hi: '#f7f5f0', front: '#bdb9ae' },
    { top: '#7b5638', hi: '#906745', front: '#563a24' },
    { top: '#b8c3cf', hi: '#ccd5df', front: '#8893a0' },
  ];
  const CHAIR_COLORS = ['#3b3f4c', '#5a3d5c', '#2f4f5f', '#4b4f2f', '#6a3b33', '#334466'];
  const SKIN = ['#f6d5bd', '#ebbb97', '#d29b70', '#b07650', '#8a5634', '#62391f'];
  const HAIR_C = ['#2b1d14', '#4a3020', '#7a4b25', '#b07a3a', '#dcb766', '#e6e1d6', '#a33b2e', '#33407a', '#6b2d5c', '#1f1f24'];
  const HAIR_STYLES = ['short', 'long', 'bun', 'bald', 'curly', 'pony', 'buzz', 'bob', 'cap', 'spiky'];
  const PANTS = ['#2f3a5c', '#3a3a42', '#7a6a4a', '#3d5a80', '#222228', '#5b4636'];
  const SHOES = ['#1f1b22', '#5b3b2a', '#e8e4dc', '#8a2f2f'];

  // State words follow SPEC §9's rail; glyph shape and word carry the meaning, colour only helps.
  const STATES = {
    working: { word: 'Working', color: '#22d3ee', rank: 5 },
    asks: { word: 'Asks you', color: '#fbbf24', rank: 1 },
    error: { word: 'Stopped with an error', color: '#f87171', rank: 0 },
    limit: { word: 'Limit reached', color: '#fb923c', rank: 2 },
    stuck: { word: 'Probably stuck', color: '#f59e0b', rank: 3 },
    done: { word: 'Done, review it', color: '#34d399', rank: 4 },
    idle: { word: 'Idle', color: '#a3acbb', rank: 6 },
    ended: { word: 'Ended', color: '#7b8494', rank: 7 },
  };
  const NEEDS_YOU = new Set(['asks', 'error', 'limit', 'stuck']);

  function mk(w, h) { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; }

  // ---------- characters: built from parts once, cached ----------
  const F = { SIT0: 0, SIT1: 1, SITIDLE: 2, SLEEP: 3, SCRATCH: 4, LEAN: 5, STAND: 6, WAVE0: 7, WAVE1: 8, ALARM: 9,
    DOWN0: 10, DOWN1: 11, UP0: 12, UP1: 13, SIDE0: 14, SIDE1: 15, LSIDE0: 16, LSIDE1: 17 };
  const FRAME_COUNT = 18;

  function characterSpec(seed, hue) {
    const r = mulberry(seed);
    const style = pick(r, HAIR_STYLES);
    return {
      skin: pick(r, SKIN), hairC: pick(r, HAIR_C), hair: style,
      shirt: hslHex((hue + r() * 24 - 12 + 360) % 360, 42 + ((r() * 22) | 0), 40 + ((r() * 16) | 0)),
      shirtStyle: pick(r, ['tee', 'collar', 'hoodie', 'tie', 'stripe', 'sweater']),
      pants: pick(r, PANTS), shoes: pick(r, SHOES), cap: hslHex((hue + 180) % 360, 55, 45),
      glasses: r() < 0.3, phones: r() < 0.18, beard: r() < 0.16,
    };
  }

  function paintPerson(ctx, ox, oy, sp, frame) {
    const R = (x, y, w, h, c) => { ctx.fillStyle = c; ctx.fillRect(ox + x, oy + y, w, h); };
    const sit = frame <= F.LEAN;
    const front = frame >= F.STAND && frame <= F.DOWN1;
    const side = frame >= F.SIDE0;
    const back = !front && !side;
    let hy = sit ? 5 : 1; // head top
    if (frame === F.SLEEP) hy = 8;
    if (frame === F.LEAN) hy = 4;
    const ty = sit ? 11 : 7; // torso top
    const shirtD = shade(sp.shirt, 0.78);
    // torso and arms
    if (side) {
      R(4, ty, 5, 6, sp.shirt); R(4, ty, 1, 6, shirtD);
    } else {
      R(2, ty, 8, sit ? 6 : 6, sp.shirt); R(8, ty, 2, 6, shirtD);
      if (sp.shirtStyle === 'stripe') { R(2, ty + 2, 8, 1, shirtD); R(2, ty + 4, 8, 1, shirtD); }
      if (sp.shirtStyle === 'sweater') R(2, ty + 5, 8, 1, shirtD);
      if (front) {
        if (sp.shirtStyle === 'collar' || sp.shirtStyle === 'tie') { R(4, ty, 1, 1, P.white); R(7, ty, 1, 1, P.white); }
        if (sp.shirtStyle === 'tie') R(5, ty + 1, 2, 4, '#7a1f2b');
        if (sp.shirtStyle === 'hoodie') { R(4, ty, 4, 1, shirtD); R(5, ty + 1, 1, 2, P.white); R(6, ty + 1, 1, 2, P.white); }
      } else if (sp.shirtStyle === 'hoodie') R(3, ty, 6, 2, shirtD);
    }
    // arms by pose
    const arm = (x, y, h) => { R(x, y, 1, h, shirtD); R(x, y + h, 1, 1, sp.skin); };
    if (frame === F.SIT0) { arm(1, ty + 1, 3); arm(10, ty + 2, 3); }
    else if (frame === F.SIT1) { arm(1, ty + 2, 3); arm(10, ty + 1, 3); }
    else if (frame === F.SITIDLE) { arm(1, ty + 1, 4); arm(10, ty + 1, 4); }
    else if (frame === F.SLEEP) { R(1, ty - 1, 2, 3, shirtD); R(9, ty - 1, 2, 3, shirtD); }
    else if (frame === F.SCRATCH) { arm(1, ty + 1, 4); R(10, hy + 2, 1, ty - hy, shirtD); R(9, hy, 2, 2, sp.skin); }
    else if (frame === F.LEAN) { R(2, hy + 1, 1, ty - hy, shirtD); R(9, hy + 1, 1, ty - hy, shirtD); R(3, hy + 1, 1, 1, sp.skin); R(8, hy + 1, 1, 1, sp.skin); }
    else if (frame === F.STAND || frame === F.DOWN0 || frame === F.DOWN1 || frame === F.UP0 || frame === F.UP1) {
      const sw = frame === F.DOWN1 || frame === F.UP1 ? 1 : 0;
      arm(1, ty + sw, 5); arm(10, ty + 1 - sw, 5);
    } else if (frame === F.WAVE0 || frame === F.WAVE1) {
      arm(1, ty, 5);
      R(10, ty - 4, 1, 5, shirtD);
      if (frame === F.WAVE0) R(10, ty - 6, 2, 2, sp.skin); else R(9, ty - 7, 2, 2, sp.skin);
    } else if (frame === F.ALARM) {
      R(1, hy + 2, 1, ty - hy + 1, shirtD); R(10, hy + 2, 1, ty - hy + 1, shirtD);
      R(2, hy + 1, 1, 2, sp.skin); R(9, hy + 1, 1, 2, sp.skin);
    } else if (side) {
      const sw = frame === F.SIDE1 || frame === F.LSIDE1;
      R(sw ? 7 : 5, ty + 1, 1, 4, shirtD); R(sw ? 7 : 5, ty + 5, 1, 1, sp.skin);
    }
    // legs (standing and walking only)
    if (!sit) {
      if (side) {
        const sw = frame === F.SIDE1 || frame === F.LSIDE1;
        if (sw) { R(5, 13, 3, 5, sp.pants); R(5, 18, 4, 1, sp.shoes); }
        else { R(4, 13, 2, 5, sp.pants); R(7, 13, 2, 5, shade(sp.pants, 0.8)); R(3, 18, 3, 1, sp.shoes); R(7, 18, 3, 1, sp.shoes); }
      } else {
        const step = frame === F.DOWN0 || frame === F.UP0 ? 1 : frame === F.DOWN1 || frame === F.UP1 ? 2 : 0;
        R(3, 13, 3, step === 1 ? 4 : 5, sp.pants); R(6, 13, 3, step === 2 ? 4 : 5, shade(sp.pants, 0.85));
        R(3, step === 1 ? 17 : 18, 3, 1, sp.shoes); R(6, step === 2 ? 17 : 18, 3, 1, sp.shoes);
      }
    }
    // head
    if (side) {
      R(4, hy, 5, 6, sp.skin);
      R(8, hy + 3, 1, 1, P.eye);
      if (sp.hair !== 'bald') { R(3, hy - 1 + (sp.hair === 'buzz' ? 1 : 0), 6, 2, sp.hairC); R(3, hy + 1, 2, sp.hair === 'long' || sp.hair === 'bob' ? 6 : 3, sp.hairC); }
      if (sp.hair === 'pony') R(2, hy + 2, 1, 4, sp.hairC);
      if (sp.hair === 'cap') { R(3, hy - 1, 6, 2, sp.cap); R(8, hy + 1, 2, 1, sp.cap); }
      if (sp.glasses) R(7, hy + 3, 2, 1, P.black);
      if (sp.phones) R(4, hy + 2, 1, 2, P.black);
      return;
    }
    R(3, hy + 1, 6, 5, sp.skin);
    const hc = sp.hairC;
    if (back) {
      if (sp.hair !== 'bald') R(3, hy, 6, 5, hc);
      else { R(3, hy + 2, 1, 2, hc); R(8, hy + 2, 1, 2, hc); }
      if (sp.hair === 'long') { R(2, hy + 1, 1, 6, hc); R(9, hy + 1, 1, 6, hc); R(3, hy + 5, 6, 2, hc); }
      if (sp.hair === 'bob') { R(2, hy + 1, 8, 5, hc); }
      if (sp.hair === 'pony') R(5, hy + 5, 2, 3, hc);
      if (sp.hair === 'bun') R(5, hy - 1, 2, 2, hc);
      if (sp.hair === 'curly') R(2, hy - 1, 8, 6, hc);
      if (sp.hair === 'spiky') { R(3, hy - 1, 1, 1, hc); R(5, hy - 1, 1, 1, hc); R(7, hy - 1, 1, 1, hc); }
      if (sp.hair === 'cap') { R(3, hy - 1, 6, 3, sp.cap); }
      if (sp.phones) { R(3, hy - 1, 6, 1, P.black); R(2, hy + 2, 1, 2, P.black); R(9, hy + 2, 1, 2, P.black); }
      R(3, hy, 1, 1, shade(hc, 1.25));
      return;
    }
    // front face
    const S = sp.hair;
    if (S === 'short' || S === 'pony' || S === 'spiky') { R(3, hy, 6, 2, hc); R(3, hy + 2, 1, 1, hc); R(8, hy + 2, 1, 1, hc); }
    if (S === 'spiky') { R(3, hy - 1, 1, 1, hc); R(5, hy - 1, 1, 1, hc); R(7, hy - 1, 1, 1, hc); }
    if (S === 'long') { R(3, hy, 6, 2, hc); R(2, hy + 1, 1, 7, hc); R(9, hy + 1, 1, 7, hc); }
    if (S === 'bob') { R(2, hy, 8, 2, hc); R(2, hy + 2, 1, 4, hc); R(9, hy + 2, 1, 4, hc); }
    if (S === 'bun') { R(3, hy, 6, 2, hc); R(5, hy - 1, 2, 1, hc); }
    if (S === 'curly') { R(2, hy - 1, 8, 3, hc); R(2, hy + 2, 1, 2, hc); R(9, hy + 2, 1, 2, hc); }
    if (S === 'buzz') R(3, hy, 6, 1, hc);
    if (S === 'bald') { R(3, hy + 2, 1, 1, hc); R(8, hy + 2, 1, 1, hc); }
    if (S === 'cap') { R(3, hy - 1, 6, 2, sp.cap); R(3, hy + 1, 7, 1, shade(sp.cap, 0.8)); }
    R(4, hy + 3, 1, 1, P.eye); R(7, hy + 3, 1, 1, P.eye);
    if (sp.glasses) { R(3, hy + 3, 6, 1, P.black); R(4, hy + 3, 1, 1, '#9fd8ff'); R(7, hy + 3, 1, 1, '#9fd8ff'); }
    if (sp.beard) { R(3, hy + 4, 1, 2, hc); R(8, hy + 4, 1, 2, hc); R(4, hy + 5, 4, 1, hc); }
    if (frame === F.WAVE0 || frame === F.WAVE1 || frame === F.ALARM) R(5, hy + 4, 2, 1, P.mouth);
    if (sp.phones) { R(3, hy - 1, 6, 1, P.black); R(2, hy + 2, 1, 2, P.black); R(9, hy + 2, 1, 2, P.black); }
  }

  // One dark outline around every opaque pixel, per frame.
  function outlineAtlas(c, color) {
    const ctx = c.getContext('2d');
    const img = ctx.getImageData(0, 0, c.width, c.height);
    const d = img.data, w = c.width, h = c.height;
    const n = parseInt(color.slice(1), 16), cr = n >> 16, cg = (n >> 8) & 255, cb = n & 255;
    const solid = new Uint8Array(w * h);
    for (let i = 0; i < w * h; i++) solid[i] = d[i * 4 + 3] > 0 ? 1 : 0;
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (solid[i]) continue;
      if ((x > 0 && solid[i - 1]) || (x < w - 1 && solid[i + 1]) || (y > 0 && solid[i - w]) || (y < h - 1 && solid[i + w])) {
        d[i * 4] = cr; d[i * 4 + 1] = cg; d[i * 4 + 2] = cb; d[i * 4 + 3] = 255;
      }
    }
    ctx.putImageData(img, 0, 0);
  }

  function buildCharacter(seed, hue) {
    const sp = characterSpec(seed, hue);
    const c = mk(FW * FRAME_COUNT, FH);
    const ctx = c.getContext('2d');
    for (let f = 0; f < F.LSIDE0; f++) paintPerson(ctx, f * FW + 1, 1, sp, f);
    for (const [src, dst] of [[F.SIDE0, F.LSIDE0], [F.SIDE1, F.LSIDE1]]) {
      ctx.save(); ctx.translate(dst * FW + FW, 0); ctx.scale(-1, 1);
      ctx.drawImage(c, src * FW, 0, FW, FH, 0, 0, FW, FH); ctx.restore();
    }
    outlineAtlas(c, P.outline);
    return { atlas: c, spec: sp };
  }

  // Helper bots stand for subagents: one small atlas per App hue.
  function buildBot(hue) {
    const c = mk(10 * 3, 11), ctx = c.getContext('2d');
    const body = hslHex(hue, 55, 52), visor = hslHex(hue, 90, 75);
    for (let f = 0; f < 3; f++) {
      const o = f * 10 + 1, y = f === 1 ? 0 : 1;
      const R = (x, yy, w, h, col) => { ctx.fillStyle = col; ctx.fillRect(o + x, y + yy, w, h); };
      R(1, 0, 6, 5, P.metal); R(1, 0, 6, 1, '#e3e8f1'); R(2, 2, 4, 2, f === 2 ? '#fbbf24' : visor);
      R(3, -0 + 5, 2, 1, P.metalDark); R(1, 6, 6, 2, body); R(0, 6, 1, 1, P.metalDark); R(7, 6, 1, 1, P.metalDark);
    }
    outlineAtlas(c, P.outline);
    return c;
  }

  // ---------- small prop sprites (baked once) ----------
  function sprite(w, h, paint, outline) {
    const c = mk(w, h); paint(c.getContext('2d'), (ctx, x, y, ww, hh, col) => { ctx.fillStyle = col; ctx.fillRect(x, y, ww, hh); });
    if (outline) outlineAtlas(c, P.outline);
    return c;
  }
  const SPR = {};
  function buildProps() {
    if (SPR.ready) return;
    SPR.fire = [0, 1, 2].map(f => sprite(12, 14, (x, R) => {
      const o = [[0, 0], [1, -1], [-1, 0]][f];
      R(x, 2 + o[0], 6, 6, 3, P.fire3); R(x, 3, 5 + o[1], 4, 5, P.fire3); R(x, 7 - o[0], 4, 2, 5, P.fire3);
      R(x, 3 + o[0], 8, 4, 4, P.fire2); R(x, 4, 6 + o[1], 3, 4, P.fire2); R(x, 5, 9, 3, 3, P.fire1); R(x, 5 + o[0], 7, 2, 2, P.fire1);
    }, false));
    SPR.smoke = sprite(8, 6, (x, R) => { R(x, 1, 1, 5, 4, 'rgba(70,72,84,.55)'); R(x, 3, 0, 4, 3, 'rgba(90,92,104,.45)'); });
    SPR.dust = [0, 1, 2].map(v => sprite(30, 10, (x, R) => {
      const r = mulberry(91 + v);
      for (let i = 0; i < 26; i++) R(x, (r() * 30) | 0, (r() * 10) | 0, 1, 1, 'rgba(225,220,205,.75)');
      for (let i = 0; i < 6; i++) R(x, (r() * 28) | 0, (r() * 9) | 0, 2, 1, 'rgba(190,185,170,.6)');
    }));
    SPR.cobweb = sprite(7, 7, (x, R) => {
      for (let i = 0; i < 7; i++) { R(x, i, 0, 1, 1, 'rgba(235,235,240,.8)'); R(x, 0, i, 1, 1, 'rgba(235,235,240,.8)'); }
      for (let i = 0; i < 5; i++) R(x, i, 4 - i, 1, 1, 'rgba(235,235,240,.7)');
      R(x, 2, 2, 1, 1, 'rgba(235,235,240,.7)');
    });
    SPR.trash = sprite(12, 11, (x, R) => {
      R(x, 1, 3, 10, 7, P.trash); R(x, 2, 2, 8, 1, P.trash); R(x, 4, 0, 4, 2, P.trash); R(x, 5, 1, 2, 1, P.trashHi);
      R(x, 2, 4, 2, 3, P.trashHi); R(x, 1, 10, 10, 1, 'rgba(0,0,0,.25)');
    }, true);
    SPR.box = sprite(12, 10, (x, R) => { R(x, 1, 2, 10, 7, P.box); R(x, 1, 2, 10, 2, shade(P.box, 1.15)); R(x, 5, 2, 2, 7, P.boxDark); }, true);
    SPR.pizza = sprite(10, 4, (x, R) => { R(x, 0, 0, 10, 4, '#d9b38a'); R(x, 0, 0, 10, 1, '#e8cba7'); R(x, 3, 1, 4, 1, '#b5523b'); });
    SPR.mug = sprite(5, 5, (x, R) => { R(x, 0, 1, 3, 4, P.white); R(x, 3, 2, 1, 2, P.white); R(x, 0, 1, 3, 1, '#6b4226'); }, true);
    SPR.papers = sprite(8, 5, (x, R) => { R(x, 0, 1, 7, 4, P.paper); R(x, 1, 0, 7, 4, P.paper); R(x, 2, 1, 4, 1, P.paperLine); R(x, 2, 3, 3, 1, P.paperLine); }, true);
    SPR.review = sprite(9, 6, (x, R) => { R(x, 0, 2, 9, 4, '#8f8fa0'); R(x, 1, 0, 7, 4, P.paper); R(x, 2, 1, 4, 1, P.paperLine); R(x, 6, 0, 2, 2, '#34d399'); }, true);
    SPR.zz = sprite(9, 9, (x, R) => { R(x, 4, 0, 4, 1, P.white); R(x, 6, 1, 1, 1, P.white); R(x, 5, 2, 1, 1, P.white); R(x, 4, 3, 4, 1, P.white); R(x, 0, 5, 3, 1, P.white); R(x, 1, 6, 1, 1, P.white); R(x, 0, 7, 3, 1, P.white); }, true);
    SPR.beacon = [0, 1].map(f => sprite(8, 8, (x, R) => {
      R(x, 1, 3, 6, 5, '#7a1d1d'); R(x, 2, 1, 4, 3, '#ff4d4d'); R(x, f ? 2 : 4, 1, 2, 2, '#ffd0d0');
    }, true));
    SPR.steam = [0, 1].map(f => sprite(5, 6, (x, R) => { R(x, f ? 1 : 2, 0, 1, 2, 'rgba(255,255,255,.7)'); R(x, f ? 2 : 1, 2, 1, 2, 'rgba(255,255,255,.6)'); R(x, f ? 1 : 2, 4, 1, 2, 'rgba(255,255,255,.5)'); }));
    SPR.cat = [0, 1, 2].map(f => sprite(12, 9, (x, R) => {
      const c = '#e09a4c', d = '#b8743a';
      if (f === 2) { R(x, 1, 4, 9, 4, c); R(x, 1, 3, 3, 2, c); R(x, 1, 2, 1, 1, c); R(x, 3, 2, 1, 1, c); R(x, 9, 6, 2, 2, d); R(x, 4, 5, 4, 1, d); return; }
      R(x, 2, 3, 7, 3, c); R(x, 8, 1, 3, 3, c); R(x, 8, 0, 1, 1, c); R(x, 10, 0, 1, 1, c); R(x, 10, 2, 1, 1, P.eye);
      R(x, 0, f ? 1 : 2, 2, 1, c); R(x, 1, 2, 1, 1, c); R(x, 4, 4, 3, 1, d);
      R(x, f ? 3 : 2, 6, 1, 2, c); R(x, f ? 7 : 8, 6, 1, 2, c);
    }, true));
    SPR.folder = sprite(6, 5, (x, R) => { R(x, 0, 1, 6, 4, '#e8b04a'); R(x, 0, 0, 3, 1, '#e8b04a'); R(x, 0, 2, 6, 1, '#f3c86a'); }, true);
    SPR.envelope = sprite(6, 5, (x, R) => { R(x, 0, 0, 6, 5, P.white); R(x, 1, 1, 1, 1, P.paperLine); R(x, 2, 2, 2, 1, P.paperLine); R(x, 4, 1, 1, 1, P.paperLine); }, true);
    SPR.ready = true;
  }

  // ---------- floor plan: stable bays, zones, rooms ----------
  function bayOrder() {
    // Column-major serpentine, so each App's run of bays stays contiguous.
    const out = [];
    for (let bx = 0; bx < BX; bx++) for (let k = 0; k < BY; k++) out.push([bx, bx % 2 ? BY - 1 - k : k]);
    return out;
  }
  const ROOMS_F0 = [['server', 8], ['meeting', 9], ['meeting', 9], ['kitchen', 12], ['archive', 8], ['lounge', 9], ['boss', 9]];
  const ROOMS_FN = [['meeting', 9], ['meeting', 9], ['quiet', 9], ['kitchen', 12], ['storage', 8], ['lounge', 9], ['phone', 8]];

  function makeFloor(index) {
    const rooms = []; let x = OX;
    for (const [kind, w] of index === 0 ? ROOMS_F0 : ROOMS_FN) { rooms.push({ kind, x, y: 2, w, h: 7, ref: null }); x += w; }
    const bays = bayOrder().map(([bx, by], i) => ({ bx, by, i, appId: null, desks: [] }));
    return { index, rooms, bays, bake: null, bakeTheme: null, grid: null, props: [] };
  }

  class Layout {
    constructor(seed) { this.seed = seed; this.floors = []; this.appBays = new Map(); this.deskOf = new Map(); this.planned = false; }
    floor(i) { while (this.floors.length <= i) this.floors.push(makeFloor(this.floors.length)); return this.floors[i]; }
    freeBays(f) { return this.floor(f).bays.filter(b => !b.appId); }
    claim(appId, n, preferFloor) {
      // Contiguous run on one floor when it fits; otherwise the next floor.
      for (let f = preferFloor ?? 0; f < 32; f++) {
        const fl = this.floor(f); const order = fl.bays;
        for (let s = 0; s + n <= order.length; s++) {
          let ok = true;
          for (let k = 0; k < n; k++) if (order[s + k].appId) { ok = false; break; }
          if (ok) { const got = order.slice(s, s + n); got.forEach(b => this.giveBay(b, appId, f)); return got; }
        }
        if (n > BAYS_PER_FLOOR) n = BAYS_PER_FLOOR;
      }
      return [];
    }
    giveBay(b, appId, f) {
      b.appId = appId;
      const x0 = OX + b.bx * BAY_W, y0 = OY + b.by * BAY_H;
      b.desks = DESK_SPOTS.map(([dx, dy], k) => ({
        id: `${f}:${b.i}:${k}`, floor: f, bay: b, tx: x0 + dx, ty: y0 + dy, appId, sessionId: null, lastSessionId: null, staff: 0,
        style: 0, chair: 0, items: null, x: (x0 + dx) * TILE, y: (y0 + dy) * TILE,
      }));
      if (!this.appBays.has(appId)) this.appBays.set(appId, []);
      this.appBays.get(appId).push(b);
      const list = this.appBays.get(appId);
      b.desks.forEach((d, k) => { d.staff = (list.length - 1) * 4 + k; });
      this.floor(f).bake = null;
    }
    desksOf(appId) { return (this.appBays.get(appId) || []).flatMap(b => b.desks); }
  }

  // Pick desks for sessions: sticky desk per session; new sessions take free desks, then ended ones.
  function seatSessions(layout, model) {
    const byApp = new Map();
    for (const s of model.sessions) { if (!byApp.has(s.appId)) byApp.set(s.appId, []); byApp.get(s.appId).push(s); }
    for (const app of model.apps) {
      const all = byApp.get(app.id) || [];
      const live = all.filter(s => s.state !== 'ended');
      const ended = all.filter(s => s.state === 'ended').sort((a, b) => (b.endedAt || 0) - (a.endedAt || 0));
      const keepEnded = live.length ? ended.slice(0, 2) : ended.slice(0, 4);
      const want = live.concat(keepEnded);
      if (!layout.appBays.has(app.id)) {
        const n = Math.max(1, Math.ceil(want.length / 4));
        const prefer = layout.floors.length ? layout.floors.length - 1 : 0;
        layout.claim(app.id, n, layout.planned ? prefer : 0);
      }
      const desks = layout.desksOf(app.id);
      const wantIds = new Set(want.map(s => s.id));
      for (const d of desks) if (d.sessionId && !wantIds.has(d.sessionId)) { d.lastSessionId = d.sessionId; d.sessionId = null; layout.deskOf.delete(d.lastSessionId); }
      for (const s of want) {
        const had = layout.deskOf.get(s.id);
        if (had && had.appId === s.appId) continue;
        if (had) { had.sessionId = null; layout.deskOf.delete(s.id); }
        let d = desks.find(x => !x.sessionId);
        if (!d && s.state !== 'ended') {
          // Annex: the App grows into the next free bay (any floor) rather than moving anyone.
          const got = layout.claim(app.id, 1, desks[0] ? desks[0].floor : 0);
          if (got.length) { desks.push(...got[0].desks); d = got[0].desks[0]; }
        }
        if (!d) continue;
        d.sessionId = s.id; layout.deskOf.set(s.id, d);
      }
    }
    layout.planned = true;
  }

  // ---------- baking a floor (once per seed, floor and theme) ----------
  function bakeFloor(layout, fl, model, night) {
    const c = mk(W * TILE, H * TILE), ctx = c.getContext('2d');
    const R = (x, y, w, h, col) => { ctx.fillStyle = col; ctx.fillRect(x, y, w, h); };
    const r = rngFor(layout.seed, 'floor', fl.index);
    const appHue = id => { const a = model.apps.find(x => x.id === id); return a ? a.hue : 220; };
    // base floor everywhere
    for (let ty = 0; ty < H; ty++) for (let tx = 0; tx < W; tx++) R(tx * TILE, ty * TILE, TILE, TILE, ((tx + ty) & 1) ? P.corridor : P.corridorDark);
    // lobby planks
    for (let ty = 2; ty < H - 1; ty++) for (let tx = 1; tx < OX; tx++) {
      R(tx * TILE, ty * TILE, TILE, TILE, (tx + (ty >> 1)) % 2 ? P.lobbyA : P.lobbyB);
      R(tx * TILE, ty * TILE + 7, TILE, 1, P.lobbyLine); R(tx * TILE + ((ty & 1) ? 4 : 11), ty * TILE, 1, 7, P.lobbyLine);
    }
    // office carpet, tinted per App zone
    for (const b of fl.bays) {
      const x0 = (OX + b.bx * BAY_W) * TILE, y0 = (OY + b.by * BAY_H) * TILE;
      const base = b.appId ? hslHex(appHue(b.appId), 16, 40) : P.carpet;
      R(x0, y0, BAY_W * TILE, BAY_H * TILE, base);
      for (let i = 0; i < 40; i++) R(x0 + ((r() * BAY_W * TILE) | 0), y0 + ((r() * BAY_H * TILE) | 0), 1, 1, shade(base, 0.86));
      R(x0, y0, BAY_W * TILE, 1, shade(base, 0.8)); R(x0, y0, 1, BAY_H * TILE, shade(base, 0.8));
    }
    // rooms
    for (const rm of fl.rooms) {
      const x0 = rm.x * TILE, y0 = rm.y * TILE, w = rm.w * TILE, h = rm.h * TILE;
      if (rm.kind === 'server') { R(x0, y0, w, h, P.serverA); for (let i = 0; i < w; i += 8) R(x0 + i, y0, 1, h, P.serverLine); for (let j = 0; j < h; j += 8) R(x0, y0 + j, w, 1, P.serverLine); }
      else if (rm.kind === 'kitchen') { for (let ty = 0; ty < rm.h; ty++) for (let tx = 0; tx < rm.w; tx++) for (let q = 0; q < 4; q++) R(x0 + tx * 16 + (q & 1) * 8, y0 + ty * 16 + (q >> 1) * 8, 8, 8, (q === 0 || q === 3) ? P.kitchenA : P.kitchenB); }
      else if (rm.kind === 'boss') { R(x0, y0, w, h, P.office); for (let j = 0; j < h; j += 6) R(x0, y0 + j, w, 1, P.officeLine); }
      else if (rm.kind === 'archive' || rm.kind === 'storage') { R(x0, y0, w, h, P.archive); for (let j = 0; j < h; j += 16) R(x0, y0 + j, w, 1, P.archiveLine); }
      else { R(x0, y0, w, h, P.meeting); for (let i = 0; i < 50; i++) R(x0 + ((r() * w) | 0), y0 + ((r() * h) | 0), 1, 1, P.meetingLine); }
    }
    // top wall face with windows, per room and lobby
    R(0, 0, W * TILE, 2, P.wallTop);
    R(0, 2, W * TILE, 2 * TILE - 2, night ? shade(P.wallFace, 0.9) : P.wallFace);
    R(0, 2 * TILE - 3, W * TILE, 3, P.wallTrim);
    const windowAt = (x, w) => {
      R(x, 6, w, 18, P.frame); R(x + 2, 8, w - 4, 14, night ? P.skyNight : P.sky);
      if (night) for (let i = 0; i < 4; i++) R(x + 3 + ((r() * (w - 6)) | 0), 9 + ((r() * 10) | 0), 1, 1, P.star);
      else { R(x + 4 + ((r() * (w - 14)) | 0), 11, 7, 2, P.cloud); }
      R(x + (w >> 1), 8, 1, 14, P.frame); R(x + 2, 8, w - 4, 1, night ? '#2b3560' : P.glassHi);
    };
    for (const rm of fl.rooms) {
      const x0 = rm.x * TILE;
      if (rm.kind === 'server') { R(x0 + 10, 8, 26, 10, '#e9edf3'); R(x0 + 12, 10, 22, 2, '#b9c1cf'); R(x0 + 12, 14, 22, 1, '#b9c1cf'); windowAt(x0 + 70, 40); }
      else if (rm.kind === 'kitchen') {
        for (let i = 0; i < 4; i++) { R(x0 + 6 + i * 22, 5, 20, 14, '#e9e2d3'); R(x0 + 6 + i * 22, 5, 20, 1, '#fff8ea'); R(x0 + 15 + i * 22, 11, 2, 2, P.metalDark); }
        windowAt(x0 + 100, 52);
      } else if (rm.kind === 'archive' || rm.kind === 'storage') { windowAt(x0 + 34, 44); }
      else if (rm.kind === 'boss') { R(x0 + 8, 6, 26, 18, '#4b3a2c'); for (let i = 0; i < 3; i++) R(x0 + 10, 8 + i * 6, 22, 4, ['#c0573d', '#3d6fb0', '#4f9a5a'][i]); windowAt(x0 + 46, 56); }
      else { windowAt(x0 + 8, 30); R(x0 + 52, 7, 66, 17, '#23262f'); R(x0 + 54, 9, 62, 13, night ? '#1a2a3a' : '#22384d'); }
    }
    // lobby wall: elevator, corkboard, clock wall (dynamic content drawn later)
    R(1 * TILE + 4, 4, 30, 28, P.metalDark); R(1 * TILE + 6, 8, 12, 24, P.metal); R(1 * TILE + 20, 8, 12, 24, P.metal); R(1 * TILE + 19, 8, 1, 24, P.metalDark);
    R(1 * TILE + 13, 1, 12, 6, '#222'); // floor indicator
    R(3 * TILE + 8, 5, 48, 22, P.corkDark); R(3 * TILE + 10, 7, 44, 18, P.cork);
    R(7 * TILE + 2, 5, 38, 22, '#e9e4d8'); R(7 * TILE + 2, 5, 38, 1, '#fff');
    // room dividers and glass fronts with door gaps
    for (const rm of fl.rooms) {
      const x0 = rm.x * TILE, yb = (rm.y + rm.h) * TILE;
      R(x0 - 2, 2 * TILE, 4, rm.h * TILE + 4, P.wallTop); R(x0 - 1, 2 * TILE, 2, rm.h * TILE, P.wallFaceDark);
      const door = rm.x + Math.floor(rm.w / 2) - 1;
      for (let tx = rm.x; tx < rm.x + rm.w; tx++) {
        if (tx === door || tx === door + 1) continue;
        R(tx * TILE, yb - 2, TILE, 5, rm.kind === 'server' || rm.kind === 'archive' || rm.kind === 'storage' ? P.wallFaceDark : P.glass);
        R(tx * TILE, yb - 2, TILE, 1, P.glassHi); R(tx * TILE, yb + 2, TILE, 1, P.wallTop);
      }
      R(door * TILE, yb - 2, 1, 5, P.wallTop); R((door + 2) * TILE - 1, yb - 2, 1, 5, P.wallTop);
    }
    R((OX + BX * BAY_W) * TILE - 2, 2 * TILE, 4, 7 * TILE + 4, P.wallTop);
    // outer walls
    R(0, 0, 6, H * TILE, P.wallTop); R(W * TILE - 6, 0, 6, H * TILE, P.wallTop); R(0, H * TILE - 6, W * TILE, 6, P.wallTop);
    // lobby / office divider: planters with gaps
    for (let ty = TOP + 1; ty < H - 2; ty += 4) plant(ctx, OX * TILE - 8, ty * TILE + 4, r, 'box');

    // room furniture
    fl.props = [];
    const prop = (kind, x, y, w, h, ref, label) => fl.props.push({ kind, x, y, w, h, ref, label });
    for (const rm of fl.rooms) {
      const x0 = rm.x * TILE, y0 = rm.y * TILE, w = rm.w * TILE;
      prop('room', x0, y0 - 2 * TILE, w, rm.h * TILE + 2 * TILE, rm, rm.kind);
      if (rm.kind === 'server') {
        for (let i = 0; i < 3; i++) { const rx = x0 + 10 + i * 20; R(rx, y0 + 2, 16, 40, '#1f2230'); R(rx + 1, y0 + 3, 14, 38, '#2a2e3e'); R(rx, y0 + 42, 16, 2, 'rgba(0,0,0,.3)'); }
        R(x0 + 76, y0 + 60, 30, 14, '#4a5060'); R(x0 + 78, y0 + 62, 26, 2, '#6b7282'); // console
        plant(ctx, x0 + w - 20, y0 + 80, r, 'tall');
      } else if (rm.kind === 'kitchen') {
        R(x0 + 4, y0, w - 8, 12, '#d8d0bf'); R(x0 + 4, y0, w - 8, 2, '#f0eadc'); R(x0 + 4, y0 + 12, w - 8, 6, '#a8a090');
        R(x0 + 30, y0 - 2, 10, 10, '#3a3a44'); R(x0 + 32, y0 + 2, 6, 3, '#6b4226'); // coffee machine
        R(x0 + 60, y0 + 2, 14, 6, P.metal); // sink
        R(x0 + w - 28, y0 - 14, 20, 40, '#e8ecf2'); R(x0 + w - 28, y0 + 4, 20, 1, '#b9c1cf'); R(x0 + w - 12, y0 - 8, 1, 8, P.metalDark);
        table(ctx, x0 + 40, y0 + 52, 44, 24, '#cf9f68', r);
        prop('coffee', x0 + 26, y0 - 8, 18, 22, rm, 'Coffee machine');
      } else if (rm.kind === 'lounge') {
        R(x0 + 14, y0 + 40, 100, 44, '#3f5a6f'); R(x0 + 16, y0 + 42, 96, 40, '#4a6a82');
        R(x0 + 18, y0 + 8, 60, 16, '#8a4f6a'); R(x0 + 18, y0 + 8, 60, 4, '#a3627f'); R(x0 + 18, y0 + 24, 60, 6, '#6c3c53');
        table(ctx, x0 + 34, y0 + 52, 30, 12, '#cf9f68', r, true);
        R(x0 + 90, y0 + 6, 20, 22, '#5a4030'); for (let k = 0; k < 3; k++) R(x0 + 92, y0 + 9 + k * 6, 16, 4, ['#3d6fb0', '#c0573d', '#e8b04a'][k]);
        plant(ctx, x0 + 4, y0 + 82, r, 'tall'); plant(ctx, x0 + 120, y0 + 82, r, 'round');
      } else if (rm.kind === 'phone') {
        for (let i = 0; i < 3; i++) { const bx = x0 + 8 + i * 38; R(bx, y0 + 4, 30, 50, '#3a4252'); R(bx + 2, y0 + 6, 26, 44, '#9fcbe6'); R(bx + 2, y0 + 6, 26, 2, '#d4ecf8'); R(bx + 10, y0 + 30, 10, 6, '#cf9f68'); }
      } else if (rm.kind === 'meeting' || rm.kind === 'quiet') {
        table(ctx, x0 + 26, y0 + 40, 76, 30, rm.kind === 'quiet' ? '#7b5638' : '#e9e7e1', r);
        plant(ctx, x0 + 8, y0 + 86, r, 'tall');
      } else if (rm.kind === 'archive' || rm.kind === 'storage') {
        for (let i = 0; i < 5; i++) cabinet(ctx, x0 + 6 + i * 20, y0 + 2);
        for (let i = 0; i < 4; i++) cabinet(ctx, x0 + 14 + i * 20, y0 + 58);
      } else if (rm.kind === 'boss') {
        R(x0 + 22, y0 + 30, 66, 12, '#7b5638'); R(x0 + 22, y0 + 30, 66, 2, '#906745'); R(x0 + 22, y0 + 42, 66, 8, '#563a24');
        R(x0 + 46, y0 + 18, 18, 13, P.black); R(x0 + 48, y0 + 20, 14, 9, '#22384d'); R(x0 + 54, y0 + 31, 2, 2, P.black);
        R(x0 + 48, y0 + 58, 14, 14, '#2a2430'); R(x0 + 46, y0 + 56, 18, 6, '#3a3242');
        R(x0 + 16, y0 + 80, 80, 20, '#7a2f3a'); R(x0 + 18, y0 + 82, 76, 16, '#8f3a46');
        R(x0 + 92, y0 + 4, 14, 34, '#5a4030'); // trophy shelf
        for (let k = 0; k < 3; k++) R(x0 + 92, y0 + 12 + k * 10, 14, 1, '#3e2a1e');
        plant(ctx, x0 + 6, y0 + 4, r, 'tall');
        prop('trophies', x0 + 90, y0 + 2, 18, 38, rm, 'Trophy shelf');
      }
    }
    // lobby furniture
    const L = TILE;
    if (fl.index === 0) {
      R(2 * L, 5 * L, 5 * L, 10, '#e9e7e1'); R(2 * L, 5 * L, 5 * L, 2, '#fff'); R(2 * L, 5 * L + 10, 5 * L, 8, '#9aa3b2'); // reception
    } else {
      R(2 * L + 4, 5 * L, 4 * L, 18, '#5a6a8a'); R(2 * L + 4, 5 * L, 4 * L, 4, '#6f80a3');
    }
    R(2 * L, 12 * L, 5 * L, 14, '#4f6a8f'); R(2 * L, 12 * L, 5 * L, 4, '#6582ab'); R(2 * L, 12 * L + 14, 5 * L, 6, '#3c5272'); // sofa
    table(ctx, 3 * L, 14 * L + 8, 3 * L, 14, '#cf9f68', r, true);
    R(7 * L + 4, 19 * L, 10, 18, '#cfe6f5'); R(7 * L + 2, 21 * L, 14, 16, '#e8ecf2'); R(7 * L + 6, 21 * L + 4, 4, 2, '#3d6fb0'); // water cooler
    prop('cooler', 7 * L, 19 * L, 18, 36, null, 'Water cooler');
    R(2 * L, 23 * L, 22, 34, '#3a2f50'); R(2 * L + 2, 23 * L + 2, 18, 20, '#5b4a7a'); for (let i = 0; i < 4; i++) R(2 * L + 4 + (i % 2) * 8, 23 * L + 4 + (i >> 1) * 9, 6, 7, ['#e5452f', '#34d399', '#fbbf24', '#3d6fb0'][i]); // vending
    prop('vending', 2 * L, 23 * L, 22, 34, null, 'Vending machine');
    for (let i = 0; i < 6; i++) { R(2 * L, (29 + i * 0.66) * L, 5 * L, 10, i % 2 ? '#9b8f7c' : '#b5a991'); R(2 * L, (29 + i * 0.66) * L, 5 * L, 1, '#d2c8b4'); } // stairs
    prop('stairs', 2 * L, 29 * L, 5 * L, 4 * L, null, 'Stairs');
    plant(ctx, 7 * L + 2, 9 * L, r, 'tall'); plant(ctx, 1 * L + 2, 17 * L, r, 'round');
    prop('elevator', 1 * L, 0, 2.6 * L, 2 * L, null, 'Elevator');
    prop('corkboard', 3 * L + 6, 2, 52, 28, null, 'Corkboard');
    prop('clocks', 7 * L, 2, 42, 28, null, 'Clock wall');

    // bays: head-strip decor and desk surfaces
    for (const b of fl.bays) {
      const x0 = (OX + b.bx * BAY_W) * TILE, y0 = (OY + b.by * BAY_H) * TILE;
      const br = rngFor(layout.seed, 'bay', fl.index, b.i);
      if (!b.appId) {
        // Empty bay: room to grow, a plant and stacked chairs.
        plant(ctx, x0 + 56, y0 + 40, br, 'round');
        R(x0 + 20, y0 + 70, 24, 8, '#7d8696'); R(x0 + 20, y0 + 70, 24, 2, '#9aa2b0');
        continue;
      }
      const zoneFirst = (layout.appBays.get(b.appId) || [])[0] === b;
      if (!zoneFirst) { if (br() < 0.7) plant(ctx, x0 + 96 + ((br() * 16) | 0), y0 + 18, br, pick(br, ['round', 'tall', 'cactus'])); }
      if (!zoneFirst && br() < 0.4) { R(x0 + 30, y0 + 10, 18, 14, '#d9dde5'); R(x0 + 32, y0 + 8, 14, 4, '#9aa2b0'); R(x0 + 30, y0 + 22, 18, 3, '#4b5163'); prop('printer', x0 + 30, y0 + 6, 18, 20, b.appId, 'Printer'); }
      for (const d of b.desks) {
        const dr = rngFor(layout.seed, 'desk', d.id);
        d.style = (hash32(layout.seed + b.appId) % DESK_STYLES.length);
        d.chair = hash32('c' + layout.seed + b.appId) % CHAIR_COLORS.length;
        d.items = deskSurface(ctx, d, dr);
      }
    }
    if (night) {
      ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = '#7a86b8'; ctx.fillRect(0, 0, c.width, c.height);
      ctx.globalCompositeOperation = 'source-over';
    }
    fl.grid = walkGrid(fl);
    return c;
  }

  // A desk top, its front panel and a few personal things, fixed per desk and seed.
  function deskSurface(ctx, d, dr) {
    const R = (x, y, w, h, col) => { ctx.fillStyle = col; ctx.fillRect(x, y, w, h); };
    const st = DESK_STYLES[d.style];
    R(d.x + 1, d.y + 4, 30, 8, st.top); R(d.x + 1, d.y + 4, 30, 1, st.hi); R(d.x + 1, d.y + 12, 30, 4, st.front);
    R(d.x + 1, d.y + 16, 30, 1, 'rgba(0,0,0,.18)'); R(d.x + 11, d.y + 8, 10, 2, P.keyb);
    const items = [];
    if (dr() < 0.5) items.push(['plant', d.x + 3, d.y - 2]);
    if (dr() < 0.45) items.push(['photo', d.x + 26, d.y + 2]);
    if (dr() < 0.35) items.push(['books', d.x + 23, d.y + 4]);
    if (dr() < 0.3) items.push(['lamp', d.x + 4, d.y - 1]);
    for (const [k, ix, iy] of items) {
      if (k === 'plant') { R(ix + 1, iy + 6, 4, 4, P.pot); R(ix, iy + 2, 6, 4, P.leaf); R(ix + 2, iy, 2, 3, P.leafHi); }
      if (k === 'photo') { R(ix, iy + 2, 4, 5, '#5a4030'); R(ix + 1, iy + 3, 2, 3, '#9fcbe6'); }
      if (k === 'books') { R(ix, iy + 2, 6, 2, '#c0573d'); R(ix + 1, iy, 5, 2, '#3d6fb0'); }
      if (k === 'lamp') { R(ix + 2, iy + 2, 1, 7, P.black); R(ix, iy, 5, 3, '#e8b04a'); R(ix + 1, iy + 9, 4, 1, P.black); }
    }
    return items;
  }

  function plant(ctx, x, y, r, kind) {
    const R = (a, b, w, h, col) => { ctx.fillStyle = col; ctx.fillRect(x + a, y + b, w, h); };
    if (kind === 'box') { R(0, 6, 16, 8, P.potDark); R(1, 6, 14, 2, P.pot); for (let i = 0; i < 4; i++) R(1 + i * 4, 0 + (i % 2) * 2, 4, 7, i % 2 ? P.leafDark : P.leaf); return; }
    if (kind === 'cactus') { R(3, 10, 8, 6, P.pot); R(5, 1, 4, 10, '#4f9a5a'); R(2, 4, 2, 4, '#4f9a5a'); R(10, 3, 2, 4, '#4f9a5a'); R(6, 2, 1, 7, '#6cc77c'); return; }
    if (kind === 'tall') { R(3, 16, 9, 8, P.pot); R(3, 16, 9, 2, shade(P.pot, 1.2)); R(1, 4, 13, 12, P.leafDark); R(3, 0, 9, 10, P.leaf); R(5, 2, 3, 4, P.leafHi); R(0, 8, 4, 4, P.leaf); return; }
    R(3, 9, 8, 6, P.pot); R(1, 2, 12, 8, P.leaf); R(3, 0, 8, 4, P.leafHi); R(2, 6, 4, 3, P.leafDark);
  }
  function table(ctx, x, y, w, h, top, r, low) {
    ctx.fillStyle = 'rgba(0,0,0,.18)'; ctx.fillRect(x + 2, y + h, w, 3);
    ctx.fillStyle = top; ctx.fillRect(x, y, w, h);
    ctx.fillStyle = shade(top, 1.15); ctx.fillRect(x, y, w, 2);
    ctx.fillStyle = shade(top, 0.75); ctx.fillRect(x, y + h - 3, w, 3);
    if (low) return;
    const n = Math.max(2, Math.floor(w / 18));
    for (let i = 0; i < n; i++) {
      const cx = x + 4 + i * ((w - 12) / (n - 1));
      ctx.fillStyle = '#3b3f4c'; ctx.fillRect(cx, y - 8, 8, 6); ctx.fillRect(cx, y + h + 2, 8, 6);
    }
  }
  function cabinet(ctx, x, y) {
    ctx.fillStyle = '#8d94a3'; ctx.fillRect(x, y, 16, 30); ctx.fillStyle = '#a6adbb'; ctx.fillRect(x, y, 16, 2);
    ctx.fillStyle = '#6f7685'; for (let i = 0; i < 3; i++) { ctx.fillRect(x + 1, y + 9 + i * 9, 14, 1); ctx.fillRect(x + 6, y + 5 + i * 9, 4, 1); }
  }

  // Walkable tiles; desks, chairs, furniture and walls block.
  function walkGrid(fl) {
    const g = new Uint8Array(W * H).fill(1);
    const block = (x, y, w, h) => { for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) if (i >= 0 && j >= 0 && i < W && j < H) g[j * W + i] = 0; };
    block(0, 0, W, 3); block(0, 0, 1, H); block(W - 1, 0, 1, H); block(0, H - 1, W, 1);
    for (const rm of fl.rooms) {
      block(rm.x, rm.y, rm.w, rm.h); // rooms are destinations, entered only at their door tile
      const door = rm.x + Math.floor(rm.w / 2) - 1;
      g[(rm.y + rm.h - 1) * W + door] = 1; g[(rm.y + rm.h - 1) * W + door + 1] = 1;
    }
    for (const b of fl.bays) if (b.appId) for (const d of b.desks) block(d.tx, d.ty, 2, 2);
    block(2, 4, 5, 2); block(2, 12, 5, 4); block(2, 23, 2, 3); block(2, 29, 5, 4);
    for (let ty = TOP + 1; ty < H - 2; ty += 4) g[ty * W + OX - 1] = 0;
    return g;
  }
  function findPath(fl, sx, sy, gx, gy) {
    const g = fl.grid; if (!g) return null;
    const prev = new Int32Array(W * H).fill(-1), start = sy * W + sx, goal = gy * W + gx;
    g[start] = 1; const q = [start]; prev[start] = start;
    for (let qi = 0; qi < q.length; qi++) {
      const cur = q[qi]; if (cur === goal) break;
      const cx = cur % W, cy = (cur / W) | 0;
      for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
        const nx = cx + dx, ny = cy + dy, ni = ny * W + nx;
        if (nx < 0 || ny < 0 || nx >= W || ny >= H || prev[ni] !== -1 || (!g[ni] && ni !== goal)) continue;
        prev[ni] = cur; q.push(ni);
      }
    }
    if (prev[goal] === -1) return null;
    const path = []; for (let c = goal; c !== start; c = prev[c]) path.push({ x: (c % W) * TILE + 8, y: ((c / W) | 0) * TILE + 14 });
    path.reverse(); return path;
  }

  // ---------- glyphs for the state of a desk (shape carries the meaning) ----------
  function glyph(ctx, kind, x, y, s, col) {
    ctx.save(); ctx.translate(x, y); ctx.strokeStyle = col; ctx.fillStyle = col; ctx.lineWidth = Math.max(1.5, s / 7); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    const h = s / 2;
    ctx.beginPath();
    switch (kind) {
      case 'working': ctx.arc(0, 0, h * 0.7, -Math.PI * 0.5, Math.PI * 1.1); ctx.stroke(); break;
      case 'asks': ctx.moveTo(0, -h * 0.75); ctx.lineTo(0, h * 0.2); ctx.stroke(); ctx.beginPath(); ctx.arc(0, h * 0.62, ctx.lineWidth * 0.6, 0, 7); ctx.fill(); break;
      case 'stuck': ctx.arc(0, -h * 0.3, h * 0.4, Math.PI * 1.1, Math.PI * 2.4); ctx.lineTo(0, h * 0.25); ctx.stroke(); ctx.beginPath(); ctx.arc(0, h * 0.65, ctx.lineWidth * 0.6, 0, 7); ctx.fill(); break;
      case 'error': ctx.moveTo(-h * 0.55, -h * 0.55); ctx.lineTo(h * 0.55, h * 0.55); ctx.moveTo(h * 0.55, -h * 0.55); ctx.lineTo(-h * 0.55, h * 0.55); ctx.stroke(); break;
      case 'limit': ctx.moveTo(-h * 0.5, -h * 0.7); ctx.lineTo(h * 0.5, -h * 0.7); ctx.lineTo(-h * 0.5, h * 0.7); ctx.lineTo(h * 0.5, h * 0.7); ctx.closePath(); ctx.stroke(); break;
      case 'done': ctx.moveTo(-h * 0.6, 0); ctx.lineTo(-h * 0.15, h * 0.45); ctx.lineTo(h * 0.65, -h * 0.5); ctx.stroke(); break;
      case 'idle': ctx.moveTo(-h * 0.3, -h * 0.5); ctx.lineTo(-h * 0.3, h * 0.5); ctx.moveTo(h * 0.3, -h * 0.5); ctx.lineTo(h * 0.3, h * 0.5); ctx.stroke(); break;
      case 'bot': ctx.rect(-h * 0.55, -h * 0.45, h * 1.1, h * 0.85); ctx.stroke(); ctx.beginPath(); ctx.moveTo(0, -h * 0.45); ctx.lineTo(0, -h * 0.8); ctx.stroke(); break;
      case 'fire': ctx.moveTo(0, -h * 0.8); ctx.quadraticCurveTo(h * 0.8, 0, 0, h * 0.75); ctx.quadraticCurveTo(-h * 0.8, 0, 0, -h * 0.8); ctx.fill(); break;
      default: ctx.rect(-h * 0.45, -h * 0.45, h * 0.9, h * 0.9); ctx.fill();
    }
    ctx.restore();
  }

  function roundRect(ctx, x, y, w, h, r) { ctx.beginPath(); ctx.moveTo(x + r, y); ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r); ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath(); }

  const FONT = '600 11px ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif';
  const FONT_S = '500 10px ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif';
  const FONT_PX = '400 8px "Silkscreen", ui-monospace, monospace';

  // App decay: 0 in use, 1 closed lately (abandoned), 2 a week (dust), 3 a month (trash, lights off).
  const DAY = 86400000;
  function decayOf(app, account) {
    if (app.running) return 0;
    let ago = app.lastActiveAgo ?? 0;
    if (account && account.lastUsedAgo > 30 * DAY) ago = Math.max(ago, account.lastUsedAgo);
    return ago < DAY ? 1 : ago < 7 * DAY ? 2 : 3;
  }
  function agoWords(ms) {
    if (ms < 3600000) return `${Math.max(1, Math.round(ms / 60000))} min`;
    if (ms < DAY) return `${Math.round(ms / 3600000)} h`;
    return `${Math.round(ms / DAY)} days`;
  }

  // ---------- the view ----------
  class FloorView {
    constructor(canvas, opts = {}) {
      // Two layers. The world (pixel art) is drawn at one backing pixel per art pixel and scaled up by the
      // compositor, so a tick repaints a small bitmap. Text, badges and bubbles live on a transparent
      // overlay at device resolution that is redrawn only when data, the camera or the selection change.
      this.canvas = canvas; this.ctx = canvas.getContext('2d');
      this.world = document.createElement('canvas'); this.wctx = this.world.getContext('2d', { alpha: false });
      Object.assign(this.world.style, { position: 'absolute', left: '0', top: '0', transformOrigin: '0 0', pointerEvents: 'none' });
      this.world.setAttribute('aria-hidden', 'true');
      canvas.parentElement.insertBefore(this.world, canvas);
      this.alertEl = document.createElement('div');
      Object.assign(this.alertEl.style, { position: 'absolute', inset: '0', pointerEvents: 'none', background: 'rgba(230,40,40,.16)', display: 'none' });
      canvas.parentElement.insertBefore(this.alertEl, canvas);
      if (!document.getElementById('floor-breathe')) { const st = document.createElement('style'); st.id = 'floor-breathe'; st.textContent = '@keyframes floorBreathe{from{opacity:.5}to{opacity:1}}'; document.head.append(st); }
      this.opts = opts; this.seed = opts.seed || 'office-1';
      this.layout = new Layout(this.seed); this.floorIx = 0;
      this.model = null; this.sessions = new Map(); this.apps = new Map(); this.accounts = new Map();
      this.chars = new Map(); this.bots = new Map(); this.cardCache = new Map(); this.labelCache = new Map();
      this.cam = { x: W * TILE / 2, y: H * TILE / 2, s: 1 }; this.fitted = false;
      this.motion = opts.motion || 'full'; this.night = !!opts.night;
      this.visible = !document.hidden; this.onscreen = true;
      this.t = 0; this.timer = 0; this.raf = 0; this.tween = null;
      this.walkers = []; this.cat = null; this.selected = null; this.hover = null;
      this.stats = { frames: 0, drawMs: 0, window: [], bakeMs: 0, buildMs: 0, ticks: 0, animating: false };
      this.dpr = Math.max(1, Math.min(3, window.devicePixelRatio || 1));
      buildProps();
      this.resize();
      new ResizeObserver(() => { this.resize(); this.invalidate(); }).observe(canvas);
      document.addEventListener('visibilitychange', () => { this.visible = !document.hidden; this.visible ? this.invalidate() : this.stop(); });
      if ('IntersectionObserver' in window) new IntersectionObserver(es => { this.onscreen = es[0].isIntersecting; this.onscreen ? this.invalidate() : this.stop(); }).observe(canvas);
      this.bindInput();
    }

    // data
    setData(model) {
      const t0 = performance.now();
      this.model = model;
      this.sessions = new Map(model.sessions.map(s => [s.id, s]));
      this.apps = new Map(model.apps.map(a => [a.id, a]));
      this.accounts = new Map(model.accounts.map(a => [a.id, a]));
      const floorsBefore = this.layout.floors.length, baysBefore = this.layout.appBays.size;
      seatSessions(this.layout, model);
      if (this.layout.floors.length !== floorsBefore || this.layout.appBays.size !== baysBefore) this.opts.onFloors && this.opts.onFloors();
      this.ensureStaff();
      this.stats.buildMs = performance.now() - t0;
      this.alert = model.mac && model.mac.hotFor >= (model.mac.alertAfter || 120);
      this.planWalks();
      this.invalidate();
    }
    regenerate(seed, model) {
      if (model) this.model = model;
      this.seed = seed; this.layout = new Layout(seed); this.chars.clear(); this.bots.clear(); this.cardCache.clear();
      this.walkers = []; this.cat = null; this.selected = null;
      if (this.model) this.setData(this.model);
      this.fit(true);
    }
    // Characters are generated once per staff slot (App x desk index) and reused.
    ensureStaff() {
      const t0 = performance.now(); let made = 0;
      for (const [appId, bays] of this.layout.appBays) {
        const app = this.apps.get(appId); if (!app) continue;
        for (const b of bays) for (const d of b.desks) {
          const key = appId + '#' + d.staff;
          if (!this.chars.has(key)) { this.chars.set(key, buildCharacter(hash32(this.seed + '|' + key), app.hue)); made++; }
        }
        if (!this.bots.has(appId)) this.bots.set(appId, buildBot(app.hue));
      }
      if (!this.npc) {
        this.npc = { mail: buildCharacter(hash32(this.seed + 'mail'), 210), runner: buildCharacter(hash32(this.seed + 'runner'), 40), reception: buildCharacter(hash32(this.seed + 'reception'), 250) };
        made += 3;
      }
      if (made) this.stats.charMs = (this.stats.charMs || 0) + (performance.now() - t0);
    }
    charFor(d) { return this.chars.get(d.appId + '#' + d.staff); }
    get floor() { return this.layout.floor(this.floorIx); }
    setFloor(i) { this.floorIx = Math.max(0, Math.min(i, this.layout.floors.length - 1)); this.walkers = this.walkers.filter(w => w.floor === this.floorIx); this.invalidate(); }
    floorSummaries() {
      return this.layout.floors.map(fl => {
        let needs = 0, live = 0;
        for (const b of fl.bays) for (const d of b.desks) { const s = d.sessionId && this.sessions.get(d.sessionId); if (!s) continue; if (s.state !== 'ended') live++; if (NEEDS_YOU.has(s.state)) needs++; }
        return { index: fl.index, needs, live, apps: new Set(fl.bays.filter(b => b.appId).map(b => b.appId)).size };
      });
    }
    setMotion(m) { this.motion = m; if (m === 'off') { this.walkers = []; this.stop(); } this.invalidate(); }
    setNight(n) { if (this.night === n) return; this.night = n; this.layout.floors.forEach(f => f.bake = null); this.invalidate(); }

    // camera
    resize() {
      const r = this.canvas.getBoundingClientRect();
      this.cw = Math.max(1, r.width); this.ch = Math.max(1, r.height);
      this.dpr = Math.max(1, Math.min(3, window.devicePixelRatio || 1));
      this.canvas.width = Math.round(this.cw * this.dpr); this.canvas.height = Math.round(this.ch * this.dpr);
      if (!this.fitted) this.fit(true);
    }
    fitScale() { return Math.min(this.cw / (W * TILE), this.ch / (H * TILE)) * 0.98; }
    fit(now) { this.fitted = true; this.flyTo(W * TILE / 2, H * TILE / 2, this.fitScale(), now); }
    level() { const s = this.cam.s; return s < 2.2 ? 'far' : s < 4.8 ? 'mid' : 'near'; }
    snapScale(s) { const d = s * this.dpr; return d >= 2 ? Math.round(d) / this.dpr : s; }
    flyTo(x, y, s, now) {
      s = this.snapScale(Math.max(this.fitScale() * 0.6, Math.min(10, s)));
      if (now || this.motion === 'off') { this.cam = { x, y, s }; this.tween = null; this.invalidate(); return; }
      this.tween = { from: { ...this.cam }, to: { x, y, s }, t0: performance.now(), dur: 420 };
      this.invalidate();
    }
    zoomLevel(lv) {
      const target = lv === 'far' ? this.fitScale() : lv === 'mid' ? 3 : 6;
      const d = this.selected && this.layout.deskOf.get(this.selected);
      if (d && d.floor === this.floorIx && lv !== 'far') this.flyTo(d.x + 16, d.y + 14, target);
      else this.flyTo(lv === 'far' ? W * TILE / 2 : this.cam.x, lv === 'far' ? H * TILE / 2 : this.cam.y, target);
    }
    focusSession(id, lv = 'mid') {
      const d = this.layout.deskOf.get(id); if (!d) return;
      if (d.floor !== this.floorIx) this.setFloor(d.floor);
      this.selected = id; const s = lv === 'near' ? 6 : 3;
      this.flyTo(d.x + 16, d.y + (lv === 'near' ? 4 : 14), Math.max(this.cam.s, s));
      this.opts.onSelect && this.opts.onSelect({ kind: 'session', id });
    }
    toWorld(sx, sy) { return { x: (sx - this.cw / 2) / this.cam.s + this.cam.x, y: (sy - this.ch / 2) / this.cam.s + this.cam.y }; }
    toScreen(wx, wy) { return { x: (wx - this.cam.x) * this.cam.s + this.cw / 2, y: (wy - this.cam.y) * this.cam.s + this.ch / 2 }; }

    // frame scheduling: draw on demand, tick only while something animated is on screen
    // A tick redraws only the world layer; anything else redraws the overlay too.
    invalidate(worldOnly) { if (!worldOnly) this.overlayNext = true; if (this.raf || !this.visible) return; this.raf = requestAnimationFrame(() => { this.raf = 0; this.draw(); }); }
    stop() { if (this.timer) { clearTimeout(this.timer); this.timer = 0; } this.stats.animating = false; }
    fps() { return this.motion === 'calm' ? 4 : 10; }
    schedule(animNeeded) {
      this.stats.animating = animNeeded;
      if (!animNeeded || this.timer || !this.visible || !this.onscreen || this.motion === 'off') return;
      const dt = 1000 / this.fps(), t0 = performance.now();
      this.timer = setTimeout(() => {
        this.timer = 0; this.stats.ticks++;
        const now = performance.now(); this.step(Math.min(250, now - t0 + 0)); this.invalidate(true);
      }, dt);
    }
    step(dt) {
      this.t += dt;
      const fl = this.floor;
      for (const w of this.walkers) {
        if (w.wait > 0) { w.wait -= dt; continue; }
        const tgt = w.path[w.i]; if (!tgt) { w.done = true; continue; }
        const dx = tgt.x - w.x, dy = tgt.y - w.y, dist = Math.hypot(dx, dy), v = 34 * dt / 1000;
        if (dist <= v) { w.x = tgt.x; w.y = tgt.y; w.i++; if (w.i >= w.path.length) { if (w.back) { w.path = w.back; w.back = null; w.i = 0; w.wait = w.pause || 1800; w.carry = w.carryBack || null; } else w.done = true; } }
        else { w.x += dx / dist * v; w.y += dy / dist * v; w.dir = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'r' : 'l') : (dy > 0 ? 'd' : 'u'); }
      }
      const finished = this.walkers.filter(w => w.done);
      if (finished.length) { this.walkers = this.walkers.filter(w => !w.done); finished.forEach(w => w.onDone && w.onDone()); }
      if (this.cat && fl.index === 0) this.stepCat(dt);
    }

    // ambient life: idle people fetch coffee, the mail carrier pins notices, a runner carries pool hand-offs
    planWalks() {
      if (this.motion !== 'full' || !this.model || !this.floor.grid) return;
      const fl = this.floor;
      if (this.walkers.length < 3 && Math.random() < 0.5) {
        const idle = [];
        for (const b of fl.bays) for (const d of b.desks) { const s = d.sessionId && this.sessions.get(d.sessionId); if (s && s.state === 'idle' && !this.walkers.some(w => w.desk === d)) idle.push(d); }
        if (idle.length) {
          const d = idle[(Math.random() * idle.length) | 0];
          const dest = this.spot(fl, Math.random() < 0.6 ? 'kitchen' : 'cooler');
          if (dest) this.walk(this.charFor(d), d.tx, d.ty + 2, dest.x, dest.y, { desk: d, pause: 2600 });
        }
      }
    }
    spot(fl, kind) {
      if (kind === 'cooler') return { x: 6, y: 20 };
      if (kind === 'cork') return { x: 4, y: 3 };
      const rm = fl.rooms.find(r => r.kind === kind); if (!rm) return null;
      return { x: rm.x + Math.floor(rm.w / 2) - 1, y: rm.y + rm.h - 1 };
    }
    walk(ch, sx, sy, gx, gy, o = {}) {
      if (!ch || this.motion !== 'full') return;
      const fl = this.floor, there = findPath(fl, sx, sy, gx, gy); if (!there || !there.length) return;
      const back = o.oneWay ? null : findPath(fl, gx, gy, sx, sy);
      const start = { x: sx * TILE + 8, y: sy * TILE + 14 };
      this.walkers.push({ ch, x: start.x, y: start.y, path: there, back, i: 0, dir: 'd', floor: fl.index, wait: 0, ...o });
      this.invalidate();
    }
    noticeArrived() {
      if (this.floorIx !== 0 || !this.npc) return;
      this.walk(this.npc.mail, 2, 33, 4, 3, { carry: 'envelope', pause: 1200, mail: true });
    }
    poolHandOff(fromId, toId) {
      const a = this.layout.deskOf.get(fromId), b = this.layout.deskOf.get(toId);
      if (!a || !b || a.floor !== this.floorIx || b.floor !== this.floorIx || !this.npc) return;
      this.walk(this.npc.runner, a.tx, a.ty + 2, b.tx, b.ty + 2, { carry: 'folder', carryBack: null, pause: 900 });
    }
    stepCat(dt) {
      const c = this.cat; c.t -= dt;
      if (c.t > 0) return;
      if (c.sleep) { c.sleep = false; }
      const hot = this.model && this.model.mac && this.model.mac.cpu < 40;
      if (hot && Math.random() < 0.4) { c.sleep = true; c.t = 6000; return; }
      c.tx = OX + 1 + ((Math.random() * (BX * BAY_W - 2)) | 0); c.ty = TOP; c.t = 2500 + Math.random() * 4000;
    }

    // drawing
    draw() {
      const t0 = performance.now();
      const fl = this.floor, dpr = this.dpr;
      if (this.tween) {
        const k = Math.min(1, (t0 - this.tween.t0) / this.tween.dur), e = 1 - Math.pow(1 - k, 3), a = this.tween.from, b = this.tween.to;
        this.cam = { x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, s: a.s + (b.s - a.s) * e };
        if (k >= 1) { this.tween = null; this.cam.s = b.s; }
        this.overlayNext = true;
      }
      if (!fl.bake || fl.bakeTheme !== this.night) {
        const b0 = performance.now(); fl.bake = bakeFloor(this.layout, fl, this.model, this.night); fl.bakeTheme = this.night; this.stats.bakeMs = performance.now() - b0;
        if (fl.index === 0 && !this.cat) this.cat = { x: (OX + 4) * TILE, y: TOP * TILE + 10, tx: OX + 4, ty: TOP, t: 3000, sleep: false };
      }
      const lv = this.level();
      const anim = this.drawWorld(fl, lv);
      if (this.overlayNext) {
        this.overlayNext = false;
        const ov = this.ctx; ov.setTransform(1, 0, 0, 1, 0, 0); ov.clearRect(0, 0, this.canvas.width, this.canvas.height);
        ov.setTransform(dpr, 0, 0, dpr, 0, 0);
        this.drawOverlay(ov, fl, lv);
        this.stats.overlayFrames = (this.stats.overlayFrames || 0) + 1;
      }
      this.syncAlert();
      const ms = performance.now() - t0;
      this.stats.frames++; this.stats.drawMs += ms; this.stats.lastMs = ms;
      const now = performance.now(); this.stats.window.push([now, ms]);
      while (this.stats.window.length && this.stats.window[0][0] < now - 1000) this.stats.window.shift();
      if (this.tween) this.invalidate();
      this.schedule((anim || this.walkers.length > 0) && lv !== 'far');
      this.opts.onFrame && this.opts.onFrame(this.stats);
    }

    // The world layer. Zoomed in (2+ device pixels per art pixel) its bitmap holds one pixel per art
    // pixel and CSS scales it with image-rendering: pixelated; the sub-pixel part of the camera is a
    // CSS translate, so panning stays smooth. Zoomed out it is drawn at device resolution.
    drawWorld(fl, lv) {
      const wc = this.world, ctx = this.wctx, dpr = this.dpr, s = this.cam.s, k = s * dpr;
      const vx0 = this.cam.x - this.cw / 2 / s, vy0 = this.cam.y - this.ch / 2 / s;
      const crisp = k >= 2;
      let bw, bh, css;
      if (crisp) {
        const x0 = Math.floor(vx0), y0 = Math.floor(vy0);
        bw = Math.ceil(this.cw / s) + 2; bh = Math.ceil(this.ch / s) + 2;
        css = `${bw * s}px|${bh * s}px|translate(${(-(vx0 - x0) * s).toFixed(2)}px,${(-(vy0 - y0) * s).toFixed(2)}px)|pixelated`;
        if (wc.width !== bw || wc.height !== bh) { wc.width = bw; wc.height = bh; }
        ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.fillStyle = this.night ? '#0b0e14' : '#cfd3dc'; ctx.fillRect(0, 0, bw, bh);
        ctx.setTransform(1, 0, 0, 1, -x0, -y0);
      } else {
        bw = Math.round(this.cw * dpr); bh = Math.round(this.ch * dpr);
        css = `${this.cw}px|${this.ch}px|none|auto`;
        if (wc.width !== bw || wc.height !== bh) { wc.width = bw; wc.height = bh; }
        ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.fillStyle = this.night ? '#0b0e14' : '#cfd3dc'; ctx.fillRect(0, 0, bw, bh);
        ctx.setTransform(k, 0, 0, k, Math.round(-vx0 * k), Math.round(-vy0 * k));
      }
      if (css !== this.worldCss) {
        const [w, h, tr, ir] = css.split('|'); this.worldCss = css;
        Object.assign(wc.style, { width: w, height: h, transform: tr, imageRendering: ir });
      }
      ctx.imageSmoothingEnabled = !crisp && k < 1.6;
      ctx.drawImage(fl.bake, 0, 0);
      ctx.imageSmoothingEnabled = false;
      const view = { x0: vx0 - 40, y0: vy0 - 40, x1: vx0 + this.cw / s + 40, y1: vy0 + this.ch / s + 40 };
      let anim = false;
      const night = this.night;

      // zones and room content (dynamic, data-driven)
      anim = this.drawRooms(ctx, fl, view, lv) || anim;
      const items = [];
      for (const b of fl.bays) {
        if (!b.appId) continue;
        const app = this.apps.get(b.appId), acct = app && this.accounts.get(app.accountId);
        const decay = app ? decayOf(app, acct) : 0;
        const x0 = (OX + b.bx * BAY_W) * TILE, y0 = (OY + b.by * BAY_H) * TILE;
        if (x0 > view.x1 || x0 + BAY_W * TILE < view.x0 || y0 > view.y1 || y0 + BAY_H * TILE < view.y0) continue;
        if (decay >= 3) {
          const r = rngFor(this.seed, 'trash', b.i);
          for (let k = 0; k < 4; k++) ctx.drawImage(r() < 0.6 ? SPR.trash : SPR.box, x0 + 18 + ((r() * 90) | 0), y0 + 4 + ((r() * 14) | 0));
        }
        const first = (this.layout.appBays.get(b.appId) || [])[0] === b;
        if (first) this.drawZoneBoard(ctx, b, app, x0, y0);
        for (const d of b.desks) items.push({ y: d.y + 30, d, app, decay });
      }
      for (const w of this.walkers) if (w.floor === fl.index) items.push({ y: w.y, w });
      items.sort((a, b) => a.y - b.y);
      for (const it of items) {
        if (it.w) { this.drawWalker(ctx, it.w); anim = true; continue; }
        if (it.d.x > view.x1 || it.d.x + 32 < view.x0 || it.d.y > view.y1 || it.d.y + 36 < view.y0) continue;
        if (this.drawDesk(ctx, it.d, it.app, it.decay, lv)) anim = true;
      }
      // receptionist and the office cat
      if (fl.index === 0 && this.npc) {
        ctx.drawImage(this.npc.reception.atlas, F.STAND * FW, 0, FW, FH, 4 * TILE + 1, 5 * TILE - 13, FW, FH);
        ctx.fillStyle = '#e9e7e1'; ctx.fillRect(2 * TILE, 5 * TILE, 5 * TILE, 10); ctx.fillStyle = '#fff'; ctx.fillRect(2 * TILE, 5 * TILE, 5 * TILE, 2);
        ctx.fillStyle = '#9aa3b2'; ctx.fillRect(2 * TILE, 5 * TILE + 10, 5 * TILE, 8);
        if (this.cat) {
          const c = this.cat; const gx = c.tx * TILE, gy = c.ty * TILE + 6;
          if (!c.sleep && this.motion === 'full') { const v = 18 * 0.1; c.x += Math.sign(gx - c.x) * Math.min(Math.abs(gx - c.x), v * 4); c.y += Math.sign(gy - c.y) * Math.min(Math.abs(gy - c.y), v * 4); }
          const moving = Math.abs(gx - c.x) > 1 && !c.sleep;
          const f = c.sleep ? 2 : moving ? ((this.t / 200) | 0) % 2 : 0;
          ctx.save(); if (gx < c.x) { ctx.translate(Math.round(c.x) + 12, 0); ctx.scale(-1, 1); ctx.drawImage(SPR.cat[f], 0, Math.round(c.y)); } else ctx.drawImage(SPR.cat[f], Math.round(c.x), Math.round(c.y)); ctx.restore();
          if (moving && lv !== 'far') anim = true;
        }
      }
      // decay lighting: long-unused zones have their lights off
      for (const b of fl.bays) {
        if (!b.appId) continue;
        const app = this.apps.get(b.appId), d = app ? decayOf(app, this.accounts.get(app.accountId)) : 0;
        if (d >= 3) { ctx.fillStyle = 'rgba(10,12,20,.38)'; ctx.fillRect((OX + b.bx * BAY_W) * TILE, (OY + b.by * BAY_H) * TILE, BAY_W * TILE, BAY_H * TILE); }
      }
      if (night) this.drawLamps(ctx, fl, view);
      // red alert: beacons on the walls; the breathing red wash is a CSS layer (see syncAlert)
      if (this.alert) for (let x = 40; x < W * TILE; x += 150) ctx.drawImage(SPR.beacon[0], x, 2);
      this.stats.worldPixels = bw * bh;
      return anim;
    }
    // Red alert breathes at 0.5 Hz as a CSS animation on its own layer (no canvas work, no flashing).
    syncAlert() {
      const on = !!this.alert, moving = on && this.motion === 'full';
      const key = on + '|' + moving;
      if (key === this.alertKey) return; this.alertKey = key;
      this.alertEl.style.display = on ? 'block' : 'none';
      this.alertEl.style.animation = moving ? 'floorBreathe 1s ease-in-out infinite alternate' : 'none';
    }

    drawRooms(ctx, fl, view, lv) {
      let anim = false; const m = this.model || {};
      for (const rm of fl.rooms) {
        const x0 = rm.x * TILE, y0 = rm.y * TILE;
        if (rm.kind === 'server') {
          const cpu = m.mac ? m.mac.cpu : 20, hot = cpu >= 85;
          for (let i = 0; i < 3; i++) {
            const rx = x0 + 12 + i * 20; const n = Math.round((cpu / 100) * 16);
            for (let k = 0; k < 16; k++) {
              const on = k < n; const blink = on && this.motion === 'full' && ((this.t / 300 + k * 7 + i * 3) | 0) % 5 === 0;
              ctx.fillStyle = !on ? '#30364a' : hot ? (blink ? '#ffb3b3' : '#ff5a5a') : blink ? '#b8ffe0' : '#34d399';
              ctx.fillRect(rx + 1 + (k % 2) * 6, y0 + 38 - (k >> 1) * 4, 4, 2);
            }
          }
          if (this.motion === 'full' && lv !== 'far')
          if (hot && this.motion === 'full') { const f = ((this.t / 300) | 0) % 2; ctx.fillStyle = 'rgba(255,170,120,.35)'; for (let i = 0; i < 3; i++) ctx.fillRect(x0 + 16 + i * 20 + f * 2, y0 - 4 - f * 2, 2, 5); anim = true; }
          else if (this.motion === 'full' && lv !== 'far') anim = true;
        }
        if (rm.kind === 'meeting' && m.pools) {
          const meetIx = fl.rooms.filter(r => r.kind === 'meeting').indexOf(rm) + fl.index * 2;
          const pool = m.pools[meetIx]; rm.ref = pool ? pool.id : null;
          if (pool) {
            // members' chairs take their App's colour; the wall screen shows the pool's step
            pool.members.forEach((sid, k) => {
              const s = this.sessions.get(sid), a = s && this.apps.get(s.appId); if (!a) return;
              const cx = x0 + 30 + (k % 4) * 18, cy = k < 4 ? y0 + 32 : y0 + 72;
              ctx.fillStyle = hslHex(a.hue, 55, 50); ctx.fillRect(cx, cy, 8, 6);
            });
            ctx.fillStyle = '#7cd4ff'; const prog = (pool.step || 0) % 5; for (let k = 0; k < 5; k++) { ctx.fillStyle = k <= prog ? '#7cd4ff' : '#2f4a63'; ctx.fillRect(x0 + 60 + k * 11, 8 * 0 + 14 - 2 * TILE + 2 * TILE, 8, 4); }
          }
        }
        if (rm.kind === 'kitchen' && this.motion === 'full' && lv !== 'far') { ctx.drawImage(SPR.steam[((this.t / 400) | 0) % 2], x0 + 33, y0 - 10); anim = true; }
        if (rm.kind === 'archive') {
          const n = Math.min(9, Math.ceil((m.stats ? m.stats.endedToday : 0) / 5));
          for (let i = 0; i < n; i++) ctx.drawImage(SPR.box, x0 + 8 + (i % 5) * 20, y0 + 92 - ((i / 5) | 0) * 9);
        }
        if (rm.kind === 'boss') {
          const n = Math.min(9, m.stats ? Math.ceil(m.stats.doneToday / 4) : 0);
          for (let i = 0; i < n; i++) { ctx.fillStyle = i % 3 === 0 ? '#e8b04a' : i % 3 === 1 ? '#c9cfdb' : '#c07a3a'; ctx.fillRect(x0 + 94 + (i % 3) * 4, y0 + 7 + ((i / 3) | 0) * 10, 3, 5); }
        }
      }
      if (fl.index === 0) {
        // corkboard: one pinned note per notice, pin colour and shape by severity
        const notes = (m.notices || []).slice(0, 8);
        notes.forEach((n, i) => {
          const nx = 3 * TILE + 12 + (i % 4) * 10, ny = 8 + ((i / 4) | 0) * 8;
          ctx.fillStyle = n.sev === 'error' ? '#ffd0cc' : n.sev === 'warn' ? '#fff1b8' : '#f2efe6'; ctx.fillRect(nx, ny, 8, 7);
          ctx.fillStyle = '#c9c3b5'; ctx.fillRect(nx + 1, ny + 3, 6, 1);
          ctx.fillStyle = n.sev === 'error' ? '#d93b3b' : n.sev === 'warn' ? '#d99a1b' : '#3d6fb0'; ctx.fillRect(nx + 3, ny, 2, 2);
        });
        // clock wall: one clock per Account, the hand points at the time left to its reset
        (m.accounts || []).slice(0, 4).forEach((a, i) => {
          const cx = 7 * TILE + 8 + i * 9, cy = 14;
          ctx.fillStyle = a.limit ? '#ffd9d0' : '#fff'; ctx.beginPath(); ctx.arc(cx, cy, 4, 0, 7); ctx.fill();
          ctx.strokeStyle = '#333'; ctx.lineWidth = 1; ctx.stroke();
          const ang = -Math.PI / 2 + ((a.resetIn || 0) / (5 * 3600000)) * Math.PI * 2;
          ctx.strokeStyle = a.limit ? '#d93b3b' : '#222'; ctx.beginPath(); ctx.moveTo(cx, cy); ctx.lineTo(cx + Math.cos(ang) * 3, cy + Math.sin(ang) * 3); ctx.stroke();
          ctx.fillStyle = hslHex((this.apps.size && [...this.apps.values()].find(p => p.accountId === a.id) || { hue: 220 }).hue, 40, 45); ctx.fillRect(cx - 3, cy + 6, 6, 2);
        });
      }
      // elevator floor number
      ctx.fillStyle = '#ffb347'; ctx.font = FONT_PX; ctx.textBaseline = 'top'; ctx.fillText(String(fl.index + 1), 1 * TILE + 17, 0);
      return anim;
    }

    drawZoneBoard(ctx, b, app, x0, y0) {
      // A whiteboard in the zone's head strip: sticky notes for the App's tasks (todo, doing, done).
      const tasks = { todo: 0, doing: 0, done: 0 };
      for (const s of this.model.sessions) if (s.appId === app.id && s.tasks) for (const t of s.tasks) tasks[t.done ? 'done' : t.doing ? 'doing' : 'todo']++;
      const wx = x0 + 34, wy = y0 + 2;
      ctx.fillStyle = '#8d94a3'; ctx.fillRect(wx - 2, wy - 2, 52, 26); ctx.fillStyle = '#f7f8fb'; ctx.fillRect(wx, wy, 48, 20);
      ctx.fillStyle = '#d5d9e2'; ctx.fillRect(wx + 16, wy + 1, 1, 18); ctx.fillRect(wx + 32, wy + 1, 1, 18);
      const col = (n, cx, c) => { for (let i = 0; i < Math.min(n, 8); i++) { ctx.fillStyle = c; ctx.fillRect(wx + cx + (i % 2) * 6, wy + 2 + ((i / 2) | 0) * 4, 5, 3); } };
      col(tasks.todo, 2, '#ffe28a'); col(tasks.doing, 18, '#8fd3ff'); col(tasks.done, 34, '#9be8b5');
      ctx.fillStyle = '#555'; ctx.fillRect(wx + 4, wy + 22, 2, 8); ctx.fillRect(wx + 42, wy + 22, 2, 8);
      // hanging zone sign: the App's mark colour
      ctx.fillStyle = '#2a2a35'; ctx.fillRect(x0 + 10, y0 + 2, 1, 6); ctx.fillRect(x0 + 24, y0 + 2, 1, 6);
      ctx.fillStyle = hslHex(app.hue, 55, 48); ctx.fillRect(x0 + 6, y0 + 8, 24, 10); ctx.fillStyle = hslHex(app.hue, 60, 62); ctx.fillRect(x0 + 6, y0 + 8, 24, 2);
      ctx.fillStyle = '#fff'; ctx.font = FONT_PX; ctx.textBaseline = 'top'; ctx.fillText((app.short || app.name).slice(0, 3).toUpperCase(), x0 + 9, y0 + 9);
    }

    drawDesk(ctx, d, app, decay, lv) {
      const s = d.sessionId ? this.sessions.get(d.sessionId) : null;
      const st = s ? s.state : null; const x = d.x, y = d.y; let anim = false;
      const ch = this.charFor(d); const full = this.motion === 'full', calm = this.motion === 'calm';
      const away = this.walkers.some(w => w.desk === d);
      const phase = hash32(d.id) % 7;
      const tick = (ms) => full || calm ? ((this.t / ms + phase) | 0) : 0;
      // monitor
      const live = s && st !== 'ended' && decay === 0;
      ctx.fillStyle = P.black; ctx.fillRect(x + 10, y - 4, 12, 9); ctx.fillRect(x + 15, y + 5, 2, 2); ctx.fillRect(x + 13, y + 7, 6, 1);
      let scr = '#1d2230';
      if (live) scr = st === 'error' ? '#5a1d24' : st === 'done' ? '#1d4a37' : st === 'limit' ? '#3a2a14' : st === 'asks' ? '#4a3a12' : st === 'idle' ? '#232a3c' : '#16314a';
      ctx.fillStyle = scr; ctx.fillRect(x + 11, y - 3, 10, 7);
      if (live && st === 'working') { const k = tick(380) % 3; ctx.fillStyle = '#7cd4ff'; for (let i = 0; i < 3; i++) ctx.fillRect(x + 12 + ((i + k) % 2) * 2, y - 2 + i * 2, 4 + ((i * 3 + k) % 4), 1); anim = true; }
      else if (live && st === 'error') { ctx.fillStyle = '#ff6b6b'; ctx.fillRect(x + 14, y - 2, 1, 1); ctx.fillRect(x + 15, y - 1, 2, 2); ctx.fillRect(x + 17, y, 1, 1); ctx.fillRect(x + 17, y - 2, 1, 1); ctx.fillRect(x + 14, y, 1, 1); }
      else if (live && st === 'done') { ctx.fillStyle = '#5ef0a8'; ctx.fillRect(x + 13, y, 2, 1); ctx.fillRect(x + 15, y + 1, 1, 1); ctx.fillRect(x + 16, y - 1, 2, 1); ctx.fillRect(x + 18, y - 2, 1, 1); }
      else if (live && st === 'asks') { ctx.fillStyle = '#ffd36b'; ctx.fillRect(x + 13, y - 2, 6, 3); ctx.fillStyle = '#4a3a12'; ctx.fillRect(x + 14, y - 1, 4, 1); }
      else if (live && st === 'idle') { ctx.fillStyle = '#3a4560'; ctx.fillRect(x + 12 + (tick(1600) % 6), y - 1, 2, 2); }
      // decay on the desk
      if (decay >= 1 || st === 'ended' || !s) {
        const r = rngFor(this.seed, 'mess', d.id);
        if (decay >= 1 && r() < 0.7) ctx.drawImage(SPR.mug, x + 24, y + 3);
        if (decay >= 1 && r() < 0.6) ctx.drawImage(SPR.papers, x + 2, y + 5);
        if (decay >= 2) { ctx.drawImage(SPR.dust[hash32(d.id) % 3], x + 1, y + 3); ctx.drawImage(SPR.cobweb, x + 21, y - 5); }
        if (decay >= 3 && r() < 0.6) ctx.drawImage(SPR.pizza, x + 3, y + 6);
      }
      if (live && st === 'done') ctx.drawImage(SPR.review, x + 23, y + 4);
      if (s && s.cpuHot && live) { ctx.drawImage(SPR.fire[full ? tick(140) % 3 : 0], x + 22, y - 6); if (full) { ctx.drawImage(SPR.smoke, x + 22 + (tick(300) % 2), y - 14 - (tick(300) % 3)); anim = true; } }
      // chair and person
      const cx = x + 10, cy = y + 21, chairC = CHAIR_COLORS[d.chair];
      const chairBack = (ox, oy) => { ctx.fillStyle = shade(chairC, 0.7); ctx.fillRect(ox + 1, oy + 7, 10, 2); ctx.fillStyle = chairC; ctx.fillRect(ox + 2, oy, 8, 7); ctx.fillStyle = shade(chairC, 1.25); ctx.fillRect(ox + 2, oy, 8, 1); ctx.fillStyle = P.black; ctx.fillRect(ox + 5, oy + 9, 2, 2); ctx.fillRect(ox + 2, oy + 11, 8, 1); };
      const sitting = live && !away && (st === 'working' || st === 'idle' || st === 'stuck' || st === 'limit' || st === 'done');
      const standing = live && !away && (st === 'asks' || st === 'error');
      const drawP = (f, px, py) => ctx.drawImage(ch.atlas, f * FW, 0, FW, FH, px, py, FW, FH);
      if (sitting && ch) {
        let f = F.SITIDLE;
        if (st === 'working') { f = tick(220) % 2 ? F.SIT1 : F.SIT0; anim = true; }
        if (st === 'stuck') f = F.SCRATCH; if (st === 'limit') f = F.SLEEP; if (st === 'done') f = F.LEAN;
        drawP(f, x + 9, y + 8);
        chairBack(cx, cy);
        if (st === 'limit') { ctx.drawImage(SPR.zz, x + 22, y - 2 - (tick(700) % 2)); if (full) anim = true; }
      } else if (standing && ch) {
        chairBack(cx + 9, cy + 2);
        const f = st === 'asks' ? (tick(320) % 2 ? F.WAVE1 : F.WAVE0) : F.ALARM;
        drawP(f, x + 6, y + 12);
        if (full || calm) anim = true;
        if (st === 'error' && full) { ctx.drawImage(SPR.smoke, x + 12, y - 12 - (tick(300) % 3)); }
      } else {
        // empty desk: tucked in when ended or free, pushed out askew when the App was left running away
        if (decay >= 1) { ctx.save(); ctx.translate(cx + 14, cy + 6); ctx.rotate(0.35); ctx.translate(-6, -6); chairBack(0, 0); ctx.restore(); }
        else chairBack(cx, cy - 2);
      }
      // subagents: up to three helper bots by the desk
      const subs = s && live ? (s.subagents || []).filter(a => a.state !== 'done') : [];
      if (subs.length && lv !== 'far') {
        const bot = this.bots.get(d.appId);
        const spots = [[x - 6, y + 18], [x + 28, y + 18], [x - 6, y + 6]];
        subs.slice(0, 3).forEach((a, k) => {
          const f = a.state === 'waiting' ? 2 : (full ? (tick(500 + k * 90) % 2) : 0);
          ctx.drawImage(bot, f * 10, 0, 10, 11, spots[k][0], spots[k][1], 10, 11);
        });
        if (full) anim = true;
      }
      return anim && lv !== 'far';
    }

    drawWalker(ctx, w) {
      const moving = w.wait <= 0 && w.i < (w.path ? w.path.length : 0);
      const step = ((this.t / 180) | 0) % 2;
      let f = F.STAND;
      if (moving) f = w.dir === 'u' ? (step ? F.UP1 : F.UP0) : w.dir === 'r' ? (step ? F.SIDE1 : F.SIDE0) : w.dir === 'l' ? (step ? F.LSIDE1 : F.LSIDE0) : (step ? F.DOWN1 : F.DOWN0);
      ctx.drawImage(w.ch.atlas, f * FW, 0, FW, FH, Math.round(w.x - 7), Math.round(w.y - 20), FW, FH);
      if (w.carry) ctx.drawImage(SPR[w.carry], Math.round(w.x + 2), Math.round(w.y - 10));
    }

    drawLamps(ctx, fl, view) {
      // Evening: lit desks glow; abandoned ones stay dark.
      if (!this.glow) {
        const g = mk(64, 48), gc = g.getContext('2d'), grad = gc.createRadialGradient(32, 24, 2, 32, 24, 30);
        grad.addColorStop(0, 'rgba(255,214,150,.42)'); grad.addColorStop(1, 'rgba(255,214,150,0)'); gc.fillStyle = grad; gc.fillRect(0, 0, 64, 48); this.glow = g;
      }
      ctx.globalCompositeOperation = 'lighter';
      for (const b of fl.bays) for (const d of b.desks) {
        const s = d.sessionId && this.sessions.get(d.sessionId); if (!s || s.state === 'ended') continue;
        const app = this.apps.get(d.appId); if (!app || decayOf(app, this.accounts.get(app.accountId)) > 0) continue;
        if (d.x > view.x1 || d.x + 32 < view.x0 || d.y > view.y1 || d.y + 36 < view.y0) continue;
        ctx.drawImage(this.glow, d.x - 16, d.y - 14);
      }
      ctx.globalCompositeOperation = 'source-over';
    }

    // Screen space: fixed-size things that must stay readable at every zoom.
    drawOverlay(ctx, fl, lv, view) {
      const sc = this.cam.s; let anim = false;
      const needs = [];
      for (const b of fl.bays) for (const d of b.desks) {
        const s = d.sessionId && this.sessions.get(d.sessionId); if (!s) continue;
        const app = this.apps.get(d.appId); if (!app || decayOf(app, this.accounts.get(app.accountId)) > 0) continue;
        const p = this.toScreen(d.x + 16, d.y - 6);
        const on = p.x > -10 && p.y > -10 && p.x < this.cw + 10 && p.y < this.ch + 10;
        if (NEEDS_YOU.has(s.state)) needs.push({ d, s, p, on });
        if (!on) continue;
        if (lv === 'far' && s.state !== 'ended') this.badge(ctx, p.x, p.y - 2, s);
      }
      // zone and room labels at far and mid
      if (lv !== 'near') {
        for (const [appId, bays] of this.layout.appBays) {
          const b = bays[0]; if (!b || !fl.bays.includes(b)) continue;
          const app = this.apps.get(appId); if (!app) continue;
          const p = this.toScreen((OX + b.bx * BAY_W) * TILE + 4, (OY + b.by * BAY_H) * TILE + 2);
          this.zoneLabel(ctx, p.x, p.y, app, bays, lv);
        }
        for (const rm of fl.rooms) {
          const p = this.toScreen((rm.x + rm.w / 2) * TILE, (rm.y + rm.h) * TILE - 6);
          this.pill(ctx, p.x, p.y, this.roomLabel(rm), null, true);
        }
        if (fl.index === 0) {
          const m = this.model || {}, n = (m.notices || []).length;
          const p = this.toScreen(4.6 * TILE, 30); this.pill(ctx, p.x, p.y + 6, n ? `${n} notices` : 'No notices', null, true);
        }
      }
      // near: a monitor card per visible desk (cached bitmaps, redrawn only when the session changes)
      if (lv === 'near') {
        for (const b of fl.bays) for (const d of b.desks) {
          const s = d.sessionId && this.sessions.get(d.sessionId); if (!s || s.state === 'ended') continue;
          const app = this.apps.get(d.appId);
          const p = this.toScreen(d.x + 16, d.y - 4); if (p.x < -200 || p.x > this.cw + 200 || p.y < -10 || p.y > this.ch + 260) continue;
          const card = this.card(d, s, app);
          ctx.drawImage(card, Math.round(p.x - card.width / this.dpr / 2), Math.round(p.y - card.height / this.dpr - 8), card.width / this.dpr, card.height / this.dpr);
        }
      }
      // needs-you bubbles: same size at every zoom, never smaller than readable
      for (const n of needs) {
        if (!n.on) continue;
        if (lv === 'far') continue;
        this.bubble(ctx, n.p.x, n.p.y - (lv === 'near' ? 4 : 10 * sc / 4), n.s.state);
      }
      // selection
      const sel = this.selected && this.layout.deskOf.get(this.selected);
      if (sel && sel.floor === fl.index) {
        const a = this.toScreen(sel.x - 2, sel.y - 8), b2 = this.toScreen(sel.x + 34, sel.y + 34);
        ctx.strokeStyle = '#a9a9ff'; ctx.lineWidth = 2; roundRect(ctx, a.x, a.y, b2.x - a.x, b2.y - a.y, 6); ctx.stroke();
      }
      if (this.hover && this.hover.kind === 'desk' && this.hover.d !== sel) {
        const d = this.hover.d, a = this.toScreen(d.x - 2, d.y - 8), b2 = this.toScreen(d.x + 34, d.y + 34);
        ctx.strokeStyle = 'rgba(169,169,255,.55)'; ctx.lineWidth = 1.5; roundRect(ctx, a.x, a.y, b2.x - a.x, b2.y - a.y, 6); ctx.stroke();
      }
      // edge arrows toward anyone off screen who needs you
      for (const n of needs) {
        if (n.on) continue;
        const cx = this.cw / 2, cy = this.ch / 2, dx = n.p.x - cx, dy = n.p.y - cy;
        const k = Math.min((this.cw / 2 - 22) / Math.abs(dx || 1e-6), (this.ch / 2 - 22) / Math.abs(dy || 1e-6));
        const ex = cx + dx * k, ey = cy + dy * k, ang = Math.atan2(dy, dx);
        ctx.save(); ctx.translate(ex, ey);
        ctx.fillStyle = 'rgba(16,19,30,.92)'; ctx.beginPath(); ctx.arc(0, 0, 13, 0, 7); ctx.fill();
        ctx.strokeStyle = STATES[n.s.state].color; ctx.lineWidth = 1.5; ctx.stroke();
        glyph(ctx, n.s.state, 0, 0, 12, STATES[n.s.state].color);
        ctx.rotate(ang); ctx.fillStyle = STATES[n.s.state].color; ctx.beginPath(); ctx.moveTo(17, 0); ctx.lineTo(12, -4); ctx.lineTo(12, 4); ctx.fill();
        ctx.restore();
      }
      if (this.alert) {
        const m = this.model.mac, text = `Red alert · This Mac at ${Math.round(m.cpu)}% for ${Math.round(m.hotFor / 60)} min`;
        ctx.font = FONT; const w = ctx.measureText(text).width + 34;
        ctx.fillStyle = 'rgba(80,10,14,.92)'; roundRect(ctx, this.cw / 2 - w / 2, 10, w, 26, 13); ctx.fill();
        ctx.strokeStyle = '#ff7a7a'; ctx.lineWidth = 1; ctx.stroke();
        glyph(ctx, 'fire', this.cw / 2 - w / 2 + 15, 23, 12, '#ff9a7a');
        ctx.fillStyle = '#ffe4e4'; ctx.textBaseline = 'middle'; ctx.fillText(text, this.cw / 2 - w / 2 + 26, 23.5);
      }
      return anim;
    }
    roomLabel(rm) {
      const m = this.model || {};
      if (rm.kind === 'server') return `This Mac · ${Math.round(m.mac ? m.mac.cpu : 0)}% CPU`;
      if (rm.kind === 'meeting') { const p = (m.pools || []).find(x => x.id === rm.ref); return p ? `${p.name} · ${p.members.length}` : 'Meeting room'; }
      if (rm.kind === 'kitchen') return 'Kitchen';
      if (rm.kind === 'archive') return `Archive · ${m.stats ? m.stats.endedToday : 0} ended today`;
      if (rm.kind === 'boss') return `Your office · ${m.stats ? m.stats.doneToday : 0} done`;
      return { quiet: 'Quiet room', storage: 'Storage', phone: 'Phone booths', lounge: 'Lounge' }[rm.kind] || rm.kind;
    }
    badge(ctx, x, y, s) {
      const st = STATES[s.state]; const subs = (s.subagents || []).filter(a => a.state !== 'done').length;
      const text = subs ? String(subs) : '';
      ctx.font = FONT_S; const tw = text ? ctx.measureText(text).width + 14 : 0;
      const w = 16 + tw, h = 16;
      ctx.fillStyle = 'rgba(16,19,30,.9)'; roundRect(ctx, x - 8, y - h, w, h, 8); ctx.fill();
      ctx.strokeStyle = st.color; ctx.lineWidth = NEEDS_YOU.has(s.state) ? 1.6 : 0.8; ctx.stroke();
      glyph(ctx, s.state, x, y - 8, 10, st.color);
      if (text) { glyph(ctx, 'bot', x + 12, y - 8, 9, '#c7cde0'); ctx.fillStyle = '#eef1f8'; ctx.textBaseline = 'middle'; ctx.fillText(text, x + 17, y - 7.5); }
    }
    bubble(ctx, x, y, state) {
      const st = STATES[state]; const words = { asks: 'Asks you', error: 'Error', limit: 'Limit', stuck: 'Stuck?' }[state];
      ctx.font = FONT; const w = ctx.measureText(words).width + 30, h = 22;
      ctx.fillStyle = '#fffaf0'; roundRect(ctx, x - w / 2, y - h - 6, w, h, 11); ctx.fill();
      ctx.beginPath(); ctx.moveTo(x - 5, y - 7); ctx.lineTo(x, y); ctx.lineTo(x + 5, y - 7); ctx.fill();
      ctx.strokeStyle = st.color; ctx.lineWidth = 2; roundRect(ctx, x - w / 2, y - h - 6, w, h, 11); ctx.stroke();
      glyph(ctx, state, x - w / 2 + 13, y - h / 2 - 6, 12, state === 'asks' ? '#b7791f' : state === 'error' ? '#c53030' : state === 'limit' ? '#c05621' : '#b7791f');
      ctx.fillStyle = '#1d1726'; ctx.textBaseline = 'middle'; ctx.fillText(words, x - w / 2 + 23, y - h / 2 - 5.5);
    }
    pill(ctx, x, y, text, color, center) {
      ctx.font = FONT_S; const w = ctx.measureText(text).width + 14;
      const px = center ? x - w / 2 : x;
      ctx.fillStyle = 'rgba(16,19,30,.78)'; roundRect(ctx, px, y - 9, w, 18, 9); ctx.fill();
      ctx.fillStyle = color || '#e6e9f2'; ctx.textBaseline = 'middle'; ctx.fillText(text, px + 7, y + 0.5);
    }
    zoneLabel(ctx, x, y, app, bays, lv) {
      // One line (mark, name, how many need you) that fits the zone's width; a second line only when the zone is closed, unused or overloaded.
      const desks = bays.flatMap(b => b.desks);
      let needs = 0;
      for (const d of desks) { const s = d.sessionId && this.sessions.get(d.sessionId); if (s && NEEDS_YOU.has(s.state)) needs++; }
      const acct = this.accounts.get(app.accountId), decay = decayOf(app, acct);
      let sub = decay === 1 ? `Closed ${agoWords(app.lastActiveAgo)} ago` : decay >= 2 ? `Unused ${agoWords(Math.max(app.lastActiveAgo || 0, acct && acct.lastUsedAgo > 30 * DAY ? acct.lastUsedAgo : 0))}` : '';
      if (app.overloaded) sub = 'Overloaded';
      const maxW = Math.max(60, BAY_W * TILE * this.cam.s - 10);
      const key = [app.name, sub, needs, Math.round(maxW), this.dpr].join('|');
      let bmp = this.labelCache.get(app.id);
      if (!bmp || bmp.key !== key) {
        const dpr = this.dpr, c = mk(1, 1), cx = c.getContext('2d');
        const chip = needs ? 22 : 0;
        cx.font = FONT; const nameW = Math.min(cx.measureText(app.name).width, maxW - 30 - chip);
        cx.font = FONT_S; const subW = sub ? Math.min(cx.measureText(sub).width, maxW - 30) : 0;
        const w = Math.ceil(Math.max(nameW + chip, subW) + 30), h = sub ? 32 : 20;
        c.width = w * dpr; c.height = h * dpr; cx.scale(dpr, dpr);
        cx.fillStyle = decay ? 'rgba(30,32,40,.86)' : 'rgba(16,19,30,.9)'; roundRect(cx, 0, 0, w, h, 6); cx.fill();
        cx.fillStyle = hslHex(app.hue, 60, 55); roundRect(cx, 5, 4, 12, 12, 3); cx.fill();
        cx.fillStyle = '#fff'; cx.font = '700 8px ui-sans-serif, system-ui, sans-serif'; cx.textBaseline = 'middle'; cx.textAlign = 'center'; cx.fillText(app.name[0], 11, 10.5); cx.textAlign = 'left';
        cx.font = FONT; cx.fillStyle = decay ? '#b9c0cc' : '#f3f5f9'; cx.fillText(clip(cx, app.name, nameW + 1), 22, 10.5);
        if (needs) { const nx = 22 + nameW + 5; cx.fillStyle = '#fbbf24'; roundRect(cx, nx, 4, 18, 12, 6); cx.fill(); cx.fillStyle = '#1d1726'; cx.font = '700 9px ui-sans-serif, system-ui, sans-serif'; cx.textAlign = 'center'; cx.fillText(String(needs), nx + 9, 10.5); cx.textAlign = 'left'; }
        if (sub) { cx.font = FONT_S; cx.fillStyle = app.overloaded ? '#ffb4a8' : '#aab3c2'; cx.fillText(clip(cx, sub, maxW - 30), 22, 24); }
        bmp = { key, c, w, h }; this.labelCache.set(app.id, bmp);
      }
      ctx.drawImage(bmp.c, Math.round(x), Math.round(y), bmp.w, bmp.h);
    }
    card(d, s, app) {
      const key = [s.id, s.state, s.title, (s.subagents || []).map(a => a.state + a.task).join(','), (s.tasks || []).map(t => (t.done ? 1 : t.doing ? 2 : 0)).join(''), this.dpr].join('|');
      const hit = this.cardCache.get(d.id); if (hit && hit.key === key) return hit.c;
      const dpr = this.dpr, w = 190;
      const subs = s.subagents || [], tasks = s.tasks || [];
      const lines = 2 + Math.min(4, tasks.length) + (subs.length ? 1 + Math.min(4, subs.length) : 0);
      const h = 14 + lines * 15;
      const c = mk(w * dpr, h * dpr), cx = c.getContext('2d'); cx.scale(dpr, dpr);
      cx.fillStyle = 'rgba(14,17,27,.95)'; roundRect(cx, 0, 0, w, h, 8); cx.fill();
      cx.strokeStyle = STATES[s.state].color; cx.lineWidth = 1; roundRect(cx, 0.5, 0.5, w - 1, h - 1, 8); cx.stroke();
      let y = 15; cx.textBaseline = 'middle';
      cx.font = FONT; cx.fillStyle = '#f3f5f9'; cx.fillText(clip(cx, s.title, w - 16), 8, y); y += 15;
      glyph(cx, s.state, 13, y, 10, STATES[s.state].color); cx.font = FONT_S; cx.fillStyle = '#b0b8c5'; cx.fillText(clip(cx, `${STATES[s.state].word} · ${app.name}`, w - 30), 22, y); y += 15;
      for (const t of tasks.slice(0, 4)) {
        cx.strokeStyle = '#8e97a6'; cx.lineWidth = 1; cx.strokeRect(8.5, y - 4.5, 8, 8);
        if (t.done) { cx.fillStyle = '#34d399'; cx.fillRect(10, y - 3, 5, 5); } else if (t.doing) { cx.fillStyle = '#22d3ee'; cx.fillRect(10, y - 1, 5, 2); }
        cx.fillStyle = t.done ? '#8e97a6' : '#e6e9f2'; cx.fillText(clip(cx, t.text, w - 30), 22, y); y += 15;
      }
      if (subs.length) {
        cx.fillStyle = '#8e97a6'; cx.fillText(`${subs.length} subagent${subs.length > 1 ? 's' : ''}`, 8, y); y += 15;
        for (const a of subs.slice(0, 4)) {
          glyph(cx, 'bot', 13, y, 9, a.state === 'done' ? '#8e97a6' : a.state === 'waiting' ? '#fbbf24' : '#22d3ee');
          cx.fillStyle = a.state === 'done' ? '#8e97a6' : '#e6e9f2'; cx.fillText(clip(cx, `${a.task} · ${a.state}`, w - 30), 22, y); y += 15;
        }
      }
      this.cardCache.set(d.id, { key, c });
      return c;
    }

    // hit testing and input
    hitAt(sx, sy) {
      const p = this.toWorld(sx, sy), fl = this.floor;
      for (const b of fl.bays) for (const d of b.desks) if (p.x >= d.x - 4 && p.x <= d.x + 36 && p.y >= d.y - 8 && p.y <= d.y + 34) return { kind: 'desk', d };
      for (const b of fl.bays) {
        if (!b.appId) continue;
        const x0 = (OX + b.bx * BAY_W) * TILE, y0 = (OY + b.by * BAY_H) * TILE;
        if ((this.layout.appBays.get(b.appId) || [])[0] === b && p.x >= x0 + 30 && p.x <= x0 + 84 && p.y >= y0 && p.y <= y0 + 30) return { kind: 'whiteboard', appId: b.appId };
        if (p.x >= x0 && p.x <= x0 + BAY_W * TILE && p.y >= y0 && p.y <= y0 + 30) return { kind: 'zone', appId: b.appId };
      }
      for (const pr of fl.props) if (pr.kind !== 'room' && p.x >= pr.x && p.x <= pr.x + pr.w && p.y >= pr.y && p.y <= pr.y + pr.h) return { kind: pr.kind, ref: pr.ref, label: pr.label };
      for (const pr of fl.props) if (pr.kind === 'room' && p.x >= pr.x && p.x <= pr.x + pr.w && p.y >= pr.y && p.y <= pr.y + pr.h) return { kind: 'room', room: pr.ref };
      if (this.cat && Math.abs(p.x - this.cat.x - 6) < 10 && Math.abs(p.y - this.cat.y - 4) < 8) return { kind: 'cat' };
      return null;
    }
    bindInput() {
      const cv = this.canvas; let drag = null, moved = false;
      cv.addEventListener('pointerdown', e => { drag = { x: e.clientX, y: e.clientY, cam: { ...this.cam } }; moved = false; cv.setPointerCapture(e.pointerId); });
      cv.addEventListener('pointermove', e => {
        const r = cv.getBoundingClientRect();
        if (drag) {
          const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
          if (Math.abs(dx) + Math.abs(dy) > 3) moved = true;
          if (moved) { this.tween = null; this.cam.x = drag.cam.x - dx / this.cam.s; this.cam.y = drag.cam.y - dy / this.cam.s; this.clampCam(); this.invalidate(); }
          return;
        }
        const h = this.hitAt(e.clientX - r.left, e.clientY - r.top);
        const key = h ? h.kind + (h.d ? h.d.id : h.appId || h.label || '') : '';
        if (key !== this.hoverKey) { this.hoverKey = key; this.hover = h; cv.style.cursor = h ? 'pointer' : 'grab'; this.invalidate(); this.opts.onHover && this.opts.onHover(h); }
      });
      cv.addEventListener('pointerup', e => {
        const r = cv.getBoundingClientRect();
        if (drag && !moved) this.click(this.hitAt(e.clientX - r.left, e.clientY - r.top), e.detail >= 2);
        drag = null;
      });
      cv.addEventListener('wheel', e => {
        e.preventDefault();
        const r = cv.getBoundingClientRect(), sx = e.clientX - r.left, sy = e.clientY - r.top;
        const before = this.toWorld(sx, sy);
        const k = Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0022));
        this.tween = null;
        this.cam.s = Math.max(this.fitScale() * 0.6, Math.min(10, this.cam.s * k));
        const after = this.toWorld(sx, sy); this.cam.x += before.x - after.x; this.cam.y += before.y - after.y; this.clampCam();
        clearTimeout(this.snapT); this.snapT = setTimeout(() => { const s2 = this.snapScale(this.cam.s); if (s2 !== this.cam.s) { const w0 = this.toWorld(sx, sy); this.cam.s = s2; const w1 = this.toWorld(sx, sy); this.cam.x += w0.x - w1.x; this.cam.y += w0.y - w1.y; this.invalidate(); } }, 160);
        this.invalidate();
      }, { passive: false });
      cv.addEventListener('keydown', e => {
        const step = 60 / this.cam.s;
        const k = e.key;
        if (k === 'ArrowLeft') this.cam.x -= step; else if (k === 'ArrowRight') this.cam.x += step; else if (k === 'ArrowUp') this.cam.y -= step; else if (k === 'ArrowDown') this.cam.y += step;
        else if (k === '+' || k === '=') this.flyTo(this.cam.x, this.cam.y, this.cam.s * 1.5); else if (k === '-') this.flyTo(this.cam.x, this.cam.y, this.cam.s / 1.5);
        else if (k === '0' || k === 'f') this.fit(); else if (k === 'n') this.opts.onNext && this.opts.onNext();
        else return;
        e.preventDefault(); this.clampCam(); this.invalidate();
      });
    }
    clampCam() { this.cam.x = Math.max(0, Math.min(W * TILE, this.cam.x)); this.cam.y = Math.max(0, Math.min(H * TILE, this.cam.y)); }
    click(h, dbl) {
      if (!h) { this.selected = null; this.opts.onSelect && this.opts.onSelect(null); this.invalidate(); return; }
      if (h.kind === 'desk') {
        const sid = h.d.sessionId;
        this.selected = sid || null;
        this.opts.onSelect && this.opts.onSelect(sid ? { kind: 'session', id: sid } : { kind: 'zone', appId: h.d.appId });
        if (dbl && sid) this.focusSession(sid, this.level() === 'near' ? 'near' : this.level() === 'mid' ? 'near' : 'mid');
        this.invalidate(); return;
      }
      this.opts.onSelect && this.opts.onSelect(h);
    }
  }
  function clip(ctx, text, max) {
    if (ctx.measureText(text).width <= max) return text;
    let t = text; while (t.length > 1 && ctx.measureText(t + '…').width > max) t = t.slice(0, -1);
    return t + '…';
  }

  // One desk on its own, for galleries and docs: the same drawing code as the floor.
  function vignette(canvas, o) {
    buildProps();
    const scale = o.scale || 4, w = 56, h = 54, hue = o.hue ?? 245;
    canvas.width = w * scale; canvas.height = h * scale;
    const ctx = canvas.getContext('2d'); ctx.imageSmoothingEnabled = false; ctx.setTransform(scale, 0, 0, scale, 0, 0);
    const base = hslHex(hue, 16, 40), r = rngFor('v', o.seed || 1);
    ctx.fillStyle = base; ctx.fillRect(0, 0, w, h);
    for (let i = 0; i < 40; i++) { ctx.fillStyle = shade(base, 0.86); ctx.fillRect((r() * w) | 0, (r() * h) | 0, 1, 1); }
    const d = { id: 'v' + (o.seed || 1), x: 12, y: 12, appId: 'A', staff: 0, chair: o.chair || 0, style: o.style || 0, sessionId: o.state ? 'S' : null };
    deskSurface(ctx, d, rngFor('vd', o.seed || 1));
    const subs = Array.from({ length: o.subs || 0 }, (_, k) => ({ state: k === 2 ? 'waiting' : 'working' }));
    const fake = Object.assign(Object.create(FloorView.prototype), {
      sessions: new Map(o.state ? [['S', { id: 'S', state: o.state, subagents: subs, cpuHot: !!o.hot }]] : []),
      chars: new Map([['A#0', buildCharacter(hash32('v|' + (o.seed || 1)), hue)]]), bots: new Map([['A', buildBot(hue)]]),
      walkers: [], seed: 'v' + (o.seed || 1), t: o.t || 0, motion: o.t ? 'full' : 'off',
    });
    if ((o.decay || 0) >= 3) { ctx.drawImage(SPR.trash, 2, 40); ctx.drawImage(SPR.box, 42, 40); ctx.drawImage(SPR.trash, 44, 2); }
    fake.drawDesk(ctx, d, { hue }, o.decay || 0, 'mid');
    if ((o.decay || 0) >= 3) { ctx.fillStyle = 'rgba(10,12,20,.38)'; ctx.fillRect(0, 0, w, h); }
    if (o.night) { ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = '#7a86b8'; ctx.fillRect(0, 0, w, h); ctx.globalCompositeOperation = 'source-over'; }
    if (o.state && NEEDS_YOU.has(o.state) && o.bubble !== false) { ctx.setTransform(1, 0, 0, 1, 0, 0); fake.bubble(ctx, (d.x + 16) * scale, (d.y - 3) * scale, o.state); }
  }

  G.FloorEngine = { FloorView, buildCharacter, buildBot, F, FW, FH, STATES, NEEDS_YOU, TILE, W, H, decayOf, agoWords, hslHex, hash32, mulberry, glyph, SPR, buildProps, P, CHAIR_COLORS, DESK_STYLES, DAY, vignette, Layout, ROOMS_F0, OX, OY, BAY_W, BAY_H };
})(window);
