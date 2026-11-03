/* The Floor, fixed-angle pixel office: a research mock.
 * A procedural 3D office (three.js) seen from one fixed camera, high above and looking down at ~38°.
 * Pan and zoom only; no rotation. It is rendered at a reduced internal resolution, scaled up with nearest
 * filtering and passed through a small pixel-art shader (tone map, colour steps, depth outlines), so it
 * reads as fine pixel art. Labels and bubbles are drawn crisp on a 2D overlay. Render on demand only.
 */
(function (G) {
  'use strict';
  const E = G.FloorEngine, ZONES = G.AngledMocks.ZONES;
  const hsl = (h, s, l) => E.hslHex(((h % 360) + 360) % 360, s, l);
  const STATE_SCREEN = { working: '#38bdf8', asks: '#fbbf24', error: '#ef4444', limit: '#fb923c', stuck: '#f59e0b', done: '#34d399', idle: '#475569' };
  const BUBBLE = { asks: ['!', 'ASKS YOU', '#d69e2e'], error: ['✕', 'ERROR', '#e53e3e'], limit: ['⌛', 'LIMIT', '#dd6b20'], stuck: ['?', 'STUCK?', '#d69e2e'] };

  function buildOffice(T, night) {
    const scene = new T.Scene(); scene.background = new T.Color(night ? 0x0b0e14 : 0xcfd5df);
    const anchors = [];
    const mats = new Map();
    const M = (c, x = {}) => { const k = c + JSON.stringify(x); if (!mats.has(k)) mats.set(k, new T.MeshStandardMaterial(Object.assign({ color: c, roughness: 0.85, metalness: 0 }, x))); return mats.get(k); };
    const geoBox = new T.BoxGeometry(1, 1, 1);
    const box = (w, h, d, mat, x, y, z, parent = scene, sh = true) => { const m = new T.Mesh(geoBox, mat); m.scale.set(w, h, d); m.position.set(x, y + h / 2, z); m.castShadow = sh; m.receiveShadow = true; parent.add(m); return m; };
    const WX0 = -9, WX1 = 9, WZ0 = -6, WZ1 = 6, WH = 3.0;
    // carpet with a little texture so the pixel pass has something to bite on
    const tex = (w, h, paint) => { const c = document.createElement('canvas'); c.width = w; c.height = h; paint(c.getContext('2d')); const t = new T.CanvasTexture(c); t.wrapS = t.wrapT = T.RepeatWrapping; t.magFilter = T.NearestFilter; return t; };
    const carpetTex = tex(64, 64, x => { const r = E.mulberry(3); x.fillStyle = '#ffffff'; x.fillRect(0, 0, 64, 64); for (let i = 0; i < 500; i++) { x.fillStyle = r() < 0.5 ? '#e6e6ea' : '#f4f4f6'; x.fillRect((r() * 64) | 0, (r() * 64) | 0, 2, 2); } });
    carpetTex.repeat.set(9, 6);
    const floorMat = new T.MeshStandardMaterial({ color: night ? '#77798a' : '#9a9caa', map: carpetTex, roughness: 0.95 });
    box(WX1 - WX0, 0.06, WZ1 - WZ0, floorMat, 0, -0.06, 0, scene, false);
    box(WX1 - WX0 + 0.4, 0.45, WZ1 - WZ0 + 0.4, M('#3a3346'), 0, -0.51, 0, scene, false);          // the floor slab, cut like a doll's house
    // back wall with night-city windows, boards, clocks and the elevator
    const wallM = M(night ? '#cbbfb0' : '#e2dacb');
    box(WX1 - WX0 + 5, WH, 0.25, wallM, -2.5, 0, WZ0);
    box(WX1 - WX0 + 5, 0.14, 0.06, M('#8f8370'), -2.5, 0, WZ0 + 0.15);
    const sky = document.createElement('canvas'); sky.width = 192; sky.height = 120; const sx = sky.getContext('2d');
    const g = sx.createLinearGradient(0, 0, 0, 120); g.addColorStop(0, night ? '#0f1736' : '#79c0ec'); g.addColorStop(1, night ? '#33407a' : '#e2f2fb'); sx.fillStyle = g; sx.fillRect(0, 0, 192, 120);
    const r = E.mulberry(17);
    for (let b = 0; b < 12; b++) { const bx = b * 16 + r() * 4, bh = 30 + r() * 70; sx.fillStyle = night ? '#18203f' : '#8eb0c9'; sx.fillRect(bx, 120 - bh, 14, bh); sx.fillStyle = night ? '#ffd27a' : '#cfe5f3'; for (let q = 0; q < 14; q++) if (r() < (night ? 0.4 : 0.5)) sx.fillRect(bx + 2 + (q % 3) * 4, 120 - bh + 4 + ((q / 3) | 0) * 7, 2, 3); }
    const skyTex = new T.CanvasTexture(sky); skyTex.magFilter = T.NearestFilter;
    const glassM = new T.MeshBasicMaterial({ map: skyTex });
    const frameM = M('#f3efe6');
    const windowAt = (x, w) => {
      const gl = new T.Mesh(new T.PlaneGeometry(w, 1.4), glassM); gl.position.set(x, 1.65, WZ0 + 0.13); scene.add(gl);
      box(w + 0.2, 0.08, 0.14, frameM, x, 0.9, WZ0 + 0.14); box(w + 0.2, 0.08, 0.14, frameM, x, 2.35, WZ0 + 0.14);
      box(0.08, 1.45, 0.14, frameM, x - w / 2 - 0.05, 0.92, WZ0 + 0.14); box(0.08, 1.45, 0.14, frameM, x + w / 2 + 0.05, 0.92, WZ0 + 0.14); box(0.05, 1.4, 0.12, frameM, x, 0.95, WZ0 + 0.15);
    };
    windowAt(-12.0, 2.4); windowAt(-7.0, 2.2); windowAt(-4.3, 2.2); windowAt(6.6, 3.2);
    box(2.2, 1.1, 0.05, M('#f7f8fb', { roughness: 0.3 }), -1.4, 1.05, WZ0 + 0.15); box(2.3, 0.05, 0.14, M('#8d94a3'), -1.4, 1.02, WZ0 + 0.17);
    ['#ffe28a', '#ffe28a', '#8fd3ff', '#9be8b5', '#8fd3ff', '#ffe28a'].forEach((c, k) => box(0.24, 0.2, 0.01, M(c), -2.1 + (k % 3) * 0.7, 1.65 - ((k / 3) | 0) * 0.36, WZ0 + 0.18, scene, false));
    box(1.4, 0.95, 0.05, M('#c08a52'), 1.0, 1.15, WZ0 + 0.15);
    ['#ffd0cc', '#fff1b8', '#f2efe6', '#fff1b8'].forEach((c, k) => box(0.26, 0.22, 0.01, M(c), 0.7 + (k % 2) * 0.6, 1.75 - ((k / 2) | 0) * 0.42, WZ0 + 0.18, scene, false));
    for (let k = 0; k < 3; k++) { const c = new T.Mesh(new T.CylinderGeometry(0.17, 0.17, 0.04, 16), M('#ffffff')); c.rotation.x = Math.PI / 2; c.position.set(-1.9 + k * 0.6, 2.5, WZ0 + 0.16); scene.add(c); const h = box(0.02, 0.12, 0.01, M('#222'), -1.9 + k * 0.6, 2.47, WZ0 + 0.19, scene, false); h.rotation.z = k * 1.3; }
    box(1.7, 2.3, 0.08, M('#8d94a3', { metalness: 0.4, roughness: 0.4 }), 3.6, 0, WZ0 + 0.15); box(0.03, 2.2, 0.1, M('#5a5f6e'), 3.6, 0, WZ0 + 0.18);
    box(0.5, 0.22, 0.05, M('#222'), 3.6, 2.45, WZ0 + 0.16); box(0.12, 0.12, 0.02, M('#000', { emissive: '#ffb347', emissiveIntensity: 1.5 }), 3.6, 2.5, WZ0 + 0.2, scene, false);
    // left: a glass wall, with the meeting room and the server room seen through it
    const glassWall = new T.MeshStandardMaterial({ color: 0xbfe0f2, transparent: true, opacity: 0.22, roughness: 0.1, metalness: 0.1 });
    box(0.06, WH - 0.2, WZ1 - WZ0, glassWall, WX0, 0, 0, scene, false);
    for (let z = WZ0; z <= WZ1; z += 2) box(0.1, WH - 0.2, 0.1, frameM, WX0, 0, z);
    box(0.1, 0.1, WZ1 - WZ0, frameM, WX0, WH - 0.2, 0);
    box(5, 0.06, 6.2, M(night ? '#5d5368' : '#7d6f8c'), -11.5, -0.06, -2.9, scene, false);
    box(5, 0.06, 5.8, M(night ? '#3c4150' : '#5a6070'), -11.5, -0.06, 3.1, scene, false);
    box(5.4, 0.45, 12.4, M('#3a3346'), -11.5, -0.51, 0, scene, false);
    box(5, WH - 0.2, 0.06, glassWall, -11.5, 0, 0.2, scene, false);
    box(2.6, 0.06, 1.2, M('#e9e7e1'), -11.5, 0.72, -3.0); box(0.12, 0.72, 0.12, M('#555'), -11.5, 0, -3.0);
    const chair = (x, z, rotY, color, askew = 0) => { const gp = new T.Group(); gp.position.set(x, 0, z); gp.rotation.y = rotY + askew; scene.add(gp); const m = M(color); box(0.5, 0.07, 0.48, m, 0, 0.44, 0, gp); box(0.5, 0.5, 0.07, m, 0, 0.55, 0.22, gp); box(0.06, 0.42, 0.06, M('#333'), 0, 0.02, 0, gp); box(0.5, 0.04, 0.5, M('#333'), 0, 0, 0, gp); return gp; };
    for (let k = 0; k < 6; k++) chair(-12.6 + (k % 3) * 1.1, k < 3 ? -3.9 : -2.1, k < 3 ? 0 : Math.PI, '#6a4a7a');
    box(1.6, 0.9, 0.06, M('#22384d', { emissive: '#2b5b80', emissiveIntensity: 0.9 }), -11.5, 1.3, WZ0 + 0.16, scene, false);
    for (let k = 0; k < 3; k++) { const rz = 2.0 + k * 1.1; box(0.8, 2.0, 0.8, M('#1f2230', { roughness: 0.5 }), -13.2, 0, rz); for (let q = 0; q < 8; q++) box(0.02, 0.05, 0.5, M('#0b0e14', { emissive: q < 5 ? '#34d399' : '#30364a', emissiveIntensity: 1.6 }), -12.79, 0.3 + q * 0.2, rz, scene, false); }
    anchors.push({ kind: 'room', pos: new T.Vector3(-11.5, 2.6, -3.0), text: 'Release crew · 3' });
    anchors.push({ kind: 'room', pos: new T.Vector3(-11.5, 2.6, 3.2), text: 'This Mac · 42% CPU' });
    // right: the wall is cut down to a stub; a lounge with beanbags and a kitchen along the back
    box(0.15, 0.35, WZ1 - WZ0, M('#d8d2c6'), WX1, 0, 0); box(WX1 - WX0, 0.35, 0.15, M('#d8d2c6'), 0, 0, WZ1);
    box(3.9, 0.92, 0.7, M('#e9e2d3'), 6.9, 0, WZ0 + 0.5); box(3.9, 0.05, 0.75, M('#bfb5a2'), 6.9, 0.92, WZ0 + 0.5);
    box(3.9, 0.7, 0.4, M('#e9e2d3'), 6.9, 2.1, WZ0 + 0.3);
    box(0.8, 2.0, 0.75, M('#e8ecf2', { roughness: 0.3, metalness: 0.2 }), 8.5, 0, WZ0 + 0.5);
    box(0.35, 0.45, 0.35, M('#3a3a44'), 5.6, 0.97, WZ0 + 0.5); box(0.6, 0.06, 0.4, M('#c9cfdb', { metalness: 0.6, roughness: 0.3 }), 6.9, 0.97, WZ0 + 0.5);
    const table = new T.Mesh(new T.CylinderGeometry(0.55, 0.55, 0.05, 20), M('#cf9f68')); table.position.set(6.8, 0.74, -3.4); table.castShadow = true; scene.add(table); box(0.08, 0.72, 0.08, M('#555'), 6.8, 0, -3.4);
    for (let k = 0; k < 2; k++) { const st = new T.Mesh(new T.CylinderGeometry(0.2, 0.2, 0.06, 12), M('#3b3f4c')); st.position.set(6.0 + k * 1.6, 0.62, -3.4); scene.add(st); box(0.05, 0.6, 0.05, M('#555'), 6.0 + k * 1.6, 0, -3.4); }
    const rug = new T.Mesh(new T.CylinderGeometry(2.0, 2.0, 0.02, 28), M(night ? '#7a4a5e' : '#b56f8c')); rug.position.set(6.7, 0.01, 3.6); rug.receiveShadow = true; scene.add(rug);
    ['#e85d75', '#f2b33d', '#4fb3a9'].forEach((c, k) => { const bb = new T.Mesh(new T.SphereGeometry(0.5, 16, 12), M(c, { roughness: 0.6 })); bb.scale.set(1, 0.62, 1); bb.position.set(5.6 + k * 1.15, 0.3, 3.0 + (k % 2) * 1.2); bb.castShadow = true; bb.receiveShadow = true; scene.add(bb); });
    box(0.8, 0.35, 0.8, M('#cf9f68'), 6.8, 0, 4.6);
    box(0.06, 1.7, 0.06, M('#333'), 8.4, 0, 5.3); const shade = new T.Mesh(new T.ConeGeometry(0.28, 0.32, 16, 1, true), M('#f3e3c2', { emissive: night ? '#ffcf87' : '#000', emissiveIntensity: 0.6, side: T.DoubleSide })); shade.position.set(8.4, 1.85, 5.3); scene.add(shade);
    anchors.push({ kind: 'room', pos: new T.Vector3(6.8, 1.6, -4.6), text: 'Kitchen' });
    anchors.push({ kind: 'room', pos: new T.Vector3(6.7, 1.4, 3.4), text: 'Lounge' });
    // plants
    const leaf = M('#3f9b55', { flatShading: true }), pot = M('#b4643a');
    const plant = (x, z, s = 1) => { const p = new T.Mesh(new T.CylinderGeometry(0.22 * s, 0.17 * s, 0.4 * s, 10), pot); p.position.set(x, 0.2 * s, z); p.castShadow = true; scene.add(p); const f = new T.Mesh(new T.IcosahedronGeometry(0.45 * s, 0), leaf); f.position.set(x, 0.75 * s, z); f.castShadow = true; scene.add(f); };
    plant(-8.3, -5.4, 1.2); plant(2.4, -5.4); plant(8.5, 5.6, 1.1); plant(-8.4, 5.5); plant(4.6, 2.2, 0.9);

    // desks, people and their state
    const SKIN = ['#f6d5bd', '#ebbb97', '#d29b70', '#b07650', '#8a5634', '#62391f'], HAIR = ['#2b1d14', '#4a3020', '#7a4b25', '#b07a3a', '#dcb766', '#e6e1d6', '#a33b2e', '#1f1f24'];
    function person(seed, hue, pose, faceUs) {
      const r2 = E.mulberry(E.hash32('px3d|' + seed));
      const gp = new T.Group();
      const shirt = M(hsl(hue + r2() * 20 - 10, 55, 48 + r2() * 10)), skin = M(SKIN[(r2() * SKIN.length) | 0]), hair = M(HAIR[(r2() * HAIR.length) | 0]), pants = M(['#2f3a5c', '#3a3a42', '#7a6a4a', '#3d5a80'][(r2() * 4) | 0]);
      const sit = pose !== 'wave' && pose !== 'alarm';
      const hip = sit ? 0.48 : 0.86;
      if (sit) { box(0.16, 0.14, 0.45, pants, -0.1, hip, -0.2, gp); box(0.16, 0.14, 0.45, pants, 0.1, hip, -0.2, gp); box(0.15, 0.45, 0.15, pants, -0.1, 0.02, -0.4, gp); box(0.15, 0.45, 0.15, pants, 0.1, 0.02, -0.4, gp); }
      else { box(0.16, hip, 0.16, pants, -0.1, 0, 0, gp); box(0.16, hip, 0.16, pants, 0.1, 0, 0, gp); }
      const lean = pose === 'sleep' ? 0.55 : pose === 'lean' ? -0.25 : 0;
      const torso = new T.Group(); torso.position.set(0, hip + 0.06, 0); torso.rotation.x = -lean; gp.add(torso);
      box(0.46, 0.55, 0.28, shirt, 0, 0, 0, torso);
      const head = new T.Mesh(new T.SphereGeometry(0.17, 14, 10), skin); head.position.set(0, 0.76, 0); head.castShadow = true; torso.add(head);
      const hr = new T.Mesh(new T.SphereGeometry(0.182, 14, 10, 0, Math.PI * 2, 0, Math.PI * (0.45 + r2() * 0.15)), hair); hr.position.set(0, 0.78, 0.01); torso.add(hr);
      if (faceUs) { box(0.04, 0.05, 0.02, M('#1d1726'), -0.06, 0.75, -0.165, torso, false); box(0.04, 0.05, 0.02, M('#1d1726'), 0.06, 0.75, -0.165, torso, false); }
      const arm = (side, up) => { const a = new T.Group(); a.position.set(side * 0.29, 0.52, 0); torso.add(a); const m = new T.Mesh(geoBox, shirt); m.scale.set(0.12, 0.48, 0.12); m.position.set(0, -0.24, 0); m.castShadow = true; a.add(m); const h = new T.Mesh(new T.SphereGeometry(0.065, 8, 6), skin); h.position.set(0, -0.5, 0); a.add(h); a.rotation.x = up ? 0 : (sit ? -1.1 : 0); if (up) a.rotation.z = side * 2.7; return a; };
      arm(-1, pose === 'alarm' || pose === 'wave'); arm(1, pose === 'wave' || pose === 'alarm' || pose === 'scratch');
      return gp;
    }
    const layout = [{ x0: -7.6, pods: [-3.7, 0.5] }, { x0: -2.6, pods: [-3.7, 0.5] }, { x0: 1.4, pods: [1.4] }];
    ZONES.forEach((zn, zi) => {
      const L = layout[zi];
      const depth = L.pods.length * 4.2;
      const cz = L.pods[0] - 1.0 + depth / 2;
      box(3.9, 0.02, depth, M(hsl(zn.hue, 34, night ? 40 : 58), { map: carpetTex }), L.x0 + 0.78, 0, cz, scene, false);
      const working = zn.desks.filter(s => s === 'working').length, needs = zn.desks.filter(s => BUBBLE[s]).length;
      anchors.push({ kind: 'zone', pos: new T.Vector3(L.x0 + 0.78, 2.5, L.pods[0] - 0.9), text: zn.name, sub: zn.decay ? 'Unused 9 days' : `${working} working${needs ? ` · ${needs} need you` : ''}`, color: hsl(zn.hue, 60, 55), needs });
      zn.desks.forEach((st, k) => {
        const pod = (k / 4) | 0, kk = k % 4;
        if (pod >= L.pods.length) return;
        const dx = L.x0 + (kk % 2) * 1.55, dz = L.pods[pod] + ((kk / 2) | 0) * 0.82;
        const facing = (kk / 2) | 0; // 0: far side, faces the camera; 1: near side, back to camera
        const deskM = M(zn.decay ? '#b3a996' : '#d6b282'), legM = M('#555');
        box(1.45, 0.05, 0.78, deskM, dx, 0.72, dz); box(0.05, 0.72, 0.7, legM, dx - 0.68, 0, dz); box(0.05, 0.72, 0.7, legM, dx + 0.68, 0, dz);
        const live = st && !zn.decay;
        const mon = new T.Group(); mon.position.set(dx, 0.77, dz + (facing ? 0.12 : -0.12)); mon.rotation.y = facing ? Math.PI : 0; scene.add(mon);
        box(0.62, 0.38, 0.04, M('#1b1b22'), 0, 0.14, 0, mon); box(0.56, 0.32, 0.01, M('#000', { emissive: live ? STATE_SCREEN[st] : '#000', emissiveIntensity: live ? 1.3 : 0 }), 0, 0.17, facing ? -0.025 : 0.025, mon, false); box(0.05, 0.14, 0.05, M('#333'), 0, 0, 0, mon);
        box(0.42, 0.02, 0.14, M('#d9dde5'), dx, 0.77, dz + (facing ? -0.22 : 0.22));
        const cz2 = dz + (facing ? -0.62 : 0.62), rotY = facing ? 0 : Math.PI;
        if (!live) {
          chair(dx + (zn.decay ? 0.3 : 0), cz2 + (zn.decay ? (facing ? -0.3 : 0.3) : 0), rotY, '#3b3f4c', zn.decay ? 0.6 + k * 0.3 : 0);
          if (zn.decay) { box(0.18, 0.1, 0.12, M('#f4f1ea'), dx - 0.4, 0.77, dz); }
          return;
        }
        const standing = st === 'asks' || st === 'error';
        if (standing) {
          chair(dx + 0.55, cz2, rotY, '#3b3f4c', 0.5);
          const p = person(zi * 10 + k, zn.hue, st === 'asks' ? 'wave' : 'alarm', true); p.position.set(dx - 0.25, 0, cz2 + (facing ? -0.2 : 0.35)); scene.add(p);
          anchors.push({ kind: 'bubble', state: st, pos: new T.Vector3(dx - 0.25, 2.25, cz2 + (facing ? -0.2 : 0.35)) });
        } else {
          chair(dx, cz2, rotY, '#3b3f4c');
          const p = person(zi * 10 + k, zn.hue, st === 'limit' ? 'sleep' : st === 'stuck' ? 'scratch' : st === 'done' ? 'lean' : 'sit', facing === 0); p.position.set(dx, 0, cz2); p.rotation.y = facing ? Math.PI : 0; scene.add(p);
          if (st === 'stuck' || st === 'limit') anchors.push({ kind: 'bubble', state: st, pos: new T.Vector3(dx, 1.95, cz2) });
        }
        for (let q = 0; q < Math.min(3, (zn.subs || [])[k] || 0); q++) {
          const d = new T.Mesh(new T.SphereGeometry(0.1, 10, 8), M(hsl(zn.hue, 60, 60), { emissive: hsl(zn.hue, 70, 55), emissiveIntensity: 0.7 }));
          d.position.set(dx - 0.6 + q * 0.3, 1.55 + (q % 2) * 0.12, dz); d.castShadow = true; scene.add(d);
        }
        if (zn.hot && k === 0) {
          const f1 = new T.Mesh(new T.ConeGeometry(0.17, 0.45, 7), M('#ff7a2f', { emissive: '#ff5a1a', emissiveIntensity: 1.8 })); f1.position.set(dx + 0.5, 0.98, dz); scene.add(f1);
          const f2 = new T.Mesh(new T.ConeGeometry(0.08, 0.26, 7), M('#ffd166', { emissive: '#ffd166', emissiveIntensity: 2 })); f2.position.set(dx + 0.5, 0.92, dz); scene.add(f2);
          const pl = new T.PointLight(0xff7a2f, 1.4, 3.5); pl.position.set(dx + 0.5, 1.3, dz); scene.add(pl);
        }
      });
      if (zn.decay >= 2) { for (let q = 0; q < 4; q++) { const tb = new T.Mesh(new T.SphereGeometry(0.28, 8, 6), M('#2c2f36', { roughness: 0.4 })); tb.scale.y = 0.8; tb.position.set(L.x0 - 0.3 + q * 0.6, 0.22, L.pods[0] + 2.5 + (q % 2) * 0.4); tb.castShadow = true; scene.add(tb); } box(0.6, 0.45, 0.5, M('#b98b57'), L.x0 + 2.4, 0, L.pods[0] + 2.6); }
    });
    // light: night is lamps and screens; day is the sun through the windows
    scene.add(new T.HemisphereLight(night ? 0x6a78b0 : 0xe6eeff, night ? 0x2a2230 : 0x6a5a48, night ? 0.35 : 0.55));
    const key = new T.DirectionalLight(night ? 0xffd9a8 : 0xfff2e0, night ? 0.55 : 1.0); key.position.set(-6, 14, 10); key.castShadow = true;
    Object.assign(key.shadow.camera, { left: -16, right: 12, top: 10, bottom: -10, near: 1, far: 45 }); key.shadow.mapSize.set(1024, 1024); key.shadow.bias = -0.0005; scene.add(key);
    if (night) [[-6.8, -1.5], [-1.8, -1.5], [2.2, 2], [6.7, 3.5], [6.8, -3.5], [-11.5, -3]].forEach(([x, z]) => { const l = new T.PointLight(0xffcf8a, 0.75, 7.5); l.position.set(x, 2.7, z); scene.add(l); });
    return { scene, anchors };
  }

  const POST_VS = 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }';
  const POST_FS = `
    uniform sampler2D tColor; uniform sampler2D tDepth; uniform vec2 res; uniform float near; uniform float far;
    uniform float levels; uniform float outline; uniform float pixel; uniform float expo;
    varying vec2 vUv;
    float lin(float d){ float z = d * 2.0 - 1.0; return (2.0 * near * far) / (far + near - z * (far - near)); }
    vec3 aces(vec3 x){ return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0); }
    void main(){
      vec3 c = texture2D(tColor, vUv).rgb;
      c = aces(c * expo);
      if (pixel > 0.5) {
        vec2 px = 1.0 / res;
        float d = lin(texture2D(tDepth, vUv).x);
        float m = 0.0;
        m = max(m, lin(texture2D(tDepth, vUv + vec2(px.x, 0.0)).x) - d);
        m = max(m, lin(texture2D(tDepth, vUv - vec2(px.x, 0.0)).x) - d);
        m = max(m, lin(texture2D(tDepth, vUv + vec2(0.0, px.y)).x) - d);
        m = max(m, lin(texture2D(tDepth, vUv - vec2(0.0, px.y)).x) - d);
        float edge = step(0.12 + d * 0.012, m);
        c = floor(c * levels + 0.5) / levels;
        c = mix(c, c * 0.42, edge * outline);
      }
      gl_FragColor = vec4(c, 1.0);
    }`;

  function PixelOffice(canvas, overlay, o = {}) {
    const T = G.THREE;
    const renderer = new T.WebGLRenderer({ canvas, antialias: false, powerPreference: 'low-power' });
    const dpr = Math.min(2, G.devicePixelRatio || 1);
    renderer.setPixelRatio(dpr);
    renderer.shadowMap.enabled = true; renderer.shadowMap.type = T.PCFShadowMap;
    renderer.shadowMap.autoUpdate = false;                       // shadows are drawn once, then reused
    const st = { night: o.night ?? true, pixel: o.pixel ?? 3, outline: 1, levels: 28, zoom: 1, panX: 0, panZ: 0 };
    let built = buildOffice(T, st.night);
    renderer.shadowMap.needsUpdate = true;
    const camera = new T.PerspectiveCamera(26, 1, 1, 140);
    let rt = null;
    const post = new T.ShaderMaterial({ vertexShader: POST_VS, fragmentShader: POST_FS, uniforms: { tColor: { value: null }, tDepth: { value: null }, res: { value: new T.Vector2() }, near: { value: 1 }, far: { value: 140 }, levels: { value: 28 }, outline: { value: 1 }, pixel: { value: 1 }, expo: { value: 1.35 } }, depthTest: false, depthWrite: false });
    const postScene = new T.Scene(); postScene.add(new T.Mesh(new T.PlaneGeometry(2, 2), post));
    const postCam = new T.OrthographicCamera(-1, 1, 1, -1, 0, 1);
    const octx = overlay.getContext('2d');
    function target(lw, lh) {
      if (rt && rt.width === lw && rt.height === lh) return rt;
      if (rt) rt.dispose();
      rt = new T.WebGLRenderTarget(lw, lh, { minFilter: T.NearestFilter, magFilter: T.NearestFilter, depthBuffer: true });
      rt.depthTexture = new T.DepthTexture(lw, lh); rt.depthTexture.type = T.UnsignedIntType;
      return rt;
    }
    function placeCamera(w, h) {
      // fixed angle: high above, looking down ~38°, straight at the back wall. Pan moves the target; zoom moves along the ray.
      const el = 0.68, dist = 29.5 / st.zoom, tx = -2.3 + st.panX, tz = -0.6 + st.panZ;
      camera.aspect = w / h; camera.updateProjectionMatrix();
      camera.position.set(tx, Math.sin(el) * dist, tz + Math.cos(el) * dist); camera.lookAt(tx, 0.4, tz);
    }
    function frame(finish) {
      const w = canvas.clientWidth || 800, h = canvas.clientHeight || 450;
      renderer.setSize(w, h, false); placeCamera(w, h);
      const k = Math.max(1, st.pixel);                          // device pixels per art pixel
      const lw = Math.max(1, Math.floor(w * dpr / k)), lh = Math.max(1, Math.floor(h * dpr / k));
      const t = target(lw, lh);
      const t0 = performance.now();
      renderer.setRenderTarget(t); renderer.render(built.scene, camera);
      post.uniforms.tColor.value = t.texture; post.uniforms.tDepth.value = t.depthTexture; post.uniforms.res.value.set(lw, lh);
      post.uniforms.pixel.value = k > 1 ? 1 : 0; post.uniforms.outline.value = st.outline; post.uniforms.levels.value = st.levels; post.uniforms.expo.value = st.night ? 1.35 : 0.95;
      renderer.setRenderTarget(null); renderer.render(postScene, postCam);
      if (finish) renderer.getContext().finish();
      const ms = performance.now() - t0;
      drawOverlay(w, h);
      return { ms, lw, lh, calls: renderer.info.render.calls };
    }
    function drawOverlay(w, h) {
      overlay.width = Math.round(w * dpr); overlay.height = Math.round(h * dpr);
      octx.setTransform(dpr, 0, 0, dpr, 0, 0); octx.clearRect(0, 0, w, h);
      const font = '"Silkscreen", ui-monospace, monospace';
      const pr = (v) => { const p = v.clone().project(camera); return [(p.x + 1) / 2 * w, (1 - p.y) / 2 * h, p.z]; };
      const placed = [];
      const free = (x0, y0, w0, h0) => { let y1 = y0; for (let i = 0; i < 6; i++) { const hit = placed.find(p => x0 < p[0] + p[2] && x0 + w0 > p[0] && y1 < p[1] + p[3] && y1 + h0 > p[1]); if (!hit) break; y1 = hit[1] - h0 - 4; } placed.push([x0, y1, w0, h0]); return y1; };
      for (const a of [...built.anchors].sort((p, q) => (p.kind === 'zone') - (q.kind === 'zone'))) {
        const [x, y, z] = pr(a.pos); if (z > 1) continue;
        if (a.kind === 'zone') {
          octx.font = `400 11px ${font}`; const t1 = a.text.toUpperCase(); const w1 = octx.measureText(t1).width;
          octx.font = `400 9px ${font}`; const t2 = a.sub.toUpperCase(); const w2 = octx.measureText(t2).width;
          const bw = Math.max(w1, w2) + 30, bh = 34, bx = Math.round(x - bw / 2), by = Math.round(free(x - bw / 2, y - bh, bw, bh));
          octx.fillStyle = 'rgba(16,19,30,.92)'; octx.fillRect(bx, by, bw, bh); octx.fillStyle = 'rgba(255,255,255,.12)'; octx.fillRect(bx, by, bw, 2);
          octx.fillStyle = a.color; octx.fillRect(bx + 7, by + 7, 10, 10);
          octx.fillStyle = '#f3f5f9'; octx.font = `400 11px ${font}`; octx.textBaseline = 'top'; octx.fillText(t1, bx + 23, by + 6);
          octx.fillStyle = a.needs ? '#ffd98a' : '#aab3c2'; octx.font = `400 9px ${font}`; octx.fillText(t2, bx + 23, by + 21);
          octx.fillStyle = 'rgba(16,19,30,.92)'; octx.fillRect(Math.round(x) - 3, by + bh, 6, 4);
        } else if (a.kind === 'room') {
          octx.font = `400 9px ${font}`; const t1 = a.text.toUpperCase(); const bw = octx.measureText(t1).width + 14;
          octx.fillStyle = 'rgba(16,19,30,.8)'; octx.fillRect(Math.round(x - bw / 2), Math.round(y - 9), bw, 17);
          octx.fillStyle = '#e6e9f2'; octx.textBaseline = 'middle'; octx.fillText(t1, Math.round(x - bw / 2 + 7), Math.round(y));
        } else {
          const [glyph, word, col] = BUBBLE[a.state]; octx.font = `400 10px ${font}`; const bw = octx.measureText(word).width + 30, bh = 20, bx = Math.round(x - bw / 2), by0 = Math.round(y - bh - 6), by = Math.round(free(bx - 2, by0 - 2, bw + 4, bh + 9) + 2);
          octx.fillStyle = col; octx.fillRect(bx - 2, by - 2, bw + 4, bh + 4); octx.fillStyle = '#fffaf0'; octx.fillRect(bx, by, bw, bh);
          octx.fillRect(Math.round(x) - 3, by + bh, 6, 3); octx.fillStyle = col; octx.fillRect(Math.round(x) - 3, by + bh + 3, 6, 2);
          octx.fillStyle = col; octx.font = '800 12px ui-sans-serif, system-ui, sans-serif'; octx.textBaseline = 'middle'; octx.fillText(glyph, bx + 7, by + bh / 2 + 1);
          octx.fillStyle = '#1d1726'; octx.font = `400 10px ${font}`; octx.fillText(word, bx + 21, by + bh / 2 + 1);
        }
      }
    }
    function rebuild() { built = buildOffice(T, st.night); renderer.shadowMap.needsUpdate = true; }
    function measure(sizes, n = 24) {
      const out = [];
      for (const k of sizes) {
        st.pixel = k; frame(true); renderer.shadowMap.needsUpdate = false;
        let tot = 0; for (let i = 0; i < n; i++) tot += frame(true).ms;
        const f = frame(true); out.push({ k, ms: tot / n, lw: f.lw, lh: f.lh });
      }
      return out;
    }
    return { st, frame, rebuild, measure, renderer, dpr };
  }

  G.PixelOffice = PixelOffice;
})(window);
