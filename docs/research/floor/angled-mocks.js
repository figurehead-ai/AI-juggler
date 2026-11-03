/* The Floor, angled looks: three mock renderers over one generic sample office.
 * A — isometric pixel office (Canvas 2D, drawn at art resolution, scaled up pixelated)
 * B — low-poly 3D office from an angled perspective camera (three.js, rendered on demand)
 * C — cutaway building: floors stacked, each room in one-point perspective (Canvas 2D)
 * Research mocks only: everything is drawn by this code; no art is copied.
 */
(function (G) {
  'use strict';
  const E = G.FloorEngine;
  const HUES = [325, 115, 245, 50, 355, 185, 140, 305];
  const hsl = (h, s, l) => E.hslHex(((h % 360) + 360) % 360, s, l);

  // ---------- one sample office (generic names) ----------
  const ZONES = [
    { name: 'Claude Code · Work', hue: HUES[0], desks: ['working', 'asks', 'working', 'working', 'idle', 'working', 'done', 'working'], decay: 0, subs: [3, 0, 2, 1, 0, 4, 0, 1] },
    { name: 'Codex · Main', hue: HUES[2], desks: ['working', 'error', 'working', 'limit', 'working', 'stuck', 'working', 'idle'], decay: 0, subs: [2, 0, 1, 0, 3, 0, 1, 0], hot: true },
    { name: 'Claude app · Home', hue: HUES[3], desks: [null, null, null, null], decay: 2 },
  ];
  const STATE_SCREEN = { working: '#38bdf8', asks: '#fbbf24', error: '#ef4444', limit: '#fb923c', stuck: '#f59e0b', done: '#34d399', idle: '#475569', ended: '#111827' };
  const BUBBLE = { asks: ['!', 'Asks you', '#b7791f'], error: ['✕', 'Error', '#c53030'], limit: ['⌛', 'Limit', '#c05621'], stuck: ['?', 'Stuck?', '#b7791f'] };

  // ======================================================================
  // A — isometric pixel office
  // ======================================================================
  function mockA(canvas, o = {}) {
    E.buildProps();
    const night = !!o.night;
    const LW = 400, LH = 250;                    // art resolution
    canvas.width = LW; canvas.height = LH;
    const ctx = canvas.getContext('2d'); ctx.imageSmoothingEnabled = false;
    const TWh = 8, THh = 4;                      // half tile: 16x8 diamonds
    const NI = 22, NJ = 15, WALL = 46;           // room size in tiles, wall height in px
    const OX = 200 + (NJ - NI) * TWh / 2 + 10, OY = 58;
    const P = (i, j, z = 0) => [Math.round(OX + (i - j) * TWh), Math.round(OY + (i + j) * THh - z)];
    const poly = (pts, fill) => { ctx.fillStyle = fill; ctx.beginPath(); pts.forEach(([x, y], k) => k ? ctx.lineTo(x, y) : ctx.moveTo(x, y)); ctx.closePath(); ctx.fill(); };
    const shade = (hex, f) => { const n = parseInt(hex.slice(1), 16); const c = [n >> 16, (n >> 8) & 255, n & 255].map(v => Math.max(0, Math.min(255, Math.round(v * f)))); return '#' + c.map(v => v.toString(16).padStart(2, '0')).join(''); };
    ctx.fillStyle = night ? '#0b0e14' : '#d9dde5'; ctx.fillRect(0, 0, LW, LH);
    const zoneOf = (i, j) => (i >= 3 && i < 10 && j >= 2 && j < 14) ? 0 : (i >= 11 && i < 18 && j >= 2 && j < 14) ? 1 : (i >= 18 && j >= 6) ? 2 : -1;
    // floor
    for (let i = 0; i < NI; i++) for (let j = 0; j < NJ; j++) {
      const z = zoneOf(i, j);
      let c = (i + j) & 1 ? '#b9bcc4' : '#b2b5bd';
      if (i < 3 && j < 6) c = (i + j) & 1 ? '#e9e3d6' : '#d8d1c2';                // kitchen tiles
      if (z >= 0) c = hsl(ZONES[z].hue, 14, (i + j) & 1 ? 58 : 55);
      if (z === 2) c = shade(c, 0.8);
      poly([P(i, j), P(i + 1, j), P(i + 1, j + 1), P(i, j + 1)], c);
    }
    // window light on the floor
    ctx.globalAlpha = night ? 0 : 0.18;
    for (let w = 0; w < 5; w++) { const i0 = 3 + w * 4; poly([P(i0, 0), P(i0 + 2, 0), P(i0 + 3, 4), P(i0 + 1, 4)], '#fff7d6'); }
    ctx.globalAlpha = 1;
    // back walls: along i at j=0 (right-hand wall) and along j at i=0 (left-hand wall)
    const wallR = night ? '#8f8a9e' : '#e7e1d6', wallL = night ? '#7a7589' : '#d6cfc2';
    poly([P(0, 0), P(NI, 0), P(NI, 0, WALL), P(0, 0, WALL)], wallR);
    poly([P(0, 0), P(0, NJ), P(0, NJ, WALL), P(0, 0, WALL)], wallL);
    poly([P(0, 0, WALL), P(NI, 0, WALL), P(NI, -0.5, WALL), P(0, -0.5, WALL)], '#433a4f');
    poly([P(0, 0, WALL), P(0, NJ, WALL), P(-0.5, NJ, WALL), P(-0.5, 0, WALL)], '#3a3346');
    // windows on the right-hand wall: sky, frame, mullion
    for (let w = 0; w < 5; w++) {
      const i0 = 3 + w * 4, z0 = 14, zh = 24;
      poly([P(i0, 0, z0 - 2), P(i0 + 2.6, 0, z0 - 2), P(i0 + 2.6, 0, z0 + zh + 2), P(i0, 0, z0 + zh + 2)], '#f4f1ea');
      poly([P(i0 + 0.2, 0, z0), P(i0 + 2.4, 0, z0), P(i0 + 2.4, 0, z0 + zh), P(i0 + 0.2, 0, z0 + zh)], night ? '#1d2547' : '#9fd0f0');
      poly([P(i0 + 0.2, 0, z0 + zh * 0.25), P(i0 + 1.6, 0, z0 + zh * 0.25), P(i0 + 1.6, 0, z0 + zh * 0.32), P(i0 + 0.2, 0, z0 + zh * 0.32)], night ? '#3a4a7a' : '#d6eefb');
      // skyline silhouettes
      for (let b = 0; b < 4; b++) { const bi = i0 + 0.3 + b * 0.5, bh = 5 + ((w * 7 + b * 5) % 9); poly([P(bi, 0, z0), P(bi + 0.42, 0, z0), P(bi + 0.42, 0, z0 + bh), P(bi, 0, z0 + bh)], night ? '#2c3560' : '#7fa9c7'); }
      poly([P(i0 + 1.28, 0, z0), P(i0 + 1.32, 0, z0), P(i0 + 1.32, 0, z0 + zh), P(i0 + 1.28, 0, z0 + zh)], '#f4f1ea');
    }
    // doors on the left-hand wall: elevator and stairs
    const door = (j0, w, col, label) => {
      poly([P(0, j0 - 0.2, 0), P(0, j0 + w + 0.2, 0), P(0, j0 + w + 0.2, 34), P(0, j0 - 0.2, 34)], '#5a5f6e');
      poly([P(0, j0, 0), P(0, j0 + w, 0), P(0, j0 + w, 31), P(0, j0, 31)], col);
      if (label) { const [x, y] = P(0, j0 + w / 2, 38); ctx.fillStyle = '#222'; ctx.fillRect(x - 5, y - 3, 10, 5); ctx.fillStyle = '#ffb347'; ctx.fillRect(x - 1, y - 2, 2, 3); }
    };
    door(7, 2.2, '#c9cfdb', true);
    poly([P(0, 8.1, 0), P(0, 8.14, 0), P(0, 8.14, 31), P(0, 8.1, 31)], '#8d94a3');
    door(11.5, 1.4, '#9b6f43');
    // wall things: whiteboard (tasks), corkboard (notices), clock
    poly([P(5, 0, 6), P(9, 0, 6), P(9, 0, 9), P(5, 0, 9)], '#8d94a3');
    const board = (i0, w, z0, h, c) => poly([P(i0, 0, z0), P(i0 + w, 0, z0), P(i0 + w, 0, z0 + h), P(i0, 0, z0 + h)], c);
    board(4.2, 1.3, 10, 1, '#00000000');
    // kitchen counter along the left wall
    const box = (i, j, w, d, h, z, top, left, right) => {
      poly([P(i, j + d, z), P(i + w, j + d, z), P(i + w, j + d, z + h), P(i, j + d, z + h)], left);
      poly([P(i + w, j, z), P(i + w, j + d, z), P(i + w, j + d, z + h), P(i + w, j, z + h)], right);
      poly([P(i, j, z + h), P(i + w, j, z + h), P(i + w, j + d, z + h), P(i, j + d, z + h)], top);
    };
    const items = [];
    const add = (d, fn) => items.push({ d, fn });
    add(1.2, () => box(0, 0, 1, 5, 12, 0, '#efe9dc', '#c9c0ae', '#b5ab98'));
    add(1.3, () => box(0.1, 1, 0.6, 0.8, 6, 12, '#3a3a44', '#2a2a33', '#22222a'));
    add(3, () => { box(1.4, 2.6, 1.4, 1.4, 9, 0, '#cf9f68', '#9b6f43', '#8a6039'); });
    add(2, () => box(0, 5.2, 1, 1, 30, 0, '#e8ecf2', '#c9cfdb', '#b4bac8'));
    // desks with people
    const ch = (seed, hue) => E.buildCharacter(E.hash32('isoA|' + seed), hue);
    const sprite = (atlas, f, i, j, z, flip) => {
      const [x, y] = P(i, j, z);
      ctx.save(); if (flip) { ctx.translate(x, 0); ctx.scale(-1, 1); ctx.drawImage(atlas, f * E.FW, 0, E.FW, E.FH, -7, y - 21, E.FW, E.FH); }
      else ctx.drawImage(atlas, f * E.FW, 0, E.FW, E.FH, x - 7, y - 21, E.FW, E.FH);
      ctx.restore();
    };
    const bubble = (i, j, z, st) => {
      const [x, y] = P(i, j, z); const b = BUBBLE[st]; if (!b) return;
      ctx.fillStyle = '#fffaf0'; ctx.fillRect(x - 5, y - 9, 11, 9); ctx.fillRect(x - 1, y, 3, 2);
      ctx.fillStyle = b[2]; ctx.font = '700 8px ui-sans-serif, system-ui, sans-serif'; ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.fillText(b[0], x + 0.5, y - 4);
    };
    ZONES.forEach((zn, zi) => {
      const i0 = zi === 0 ? 4 : zi === 1 ? 12 : 18.5, j0 = zi === 2 ? 7 : 3;
      zn.desks.forEach((st, k) => {
        const pod = (k / 4) | 0, kk = k % 4;
        const di = i0 + (kk % 2) * 2.4, dj = j0 + pod * 5 + ((kk / 2) | 0) * 1.6;
        const facing = (kk / 2) | 0; // 0 faces the back wall, 1 faces the viewer
        const person = st && zn.decay === 0 ? ch(zi * 10 + k, zn.hue) : null;
        const dd = di + dj;
        add(dd + 0.5, () => {
          box(di, dj, 2, 1, 9, 0, zn.decay ? '#a89a86' : '#d6b282', '#a7814f', '#94703f');
          // monitor faces the person
          const scr = st ? STATE_SCREEN[st] : '#111827';
          box(di + 0.7, dj + (facing ? 0.75 : 0.15), 0.7, 0.12, 7, 9, '#222', facing ? scr : '#2a2a33', '#1b1b22');
          if (zn.decay >= 2) { ctx.fillStyle = 'rgba(230,226,214,.7)'; for (let q = 0; q < 10; q++) { const [x, y] = P(di + (q * 7 % 20) / 10, dj + (q * 3 % 10) / 10, 9); ctx.fillRect(x, y, 1, 1); } }
        });
        const ci = di + 1, cj = facing ? dj - 0.55 : dj + 1.55;
        if (person) {
          const standing = st === 'asks' || st === 'error';
          if (standing) {
            add(ci + cj + 1.2, () => { sprite(person.atlas, st === 'asks' ? E.F.WAVE0 : E.F.ALARM, ci + 0.6, cj + (facing ? -0.4 : 0.6), 0); bubble(ci + 0.6, cj + (facing ? -0.4 : 0.6), 30, st); });
          } else {
            const f = st === 'limit' ? E.F.SLEEP : st === 'stuck' ? E.F.SCRATCH : st === 'done' ? E.F.LEAN : st === 'idle' ? E.F.SITIDLE : E.F.SIT0;
            add(ci + cj + (facing ? -0.2 : 0.8), () => {
              if (facing) sprite(person.atlas, E.F.STAND, ci, cj, 2); else sprite(person.atlas, f, ci, cj, 4);
              box(ci - 0.35, cj - 0.2, 0.7, 0.4, 7, 5, '#3b3f4c', '#2c2f3a', '#262833');
              if (st === 'stuck' || st === 'limit') bubble(ci, cj, 26, st);
            });
          }
          if (zn.subs && zn.subs[k]) add(dd + 2.5, () => { const [x, y] = P(di + 2.2, dj + 0.5, 20 + (k % 2) * 3); ctx.drawImage(E.SPR.ready ? botFor(zn.hue) : null, 0, 0, 10, 11, x - 5, y - 11, 10, 11); });
          if (zn.hot && k === 0) add(dd + 0.9, () => { const [x, y] = P(di + 1.6, dj + 0.3, 9); ctx.drawImage(E.SPR.fire[1], x - 6, y - 14); });
        } else {
          add(ci + cj, () => box(ci - 0.35 + (zn.decay ? 0.4 : 0), cj - 0.2 + (zn.decay ? 0.3 : 0), 0.7, 0.4, 7, 5, '#4b4f5c', '#383b46', '#30323c'));
        }
      });
      if (zn.decay >= 2) for (let q = 0; q < 4; q++) add(i0 + j0 + q, () => { const [x, y] = P(i0 + (q * 1.7) % 4.5, j0 + 3.2 + (q % 2) * 0.9, 0); ctx.drawImage(q % 2 ? E.SPR.box : E.SPR.trash, x - 6, y - 10); });
    });
    // plants
    [[2.5, 13], [10.5, 1.5], [20.5, 2], [10.6, 13.5]].forEach(([i, j], k) => add(i + j, () => {
      box(i, j, 0.7, 0.7, 6, 0, '#b4643a', '#8d4b2a', '#7a4024');
      const [x, y] = P(i + 0.35, j + 0.35, 6); ctx.fillStyle = '#2b7240'; ctx.fillRect(x - 6, y - 12, 12, 12); ctx.fillStyle = '#3f9b55'; ctx.fillRect(x - 5, y - 15, 10, 10); ctx.fillStyle = '#6cc77c'; ctx.fillRect(x - 2, y - 15, 4, 4);
    }));
    items.sort((a, b) => a.d - b.d).forEach(it => it.fn());
    // whiteboard (tasks) and corkboard (notices) on the wall, drawn after the floor items behind them
    const notes = (i0, z0, cols) => cols.forEach((c, k) => { const [x, y] = P(i0 + (k % 3) * 0.35, 0, z0 + ((k / 3) | 0) * 4); ctx.fillStyle = c; ctx.fillRect(x, y - 3, 3, 3); });
    poly([P(5, 0, 18), P(8.5, 0, 18), P(8.5, 0, 34), P(5, 0, 34)], '#8d94a3'); poly([P(5.15, 0, 19), P(8.35, 0, 19), P(8.35, 0, 33), P(5.15, 0, 33)], '#f7f8fb');
    notes(5.4, 21, ['#ffe28a', '#ffe28a', '#8fd3ff', '#8fd3ff', '#9be8b5', '#ffe28a']);
    poly([P(13, 0, 18), P(15.4, 0, 18), P(15.4, 0, 32), P(13, 0, 32)], '#9c6c3c'); poly([P(13.15, 0, 19), P(15.25, 0, 19), P(15.25, 0, 31), P(13.15, 0, 31)], '#c08a52');
    notes(13.4, 21, ['#ffd0cc', '#fff1b8', '#f2efe6', '#fff1b8']);
    const [cx, cy] = P(10.2, 0, 34); ctx.fillStyle = '#fff'; ctx.beginPath(); ctx.ellipse(cx, cy, 3, 4, 0, 0, 7); ctx.fill(); ctx.fillStyle = '#222'; ctx.fillRect(cx, cy - 3, 1, 3);
    // front cutaway: the near walls are stubs, so you see in
    poly([P(NI, 0), P(NI, NJ), P(NI, NJ, 4), P(NI, 0, 4)], '#cfc8bb');
    poly([P(0, NJ), P(NI, NJ), P(NI, NJ, 4), P(0, NJ, 4)], '#bfb8aa');
    if (night) { ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = '#8a92c0'; ctx.fillRect(0, 0, LW, LH); ctx.globalCompositeOperation = 'source-over'; }
  }
  const bots = new Map();
  function botFor(hue) { if (!bots.has(hue)) bots.set(hue, E.buildBot(hue)); return bots.get(hue); }

  // ======================================================================
  // B — low-poly 3D, angled perspective camera (three.js, render on demand)
  // ======================================================================
  function mockB(canvas, o = {}) {
    const T = G.THREE; if (!T) return null;
    const night = !!o.night;
    const renderer = new T.WebGLRenderer({ canvas, antialias: true, powerPreference: 'low-power' });
    renderer.setPixelRatio(Math.min(2, G.devicePixelRatio || 1));
    renderer.shadowMap.enabled = true; renderer.shadowMap.type = T.PCFSoftShadowMap;
    renderer.outputEncoding = T.sRGBEncoding; renderer.toneMapping = T.ACESFilmicToneMapping; renderer.toneMappingExposure = night ? 0.85 : 0.92;
    const scene = new T.Scene(); scene.background = new T.Color(night ? 0x0b0e14 : 0xcfd5df);
    const camera = new T.PerspectiveCamera(30, 1, 0.5, 200);
    const mats = new Map();
    const M = (c, x = {}) => { const k = c + JSON.stringify(x); if (!mats.has(k)) mats.set(k, new T.MeshStandardMaterial(Object.assign({ color: c, roughness: 0.85, metalness: 0 }, x))); return mats.get(k); };
    const geoBox = new T.BoxGeometry(1, 1, 1);
    const box = (w, h, d, mat, x, y, z, parent = scene, sh = true) => { const m = new T.Mesh(geoBox, mat); m.scale.set(w, h, d); m.position.set(x, y + h / 2, z); m.castShadow = sh; m.receiveShadow = true; parent.add(m); return m; };
    const W2 = 10, D2 = 6.5, WH = 3.2;
    // floor, zone carpets
    box(W2 * 2, 0.1, D2 * 2, M(night ? '#5f5c58' : '#8f8b84'), 0, -0.1, 0, scene, false);
    // walls: back (z=-8) and left (x=-13); front/right are cut down to stubs
    const wallM = M(night ? '#9b96a8' : '#e2dacb');
    box(W2 * 2, WH, 0.2, wallM, 0, 0, -D2); box(0.2, WH, D2 * 2, wallM, -W2, 0, 0);
    box(W2 * 2, 0.3, 0.15, M('#d8d2c6'), 0, 0, D2); box(0.15, 0.3, D2 * 2, M('#d8d2c6'), W2, 0, 0);
    box(W2 * 2, 0.12, 0.05, M('#a89c88'), 0, 0, -D2 + 0.12); // skirting
    // windows with a procedural skyline
    const sky = document.createElement('canvas'); sky.width = 256; sky.height = 160; const sx = sky.getContext('2d');
    const g = sx.createLinearGradient(0, 0, 0, 160); g.addColorStop(0, night ? '#0f1736' : '#7cc3ee'); g.addColorStop(1, night ? '#2a3466' : '#e4f3fb'); sx.fillStyle = g; sx.fillRect(0, 0, 256, 160);
    const r = E.mulberry(5);
    for (let b = 0; b < 14; b++) { const bx = b * 19 + r() * 6, bh = 40 + r() * 80; sx.fillStyle = night ? '#1b2245' : '#8fb2cc'; sx.fillRect(bx, 160 - bh, 16, bh); sx.fillStyle = night ? '#ffd27a' : '#c9e2f2'; for (let q = 0; q < 12; q++) if (r() < (night ? 0.35 : 0.5)) sx.fillRect(bx + 2 + (q % 3) * 5, 160 - bh + 6 + ((q / 3) | 0) * 9, 3, 4); }
    const skyTex = new T.CanvasTexture(sky); skyTex.encoding = T.sRGBEncoding;
    const glassM = new T.MeshBasicMaterial({ map: skyTex, toneMapped: false });
    const frameM = M('#f6f3ec');
    for (let k = 0; k < 6; k++) {
      if (k === 2 || k === 3) continue;
      const x = -8.2 + k * 3.3;
      const gl = new T.Mesh(new T.PlaneGeometry(2.0, 1.5), glassM); gl.position.set(x, 1.75, -D2 + 0.11); scene.add(gl);
      box(2.2, 0.08, 0.12, frameM, x, 0.95, -D2 + 0.12); box(2.2, 0.08, 0.12, frameM, x, 2.5, -D2 + 0.12);
      box(0.08, 1.6, 0.12, frameM, x - 1.05, 0.95, -D2 + 0.12); box(0.08, 1.6, 0.12, frameM, x + 1.05, 0.95, -D2 + 0.12); box(0.05, 1.5, 0.1, frameM, x, 1.0, -D2 + 0.13);
    }
    // doors on the left wall: elevator (metal, double) and stairs (wood)
    box(0.12, 2.3, 1.9, M('#8d94a3', { metalness: 0.4, roughness: 0.4 }), -W2 + 0.11, 0, -1.2); box(0.13, 2.2, 0.03, M('#5a5f6e'), -W2 + 0.12, 0, -1.2);
    box(0.1, 0.25, 0.5, M('#222'), -W2 + 0.12, 2.45, -1.2);
    box(0.1, 2.2, 1.0, M('#9b6f43'), -W2 + 0.11, 0, 1.4); box(0.12, 0.05, 0.1, M('#d9b25c', { metalness: 0.6 }), -W2 + 0.15, 1.0, 1.75);
    // meeting room: glass partition in the back-left corner
    const glassWall = new T.MeshStandardMaterial({ color: 0xbfe0f2, transparent: true, opacity: 0.28, roughness: 0.1, metalness: 0.1 });
    box(0.06, WH - 0.4, 3.6, glassWall, -5.2, 0, -4.7, scene, false); box(4.8, WH - 0.4, 0.06, glassWall, -7.6, 0, -2.9, scene, false);
    box(0.08, 0.08, 3.6, frameM, -5.2, WH - 0.4, -4.7); box(4.8, 0.08, 0.08, frameM, -7.6, WH - 0.4, -2.9);
    box(2.6, 0.06, 1.2, M('#e9e7e1'), -7.6, 0.72, -4.7); box(0.1, 0.72, 0.1, M('#555'), -7.6, 0, -4.7);
    for (let k = 0; k < 6; k++) chair(-8.6 + (k % 3) * 1.0, k < 3 ? -5.6 : -3.8, k < 3 ? 0 : Math.PI, '#5a3d5c');
    box(0.06, 1.0, 1.8, M('#23262f'), -W2 + 0.14, 1.4, -4.7); box(0.02, 0.85, 1.6, M('#22384d', { emissive: '#2b5b80', emissiveIntensity: 0.8 }), -W2 + 0.18, 1.48, -4.7, scene, false);
    // server room: racks with lit LEDs in the front-left corner, behind glass
    box(0.06, WH - 0.4, 3.4, glassWall, -6.6, 0, 4.8, scene, false); box(3.4, WH - 0.4, 0.06, glassWall, -8.3, 0, 3.1, scene, false);
    for (let k = 0; k < 3; k++) { const rz = 3.9 + k * 1.0; box(0.9, 2.0, 0.7, M('#1f2230', { roughness: 0.5 }), -9.4, 0, rz); for (let q = 0; q < 8; q++) box(0.02, 0.04, 0.42, M('#0b0e14', { emissive: q < 5 ? '#34d399' : '#30364a', emissiveIntensity: 1.4 }), -8.94, 0.3 + q * 0.2, rz, scene, false); }
    // kitchen in the back-right corner
    box(3.6, 0.92, 0.7, M('#e9e2d3'), 7.6, 0, -D2 + 0.45); box(3.6, 0.05, 0.75, M('#bfb5a2'), 7.6, 0.92, -D2 + 0.45);
    box(0.8, 2.0, 0.75, M('#e8ecf2', { roughness: 0.3, metalness: 0.2 }), 9.5, 0, -D2 + 0.5);
    box(0.35, 0.45, 0.35, M('#3a3a44'), 6.4, 0.97, -D2 + 0.45);
    const rt = new T.Mesh(new T.CylinderGeometry(0.6, 0.6, 0.05, 24), M('#cf9f68')); rt.position.set(8.0, 0.74, -4.6); rt.castShadow = true; scene.add(rt);
    box(0.08, 0.72, 0.08, M('#555'), 8.0, 0, -4.6);
    // wall things: whiteboard (tasks), corkboard (notices), clock wall (resets)
    box(2.2, 1.2, 0.05, M('#f7f8fb', { roughness: 0.3 }), -1.5, 1.1, -D2 + 0.13); box(2.3, 0.05, 0.12, M('#8d94a3'), -1.5, 1.07, -D2 + 0.16);
    ['#ffe28a', '#ffe28a', '#8fd3ff', '#9be8b5', '#8fd3ff', '#ffe28a'].forEach((c, k) => box(0.22, 0.18, 0.01, M(c), -2.2 + (k % 3) * 0.7, 1.75 - ((k / 3) | 0) * 0.35, -D2 + 0.16, scene, false));
    box(1.5, 1.0, 0.05, M('#c08a52'), 1.7, 1.2, -D2 + 0.13);
    ['#ffd0cc', '#fff1b8', '#f2efe6', '#fff1b8'].forEach((c, k) => box(0.26, 0.22, 0.01, M(c), 1.2 + (k % 2) * 0.6, 1.8 - ((k / 2) | 0) * 0.42, -D2 + 0.16, scene, false));
    for (let k = 0; k < 3; k++) { const c = new T.Mesh(new T.CylinderGeometry(0.17, 0.17, 0.04, 20), M('#ffffff')); c.rotation.x = Math.PI / 2; c.position.set(1.2 + k * 0.5, 2.55, -D2 + 0.14); scene.add(c); }
    // plants
    const leaf = M('#3f9b55', { flatShading: true }), pot = M('#b4643a');
    const plant = (x, z, s = 1) => { const p = new T.Mesh(new T.CylinderGeometry(0.22 * s, 0.17 * s, 0.4 * s, 12), pot); p.position.set(x, 0.2 * s, z); p.castShadow = true; scene.add(p); const f = new T.Mesh(new T.IcosahedronGeometry(0.45 * s, 0), leaf); f.position.set(x, 0.75 * s, z); f.castShadow = true; scene.add(f); };
    plant(-4.6, -5.9, 1.2); plant(3.4, -5.9); plant(9.3, 5.9, 1.3); plant(-5.6, 5.9);

    // people, desks and their state
    function chair(x, z, rotY, color, askew = 0) {
      const gp = new T.Group(); gp.position.set(x, 0, z); gp.rotation.y = rotY + askew; scene.add(gp);
      const m = M(color);
      box(0.5, 0.07, 0.48, m, 0, 0.44, 0, gp); box(0.5, 0.5, 0.07, m, 0, 0.55, 0.22, gp); box(0.06, 0.42, 0.06, M('#333'), 0, 0.02, 0, gp); box(0.5, 0.04, 0.5, M('#333'), 0, 0, 0, gp);
      return gp;
    }
    const SKIN = ['#f6d5bd', '#ebbb97', '#d29b70', '#b07650', '#8a5634', '#62391f'], HAIR = ['#2b1d14', '#4a3020', '#7a4b25', '#b07a3a', '#dcb766', '#e6e1d6', '#a33b2e', '#1f1f24'];
    function person(seed, hue, pose) {
      const r2 = E.mulberry(E.hash32('p3d|' + seed));
      const gp = new T.Group();
      const shirt = M(hsl(hue + r2() * 20 - 10, 50, 46 + r2() * 10)), skin = M(SKIN[(r2() * SKIN.length) | 0]), hair = M(HAIR[(r2() * HAIR.length) | 0]), pants = M(['#2f3a5c', '#3a3a42', '#7a6a4a', '#3d5a80'][(r2() * 4) | 0]);
      const sit = pose !== 'stand' && pose !== 'wave' && pose !== 'alarm';
      const hip = sit ? 0.48 : 0.86;
      if (sit) { box(0.16, 0.14, 0.45, pants, -0.1, hip, -0.2, gp); box(0.16, 0.14, 0.45, pants, 0.1, hip, -0.2, gp); box(0.15, 0.45, 0.15, pants, -0.1, 0.02, -0.4, gp); box(0.15, 0.45, 0.15, pants, 0.1, 0.02, -0.4, gp); }
      else { box(0.16, hip, 0.16, pants, -0.1, 0, 0, gp); box(0.16, hip, 0.16, pants, 0.1, 0, 0, gp); }
      const lean = pose === 'sleep' ? 0.5 : pose === 'lean' ? -0.25 : 0;
      const torso = new T.Group(); torso.position.set(0, hip + 0.06, 0); torso.rotation.x = -lean; gp.add(torso);
      box(0.44, 0.55, 0.26, shirt, 0, 0, 0, torso);
      const head = new T.Mesh(new T.SphereGeometry(0.15, 16, 12), skin); head.position.set(0, 0.74, 0); head.castShadow = true; torso.add(head);
      const hr = new T.Mesh(new T.SphereGeometry(0.162, 16, 12, 0, Math.PI * 2, 0, Math.PI * (0.45 + r2() * 0.15)), hair); hr.position.set(0, 0.76, 0.01); hr.castShadow = true; torso.add(hr);
      const arm = (side, up) => { const a = new T.Group(); a.position.set(side * 0.28, 0.52, 0); torso.add(a); const m = new T.Mesh(geoBox, shirt); m.scale.set(0.11, 0.48, 0.11); m.position.set(0, -0.24, 0); m.castShadow = true; a.add(m); const h = new T.Mesh(new T.SphereGeometry(0.06, 8, 6), skin); h.position.set(0, -0.5, 0); a.add(h); a.rotation.x = up ? 0 : (sit ? -1.1 : 0); if (up) a.rotation.z = side * 2.6; return a; };
      arm(-1, pose === 'alarm'); arm(1, pose === 'wave' || pose === 'alarm' || pose === 'scratch');
      return gp;
    }
    const labelTex = (lines, accent) => {
      const c = document.createElement('canvas'); const dpr = 2; c.width = 460 * dpr; c.height = 120 * dpr; const x = c.getContext('2d'); x.scale(dpr, dpr);
      x.fillStyle = 'rgba(16,19,30,.9)'; x.beginPath(); x.roundRect ? x.roundRect(4, 4, 452, 112, 22) : x.rect(4, 4, 452, 112); x.fill();
      x.fillStyle = accent; x.beginPath(); x.roundRect ? x.roundRect(22, 26, 40, 40, 10) : x.rect(22, 26, 40, 40); x.fill();
      x.fillStyle = '#fff'; x.font = '700 38px ui-sans-serif, system-ui, sans-serif'; x.fillText(lines[0], 78, 58);
      x.fillStyle = '#c7cde0'; x.font = '500 30px ui-sans-serif, system-ui, sans-serif'; x.fillText(lines[1], 78, 98);
      const t = new T.CanvasTexture(c); t.encoding = T.sRGBEncoding; return t;
    };
    const sprite = (tex, w, h, x, y, z) => { const s = new T.Sprite(new T.SpriteMaterial({ map: tex, depthTest: false, toneMapped: false })); s.scale.set(w, h, 1); s.position.set(x, y, z); s.renderOrder = 10; scene.add(s); return s; };
    const bubbleTex = st => {
      const [glyph, word, col] = BUBBLE[st]; const c = document.createElement('canvas'); c.width = 320; c.height = 120; const x = c.getContext('2d');
      x.fillStyle = '#fffaf0'; x.strokeStyle = col; x.lineWidth = 8; x.beginPath(); x.roundRect ? x.roundRect(8, 8, 304, 84, 42) : x.rect(8, 8, 304, 84); x.fill(); x.stroke();
      x.beginPath(); x.moveTo(140, 92); x.lineTo(160, 116); x.lineTo(180, 92); x.fill();
      x.fillStyle = col; x.font = '800 50px ui-sans-serif, system-ui, sans-serif'; x.textBaseline = 'middle'; x.fillText(glyph, 36, 52); x.fillStyle = '#1d1726'; x.font = '700 42px ui-sans-serif, system-ui, sans-serif'; x.fillText(word, 90, 54);
      const t = new T.CanvasTexture(c); t.encoding = T.sRGBEncoding; return t;
    };
    ZONES.forEach((zn, zi) => {
      const x0 = zi === 0 ? -3.6 : zi === 1 ? 1.9 : 7.6, z0 = zi === 2 ? -1.6 : -2.6;
      const carpet = box(zi === 2 ? 3.6 : 4.4, 0.02, zi === 2 ? 7.0 : 8.6, M(hsl(zn.hue, 30, night ? 38 : 56)), x0 + 0.78, 0, z0 + (zi === 2 ? 2.6 : 2.4), scene, false); carpet.castShadow = false;
      const working = zn.desks.filter(s => s === 'working').length, needs = zn.desks.filter(s => BUBBLE[s]).length;
      sprite(labelTex([zn.name, zn.decay ? 'Unused 9 days' : `${working} working${needs ? ` · ${needs} need you` : ''}`], hsl(zn.hue, 60, 55)), 2.4, 0.62, x0 + 0.78, 2.7, z0 - 0.9);
      zn.desks.forEach((st, k) => {
        const pod = (k / 4) | 0, kk = k % 4;
        const dx = x0 + (kk % 2) * 1.55, dz = z0 + pod * 3.9 + ((kk / 2) | 0) * 0.82;
        const facing = (kk / 2) | 0; // 0: person sits on the far side, facing the camera; 1: near side, back to camera
        const deskM = M(zn.decay ? '#b3a996' : '#d6b282'), legM = M('#555');
        box(1.45, 0.05, 0.78, deskM, dx, 0.72, dz); box(0.05, 0.72, 0.7, legM, dx - 0.68, 0, dz); box(0.05, 0.72, 0.7, legM, dx + 0.68, 0, dz);
        const scr = st && !zn.decay ? STATE_SCREEN[st] : '#0b0e14';
        const mon = new T.Group(); mon.position.set(dx, 0.77, dz + (facing ? 0.12 : -0.12)); mon.rotation.y = facing ? Math.PI : 0; scene.add(mon);
        box(0.62, 0.38, 0.04, M('#1b1b22'), 0, 0.14, 0, mon); box(0.56, 0.32, 0.01, M('#000', { emissive: scr, emissiveIntensity: st && !zn.decay ? 1.2 : 0 }), 0, 0.17, facing ? -0.025 : 0.025, mon, false); box(0.05, 0.14, 0.05, M('#333'), 0, 0, 0, mon);
        box(0.42, 0.02, 0.14, M('#d9dde5'), dx, 0.77, dz + (facing ? -0.22 : 0.22));
        const cz = dz + (facing ? -0.62 : 0.62), rotY = facing ? 0 : Math.PI;
        if (!st || zn.decay) {
          chair(dx + (zn.decay ? 0.3 : 0), cz + (zn.decay ? (facing ? -0.3 : 0.3) : 0), rotY, '#3b3f4c', zn.decay ? 0.6 + k * 0.3 : 0);
          if (zn.decay) { box(0.18, 0.1, 0.12, M('#f4f1ea'), dx - 0.4, 0.77, dz); }
          return;
        }
        const standing = st === 'asks' || st === 'error';
        if (standing) {
          chair(dx + 0.55, cz, rotY, '#3b3f4c', 0.5);
          const p = person(zi * 10 + k, zn.hue, st === 'asks' ? 'wave' : 'alarm'); p.position.set(dx - 0.25, 0, cz + (facing ? -0.25 : 0.25)); p.rotation.y = Math.atan2(camera.position.x - p.position.x, camera.position.z - p.position.z) || 0.5; scene.add(p);
          p.userData.faceCamera = true;
          sprite(bubbleTex(st), 1.1, 0.42, dx - 0.25, 2.25, cz + (facing ? -0.25 : 0.25));
        } else {
          chair(dx, cz, rotY, '#3b3f4c');
          const p = person(zi * 10 + k, zn.hue, st === 'limit' ? 'sleep' : st === 'stuck' ? 'scratch' : st === 'done' ? 'lean' : 'sit'); p.position.set(dx, 0, cz); p.rotation.y = facing ? 0 : Math.PI; scene.add(p);
          if (st === 'stuck' || st === 'limit') sprite(bubbleTex(st), 1.0, 0.38, dx, 2.0, cz);
        }
        // subagents: helper drones above the desk
        for (let q = 0; q < Math.min(3, (zn.subs || [])[k] || 0); q++) {
          const d = new T.Mesh(new T.SphereGeometry(0.09, 12, 8), M(hsl(zn.hue, 60, 60), { emissive: hsl(zn.hue, 70, 55), emissiveIntensity: 0.6 }));
          d.position.set(dx - 0.6 + q * 0.3, 1.55 + (q % 2) * 0.12, dz); d.castShadow = true; scene.add(d);
          const ring = new T.Mesh(new T.TorusGeometry(0.13, 0.015, 6, 20), M('#c9cfdb', { metalness: 0.5 })); ring.rotation.x = Math.PI / 2; ring.position.copy(d.position); scene.add(ring);
        }
        if (zn.hot && k === 0) {
          const f1 = new T.Mesh(new T.ConeGeometry(0.16, 0.42, 8), M('#ff7a2f', { emissive: '#ff5a1a', emissiveIntensity: 1.6 })); f1.position.set(dx + 0.5, 0.97, dz); scene.add(f1);
          const f2 = new T.Mesh(new T.ConeGeometry(0.08, 0.26, 8), M('#ffd166', { emissive: '#ffd166', emissiveIntensity: 1.8 })); f2.position.set(dx + 0.5, 0.92, dz); scene.add(f2);
          const pl = new T.PointLight(0xff7a2f, 1.4, 3.5); pl.position.set(dx + 0.5, 1.3, dz); scene.add(pl);
        }
      });
      if (zn.decay >= 2) for (let q = 0; q < 5; q++) { const tb = new T.Mesh(new T.SphereGeometry(0.28, 8, 6), M('#2c2f36', { roughness: 0.4 })); tb.scale.y = 0.8; tb.position.set(x0 - 0.4 + q * 0.55, 0.22, z0 + 3.6 + (q % 2) * 0.5); tb.castShadow = true; scene.add(tb); }
      if (zn.decay >= 2) { box(0.6, 0.45, 0.5, M('#b98b57'), x0 + 1.8, 0, z0 + 4.6); box(0.5, 0.4, 0.45, M('#a77b4a'), x0 + 1.6, 0.45, z0 + 4.6); }
    });
    // light: sky fill, warm key with soft shadows, cool window light
    scene.add(new T.HemisphereLight(night ? 0x8090c0 : 0xe6eeff, night ? 0x2a2230 : 0x6a5a48, night ? 0.32 : 0.5));
    const key = new T.DirectionalLight(night ? 0xffd9a8 : 0xfff2e0, night ? 0.55 : 1.05); key.position.set(8, 14, 9); key.castShadow = true;
    Object.assign(key.shadow.camera, { left: -13, right: 13, top: 10, bottom: -10, near: 1, far: 45 }); key.shadow.mapSize.set(2048, 2048); key.shadow.bias = -0.0004; scene.add(key);
    const win = new T.DirectionalLight(night ? 0x405080 : 0xcfe6ff, night ? 0.15 : 0.4); win.position.set(0, 5, -14); scene.add(win);
    if (night) { ZONES.forEach((zn, zi) => { if (zn.decay) return; const l = new T.PointLight(0xffd29a, 0.7, 7); l.position.set(zi === 0 ? -2.8 : 2.7, 2.6, 0); scene.add(l); }); }

    // camera from above at an angle; render only on demand
    const view = { az: o.az ?? 0.55, el: o.el ?? 0.76, dist: o.dist ?? 25, tx: 0.4, tz: 0.4 };
    function frame() {
      const w = canvas.clientWidth || 800, h = canvas.clientHeight || 450;
      renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix();
      camera.position.set(view.tx + Math.sin(view.az) * Math.cos(view.el) * view.dist, Math.sin(view.el) * view.dist, view.tz + Math.cos(view.az) * Math.cos(view.el) * view.dist);
      camera.lookAt(view.tx, 0.6, view.tz);
      scene.traverse(m => { if (m.userData.faceCamera) m.rotation.y = Math.atan2(camera.position.x - m.position.x, camera.position.z - m.position.z); });
      const t0 = performance.now(); renderer.render(scene, camera); const ms = performance.now() - t0;
      return { ms, calls: renderer.info.render.calls, triangles: renderer.info.render.triangles, meshes: countMeshes() };
    }
    function countMeshes() { let n = 0; scene.traverse(m => { if (m.isMesh) n++; }); return n; }
    return { frame, view, renderer };
  }

  // ======================================================================
  // C — cutaway building: stacked floors, each room in one-point perspective
  // ======================================================================
  function mockC(canvas, o = {}) {
    const night = !!o.night, dpr = Math.min(2, G.devicePixelRatio || 1);
    const Wc = canvas.clientWidth || 900, Hc = canvas.clientHeight || 560;
    canvas.width = Wc * dpr; canvas.height = Hc * dpr;
    const ctx = canvas.getContext('2d'); ctx.scale(dpr, dpr);
    const poly = (pts, fill, stroke) => { ctx.beginPath(); pts.forEach(([x, y], k) => k ? ctx.lineTo(x, y) : ctx.moveTo(x, y)); ctx.closePath(); if (fill) { ctx.fillStyle = fill; ctx.fill(); } if (stroke) { ctx.strokeStyle = stroke; ctx.stroke(); } };
    // sky behind the building
    const g = ctx.createLinearGradient(0, 0, 0, Hc); g.addColorStop(0, night ? '#0f1736' : '#9fd3f5'); g.addColorStop(1, night ? '#2a3466' : '#e8f4fb'); ctx.fillStyle = g; ctx.fillRect(0, 0, Wc, Hc);
    const r = E.mulberry(9);
    for (let b = 0; b < 22; b++) { const bx = b * 46 - 20 + r() * 10, bh = 120 + r() * 220; ctx.fillStyle = night ? '#1b2245' : '#b8d3e6'; ctx.fillRect(bx, Hc - bh, 40, bh); }
    ctx.fillStyle = night ? '#141a33' : '#7f9db3'; ctx.fillRect(0, Hc - 26, Wc, 26);
    // the building
    const BX = 40, BW = Wc - 150, FH = (Hc - 60) / 3, depth = 0.24;
    const floors = [
      { label: 'Floor 3 · annex', zone: ZONES[2], extra: 'Storage · Phone booths' },
      { label: 'Floor 2', zone: ZONES[1], extra: 'Meeting room · Release crew' },
      { label: 'Floor 1 · lobby', zone: ZONES[0], extra: 'Server room · This Mac' },
    ];
    ctx.fillStyle = night ? '#3a3346' : '#6d6475'; ctx.fillRect(BX - 10, 20, BW + 20, FH * 3 + 14);
    floors.forEach((f, fi) => {
      const y0 = 26 + fi * FH, x0 = BX, x1 = BX + BW, y1 = y0 + FH - 8;
      const vx = (x0 + x1) / 2 + 40, vy = y0 + (y1 - y0) * 0.42;            // vanishing point
      const inset = (x, y) => [x + (vx - x) * depth, y + (vy - y) * depth];
      const [bx0, by0] = inset(x0, y0), [bx1, by1] = inset(x1, y1);
      const wallC = night ? '#8f8a9e' : '#ece7dd', sideC = night ? '#7a7589' : '#ddd6ca', ceilC = night ? '#5b5668' : '#f6f3ee', floorC = hsl(f.zone.hue, 14, night ? 36 : 66);
      poly([[x0, y0], [x1, y0], [bx1, by0], [bx0, by0]], ceilC);
      poly([[x0, y1], [x1, y1], [bx1, by1], [bx0, by1]], floorC);
      poly([[x0, y0], [bx0, by0], [bx0, by1], [x0, y1]], sideC);
      poly([[x1, y0], [bx1, by0], [bx1, by1], [x1, y1]], sideC);
      poly([[bx0, by0], [bx1, by0], [bx1, by1], [bx0, by1]], wallC);
      // windows on the back wall, with the skyline through them
      const bw = bx1 - bx0, bh = by1 - by0;
      for (let k = 0; k < 5; k++) {
        if (k === 2) continue;
        const wx = bx0 + bw * (0.06 + k * 0.19), wy = by0 + bh * 0.18, ww = bw * 0.13, wh = bh * 0.45;
        ctx.fillStyle = '#f6f3ec'; ctx.fillRect(wx - 3, wy - 3, ww + 6, wh + 6);
        const wg = ctx.createLinearGradient(0, wy, 0, wy + wh); wg.addColorStop(0, night ? '#101a40' : '#7cc3ee'); wg.addColorStop(1, night ? '#2a3466' : '#d9eef9'); ctx.fillStyle = wg; ctx.fillRect(wx, wy, ww, wh);
        for (let q = 0; q < 4; q++) { const qh = wh * (0.25 + ((k * 3 + q * 5) % 7) / 14); ctx.fillStyle = night ? '#1f2852' : '#9dbcd2'; ctx.fillRect(wx + q * ww / 4 + 1, wy + wh - qh, ww / 4 - 2, qh); }
        ctx.fillStyle = '#f6f3ec'; ctx.fillRect(wx + ww / 2 - 1, wy, 2, wh);
      }
      // a door in the middle of the back wall, with a sign
      const dx = bx0 + bw * 0.44, dw = bw * 0.1, dy = by0 + bh * 0.28;
      ctx.fillStyle = '#5a5f6e'; ctx.fillRect(dx - 3, dy - 3, dw + 6, by1 - dy + 3); ctx.fillStyle = fi === 1 ? '#bfe0f2' : '#9b6f43'; ctx.fillRect(dx, dy, dw, by1 - dy);
      ctx.fillStyle = '#d9b25c'; ctx.fillRect(dx + dw - 6, dy + (by1 - dy) * 0.5, 3, 3);
      ctx.fillStyle = 'rgba(16,19,30,.85)'; ctx.font = '600 10px ui-sans-serif, system-ui, sans-serif'; const tw = ctx.measureText(f.extra).width; ctx.fillRect(dx + dw / 2 - tw / 2 - 6, dy - 20, tw + 12, 15); ctx.fillStyle = '#e6e9f2'; ctx.textAlign = 'center'; ctx.fillText(f.extra, dx + dw / 2, dy - 9.5); ctx.textAlign = 'left';
      // desks and people at two depths: back row smaller, front row larger
      const rows = [{ t: 0.35, n: 4 }, { t: 0.8, n: 4 }];
      let k = 0;
      rows.forEach(row => {
        const fy = by1 + (y1 - by1) * row.t, sxL = bx0 + (x0 - bx0) * row.t, sxR = bx1 + (x1 - bx1) * row.t, sc = (0.42 + row.t * 0.38) * FH / 180;
        for (let q = 0; q < row.n; q++, k++) {
          const st = f.zone.desks[k % f.zone.desks.length], cx = sxL + (sxR - sxL) * ((q + 0.5) / row.n) + (row.t < 0.5 ? 20 : -10);
          drawDeskC(ctx, cx, fy, sc, st, f.zone, k, night);
        }
      });
      if (f.zone.decay) { ctx.fillStyle = 'rgba(10,12,20,.35)'; poly([[x0, y0], [x1, y0], [x1, y1], [x0, y1]], 'rgba(10,12,20,.32)'); }
      // slab and label
      ctx.fillStyle = night ? '#2a2433' : '#4b4356'; ctx.fillRect(x0 - 6, y1, BW + 12, 8);
      const needs = f.zone.desks.filter(s => BUBBLE[s]).length;
      ctx.fillStyle = 'rgba(16,19,30,.88)'; ctx.fillRect(x1 + 14, y0 + 10, 96, needs ? 40 : 24);
      ctx.fillStyle = '#f3f5f9'; ctx.font = '700 11px ui-sans-serif, system-ui, sans-serif'; ctx.fillText(f.label.split(' · ')[0], x1 + 22, y0 + 26);
      if (needs) { ctx.fillStyle = '#fbbf24'; ctx.font = '600 10px ui-sans-serif, system-ui, sans-serif'; ctx.fillText(`${needs} need you`, x1 + 22, y0 + 42); }
    });
    // elevator shaft on the right
    ctx.fillStyle = night ? '#4b4356' : '#8a8294'; ctx.fillRect(BX + BW + 2, 20, 8, FH * 3 + 14);
    ctx.fillStyle = '#c9cfdb'; ctx.fillRect(BX + BW + 2, 26 + FH + FH * 0.4, 8, 22);
    // roof and sign
    ctx.fillStyle = night ? '#2a2433' : '#4b4356'; ctx.fillRect(BX - 14, 12, BW + 28, 10);
  }
  function drawDeskC(ctx, cx, fy, sc, st, zone, k, night) {
    ctx.save(); ctx.translate(cx, fy); ctx.scale(sc, sc);
    const live = st && !zone.decay;
    // shadow, desk, monitor (screen toward us when the person sits behind)
    ctx.fillStyle = 'rgba(0,0,0,.15)'; ctx.beginPath(); ctx.ellipse(0, 2, 46, 7, 0, 0, 7); ctx.fill();
    const facing = k % 2; // 0: sits behind the desk facing us; 1: sits in front, back to us
    const skin = ['#f6d5bd', '#d29b70', '#8a5634', '#ebbb97'][k % 4], hair = ['#2b1d14', '#b07a3a', '#1f1f24', '#a33b2e'][k % 4], shirt = hsl(zone.hue + (k * 13) % 20, 50, 50);
    const drawPerson = (x, y, pose) => {
      if (pose === 'wave' || pose === 'alarm') {
        ctx.fillStyle = '#2f3a5c'; ctx.fillRect(x - 8, y - 30, 7, 30); ctx.fillRect(x + 1, y - 30, 7, 30);
        ctx.fillStyle = shirt; ctx.beginPath(); ctx.roundRect ? ctx.roundRect(x - 12, y - 62, 24, 34, 8) : ctx.rect(x - 12, y - 62, 24, 34); ctx.fill();
        ctx.strokeStyle = shirt; ctx.lineWidth = 6; ctx.lineCap = 'round'; ctx.beginPath(); ctx.moveTo(x + 10, y - 56); ctx.lineTo(x + 22, y - 76); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(x - 10, y - 56); ctx.lineTo(pose === 'alarm' ? x - 20 : x - 14, pose === 'alarm' ? y - 76 : y - 36); ctx.stroke();
        ctx.fillStyle = skin; ctx.beginPath(); ctx.arc(x, y - 72, 10, 0, 7); ctx.fill(); ctx.fillStyle = hair; ctx.beginPath(); ctx.arc(x, y - 75, 10, Math.PI, 0); ctx.fill();
        ctx.fillStyle = '#2a2230'; ctx.fillRect(x - 4, y - 72, 2, 2); ctx.fillRect(x + 3, y - 72, 2, 2);
        return;
      }
      const slump = pose === 'sleep' ? 10 : 0;
      ctx.fillStyle = shirt; ctx.beginPath(); ctx.roundRect ? ctx.roundRect(x - 12, y - 48 + slump, 24, 30, 8) : ctx.rect(x - 12, y - 48, 24, 30); ctx.fill();
      ctx.fillStyle = skin; ctx.beginPath(); ctx.arc(x, y - 58 + slump * 1.4, 10, 0, 7); ctx.fill();
      ctx.fillStyle = hair; ctx.beginPath(); ctx.arc(x, y - 61 + slump * 1.4, 10, Math.PI, 0); ctx.fill();
      if (facing === 1) { ctx.beginPath(); ctx.arc(x, y - 58 + slump * 1.4, 10, 0, 7); ctx.fill(); }
      else { ctx.fillStyle = '#2a2230'; ctx.fillRect(x - 4, y - 58, 2, 2); ctx.fillRect(x + 3, y - 58, 2, 2); }
      if (pose === 'scratch') { ctx.strokeStyle = shirt; ctx.lineWidth = 6; ctx.lineCap = 'round'; ctx.beginPath(); ctx.moveTo(x + 10, y - 44); ctx.lineTo(x + 12, y - 66); ctx.stroke(); }
    };
    const chair = (x, y) => { ctx.fillStyle = '#3b3f4c'; ctx.fillRect(x - 14, y - 34, 28, 22); ctx.fillRect(x - 2, y - 12, 4, 10); ctx.fillRect(x - 12, y - 3, 24, 3); };
    const desk = () => {
      ctx.fillStyle = zone.decay ? '#a89a86' : '#c99a62'; ctx.fillRect(-44, -30, 88, 8); ctx.fillStyle = zone.decay ? '#8a7d6a' : '#9b6f43'; ctx.fillRect(-44, -22, 88, 22);
      ctx.fillStyle = '#1b1b22'; ctx.fillRect(-16, -56, 32, 22); ctx.fillRect(-2, -34, 4, 4);
      ctx.fillStyle = facing === 0 ? '#2a2a33' : (live ? STATE_SCREEN[st] : '#0b0e14'); ctx.fillRect(-14, -54, 28, 18);
      if (zone.decay) { ctx.fillStyle = '#f4f1ea'; ctx.fillRect(22, -36, 8, 6); ctx.fillStyle = 'rgba(230,226,214,.6)'; for (let q = 0; q < 12; q++) ctx.fillRect(-40 + q * 7, -30 + (q % 3), 2, 1); }
    };
    if (!live) { desk(); chair(zone.decay ? 30 : 0, zone.decay ? 6 : 14); if (zone.decay) { ctx.fillStyle = '#2c2f36'; ctx.beginPath(); ctx.ellipse(-56, -6, 12, 10, 0, 0, 7); ctx.fill(); } ctx.restore(); return; }
    const standing = st === 'asks' || st === 'error';
    const pose = st === 'asks' ? 'wave' : st === 'error' ? 'alarm' : st === 'limit' ? 'sleep' : st === 'stuck' ? 'scratch' : 'sit';
    if (facing === 0 && !standing) { chair(0, -14); drawPerson(0, -14, pose); desk(); }
    else if (standing) { desk(); chair(34, 14); drawPerson(-30, 4, pose); }
    else { desk(); drawPerson(0, 12, pose); chair(0, 18); }
    if (BUBBLE[st]) {
      const [glyph, word, col] = BUBBLE[st]; ctx.font = '700 12px ui-sans-serif, system-ui, sans-serif'; const w = ctx.measureText(word).width + 30, bx = standing ? -30 : 0, by = standing ? -100 : -88;
      ctx.fillStyle = '#fffaf0'; ctx.beginPath(); ctx.roundRect ? ctx.roundRect(bx - w / 2, by - 12, w, 24, 12) : ctx.rect(bx - w / 2, by - 12, w, 24); ctx.fill(); ctx.strokeStyle = col; ctx.lineWidth = 2; ctx.stroke();
      ctx.fillStyle = col; ctx.fillText(glyph, bx - w / 2 + 9, by + 4); ctx.fillStyle = '#1d1726'; ctx.fillText(word, bx - w / 2 + 22, by + 4);
    }
    if (zone.hot && k === 0) { ctx.fillStyle = '#ff7a2f'; ctx.beginPath(); ctx.moveTo(30, -30); ctx.quadraticCurveTo(44, -44, 36, -62); ctx.quadraticCurveTo(28, -46, 24, -30); ctx.fill(); ctx.fillStyle = '#ffd166'; ctx.beginPath(); ctx.moveTo(31, -30); ctx.quadraticCurveTo(38, -40, 34, -50); ctx.quadraticCurveTo(29, -40, 27, -30); ctx.fill(); }
    ctx.restore();
  }

  G.AngledMocks = { mockA, mockB, mockC, ZONES };
})(window);
