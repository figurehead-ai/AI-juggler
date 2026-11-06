//! The Floor's shaders, in WGSL: the scene (with the sun's shadow, a pick
//! id per pixel, banded sunlight for a painted look, point lights, glow
//! and the cutaway of walls between the camera and the room), the post
//! pass (hard-edged upscale, outlines, the hovered thing outlined, sky,
//! the red-alert wash, a soft vignette) and the overlay. In Full
//! resolution (D-80) the scene is at device pixels: textures sharp-
//! bilinear, edges smoothed, outlines as wide as Pixel art's look.

/// The scene and shadow passes, with the materials' table
/// ([`juggler_floor::materials::wgsl_table`]) in place.
#[must_use]
pub fn scene_wgsl() -> String {
    SCENE_WGSL.replace("//MATERIALS//", &juggler_floor::materials::wgsl_table())
}

/// The scene and shadow passes (the materials' table still to fill in).
const SCENE_WGSL: &str = r"
struct G { vp: mat4x4<f32>, lvp: mat4x4<f32>, eye: vec4<f32>, sun: vec4<f32>, sky: vec4<f32>, ground: vec4<f32>, params: vec4<f32>, sunc: vec4<f32>, look: vec4<f32>, mat: vec4<f32>, atl: vec4<f32> };
struct Lt { p: vec4<f32>, c: vec4<f32> };
@group(0) @binding(0) var<uniform> g: G;
@group(0) @binding(1) var<storage, read> lights: array<Lt>;
@group(0) @binding(2) var atlas: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var smap: texture_depth_2d;
@group(0) @binding(5) var ssamp: sampler_comparison;
@group(0) @binding(6) var mats: texture_2d_array<f32>;
@group(0) @binding(7) var msamp: sampler;
//MATERIALS//
struct VI {
  @location(0) pos: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
  @location(7) col: vec4<f32>, @location(8) tex: vec4<f32>, @location(9) misc: vec4<f32>, @location(10) off: vec2<f32>, @location(11) id: u32,
  @location(12) mat: u32,
};
struct VO {
  @builtin(position) pos: vec4<f32>,
  @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) col: vec4<f32>, @location(4) tex: vec4<f32>, @location(5) misc: vec4<f32>, @location(6) off: vec2<f32>,
  @location(7) @interpolate(flat) id: u32,
  // 0.4.1's second pass: where the pixel is in the part's own space (in
  // metres, or its unit shape for a leaf or a face), its normal there,
  // the part's turn and scale, and its material
  @location(8) lp: vec3<f32>, @location(9) ln: vec3<f32>,
  @location(10) r0: vec3<f32>, @location(11) r1: vec3<f32>, @location(12) r2: vec3<f32>,
  @location(13) sc: vec3<f32>, @location(14) @interpolate(flat) mat: u32,
};
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
const DISK = array<vec2<f32>, 8>(
  vec2<f32>(-0.613, 0.617), vec2<f32>(0.170, -0.040), vec2<f32>(-0.299, 0.791), vec2<f32>(0.645, 0.493),
  vec2<f32>(-0.651, -0.717), vec2<f32>(0.421, 0.027), vec2<f32>(-0.817, -0.271), vec2<f32>(-0.705, -0.668));
// things on a wall vanish with it when the camera is behind that wall (the cutaway)
fn culled(v: VI) -> bool {
  let flags = u32(v.misc.w + 0.5);
  if ((flags & 2u) == 0u) { return false; }
  return dot(g.eye.xyz - v.m3.xyz, normalize(v.m2.xyz)) < 0.0;
}
@vertex fn vs(v: VI) -> VO {
  let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
  let wp = (m * vec4<f32>(v.pos, 1.0)).xyz;
  var o: VO;
  o.wp = wp;
  o.n = (m * vec4<f32>(v.n, 0.0)).xyz;
  o.pos = g.vp * vec4<f32>(wp, 1.0);
  if (culled(v)) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  o.uv = v.uv; o.col = v.col; o.tex = v.tex; o.misc = v.misc; o.off = v.off; o.id = v.id;
  // the atlas keeps only its rows in use: a place's v scaled to them
  if ((u32(v.misc.w + 0.5) & 1u) != 0u) { o.tex = vec4<f32>(v.tex.x, v.tex.y * g.atl.x, v.tex.z, v.tex.w * g.atl.x); }
  let sc = max(vec3<f32>(length(v.m0.xyz), length(v.m1.xyz), length(v.m2.xyz)), vec3<f32>(1e-5));
  let unit = MKIND[min(v.mat, 17u)].x < 0.0;
  o.lp = select(v.pos * sc, v.pos + vec3<f32>(0.5, 0.0, 0.0), unit);
  o.ln = select(normalize(v.n / sc), v.n, unit);
  o.r0 = v.m0.xyz / sc.x; o.r1 = v.m1.xyz / sc.y; o.r2 = v.m2.xyz / sc.z;
  o.sc = sc; o.mat = v.mat;
  return o;
}
@vertex fn vs_shadow(v: VI) -> @builtin(position) vec4<f32> {
  let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
  if (culled(v)) { return vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  return g.lvp * m * vec4<f32>(v.pos, 1.0);
}
struct FO { @location(0) c: vec4<f32>, @location(1) id: u32 };
// Full resolution (D-80): a pixel texture's texels stay crisp, their
// seams smoothed over one screen pixel ('sharp bilinear'), within its own
// tile of the atlas so a neighbour never bleeds in.
fn sharp(v: VO, p: vec2<f32>, w: vec2<f32>) -> vec4<f32> {
  let dims = vec2<f32>(textureDimensions(atlas));
  let lo = vec2<i32>(v.tex.xy * dims);
  let hi = max(vec2<i32>((v.tex.xy + v.tex.zw) * dims) - vec2<i32>(1), lo);
  let i = floor(p);
  let f = clamp((p - i - vec2<f32>(0.5)) / w + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
  let a = vec2<i32>(i);
  let t00 = textureLoad(atlas, clamp(a, lo, hi), 0);
  let t10 = textureLoad(atlas, clamp(a + vec2<i32>(1, 0), lo, hi), 0);
  let t01 = textureLoad(atlas, clamp(a + vec2<i32>(0, 1), lo, hi), 0);
  let t11 = textureLoad(atlas, clamp(a + vec2<i32>(1, 1), lo, hi), 0);
  return mix(mix(t00, t10, f.x), mix(t01, t11, f.x), f.y);
}
// A capsule's distance: from p to the segment a–b.
fn seg(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let pa = p - a; let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h);
}
// 0.4.1's faces: eyes, brows and mouth drawn on a head's front, from the
// expression in the part (tex: open, brow tilt, brow raise, smile; misc:
// mouth open, happy). fp is the face's point (unit head, centred), aa a
// pixel's size there. Returns the colour over the skin and its cover.
fn face(skin: vec3<f32>, fp: vec2<f32>, front: f32, e4: vec4<f32>, m2: vec2<f32>, aa: f32) -> vec4<f32> {
  var c = skin;
  let s = select(-1.0, 1.0, fp.x > 0.0);
  let ink = skin * 0.28 + vec3<f32>(0.03, 0.02, 0.03);
  // cheeks, when smiling
  let ch = length(fp - vec2<f32>(s * 0.2, -0.075)) / 0.065;
  c = mix(c, c * vec3<f32>(1.04, 0.86, 0.86), (1.0 - smoothstep(0.4, 1.0, ch)) * clamp(e4.w, 0.0, 1.0) * 0.6);
  let e = vec2<f32>(s * 0.14, 0.05);
  let open = e4.x;
  if (open > 0.08) {
    let q = (fp - e) / vec2<f32>(0.056, 0.064 * clamp(open, 0.2, 1.25));
    let r = length(q);
    let w = max(aa / 0.075, 0.02);
    let white = 1.0 - smoothstep(1.0 - w, 1.0 + w, r);
    c = mix(c, vec3<f32>(0.97, 0.95, 0.93), white);
    let ic = e + vec2<f32>(0.0, -0.004);
    let ri = length(fp - ic) / 0.04;
    let wi = max(aa / 0.04, 0.02);
    let iris = (1.0 - smoothstep(1.0 - wi, 1.0 + wi, ri)) * white;
    c = mix(c, mix(vec3<f32>(0.3, 0.2, 0.13), vec3<f32>(0.16, 0.1, 0.07), smoothstep(0.2, 1.0, ri)), iris);
    c = mix(c, vec3<f32>(0.05, 0.04, 0.06), (1.0 - smoothstep(0.42 - wi, 0.42 + wi, ri)) * white);
    let hl = length(fp - ic - vec2<f32>(0.014, 0.016)) / 0.011;
    c = mix(c, vec3<f32>(1.0), (1.0 - smoothstep(1.0 - wi * 3.0, 1.0 + wi * 3.0, hl)) * white);
    // the upper lid's lashes
    let lid = (1.0 - smoothstep(0.1, 0.1 + w * 2.0, abs(r - 1.0))) * smoothstep(0.05, 0.45, q.y);
    c = mix(c, ink, lid * 0.95);
  } else {
    // shut: an arc (up when happy, down when asleep)
    let dx = (fp.x - e.x) / 0.065;
    let bend = select(-0.022, 0.026, m2.y > 0.5);
    let y = e.y - 0.005 + bend * (1.0 - dx * dx);
    let d = abs(fp.y - y);
    let line = (1.0 - smoothstep(0.009, 0.009 + aa, d)) * step(abs(dx), 1.0);
    c = mix(c, ink, line);
  }
  // brows
  let bc = vec2<f32>(s * 0.15, 0.175 + 0.035 * e4.z);
  let slope = -s * e4.y * 0.45;
  let bd = normalize(vec2<f32>(1.0, slope)) * 0.065;
  let bb = seg(fp, bc - bd, bc + bd);
  c = mix(c, min(ink, vec3<f32>(0.14, 0.09, 0.07)), 1.0 - smoothstep(0.019, 0.019 + aa, bb));
  // the mouth
  let mo = m2.x;
  let my = -0.17;
  if (mo > 0.05) {
    let q = (fp - vec2<f32>(0.0, my - 0.01)) / vec2<f32>(0.05 + 0.02 * max(e4.w, 0.0), 0.012 + 0.055 * mo);
    let r = length(q);
    let w = max(aa / 0.04, 0.03);
    let inside = 1.0 - smoothstep(1.0 - w, 1.0 + w, r);
    let tongue = smoothstep(0.1, 0.6, -q.y);
    c = mix(c, mix(vec3<f32>(0.36, 0.1, 0.13), vec3<f32>(0.78, 0.36, 0.38), tongue * 0.7), inside);
    c = mix(c, ink, (1.0 - smoothstep(0.1, 0.1 + w * 2.0, abs(r - 1.0))) * 0.6);
  } else {
    let x = fp.x / 0.075;
    let y = my + e4.w * 0.045 * (x * x - 0.35);
    let d = abs(fp.y - y);
    let th = 0.011 * (1.0 - 0.5 * x * x);
    let line = (1.0 - smoothstep(th, th + aa, d)) * (1.0 - smoothstep(0.95, 1.05, abs(x)));
    c = mix(c, vec3<f32>(0.42, 0.18, 0.18) * 0.8 + skin * 0.2, line);
  }
  return vec4<f32>(c, front);
}
// 0.4.1's second pass: the part's material and how it's lit.
struct Surf { n: vec3<f32>, alb: f32, rough: f32 };
fn shade(v: VO) -> vec4<f32> {
  var base = v.col;
  let flags = u32(v.misc.w + 0.5);
  // the material's three planes and their footprints (derivatives before
  // any branch, as the texture's mip choice needs)
  let mk = MKIND[min(v.mat, 17u)];
  let q3 = v.lp / max(abs(mk.x), 0.001);
  let pX = q3.zy; let pY = q3.xz; let pZ = q3.xy;
  let gk = g.mat.y;
  let dXx = dpdx(pX) * gk; let dXy = dpdy(pX) * gk;
  let dYx = dpdx(pY) * gk; let dYy = dpdy(pY) * gk;
  let dZx = dpdx(pZ) * gk; let dZy = dpdy(pZ) * gk;
  let fp = vec2<f32>(v.lp.x - 0.5, v.lp.y - 0.5);
  let faa = max(fwidth(fp.x), fwidth(fp.y));
  let uv = v.tex.xy + fract(v.uv * v.misc.xy + v.off) * v.tex.zw;
  // Derivatives before any branch on the item (uniform control flow).
  let tp = uv * vec2<f32>(textureDimensions(atlas)) - vec2<f32>(0.5);
  let tw = max(fwidth(tp), vec2<f32>(0.0001));
  if ((flags & 1u) != 0u) {
    if (g.params.w > 0.5) {
      base = base * sharp(v, tp, tw);
    } else {
      base = base * textureSampleLevel(atlas, samp, uv, 0.0);
    }
  }
  var n = normalize(v.n);
  var sf = Surf(n, 1.0, 0.6);
  if (g.mat.x > 0.5 && v.mat != 0u) {
    let layer = select(i32(v.mat) - 1, 6, v.mat == 17u);
    let w0 = pow(abs(v.ln), vec3<f32>(4.0));
    let w = w0 / max(w0.x + w0.y + w0.z, 1e-4);
    let a = textureSampleGrad(mats, msamp, pX, layer, dXx, dXy);
    let b = textureSampleGrad(mats, msamp, pY, layer, dYx, dYy);
    let c = textureSampleGrad(mats, msamp, pZ, layer, dZx, dZy);
    let tX = a.ba * 2.0 - 1.0; let tY = b.ba * 2.0 - 1.0; let tZ = c.ba * 2.0 - 1.0;
    let nl = normalize(v.ln + w.x * vec3<f32>(0.0, tX.y, tX.x) + w.y * vec3<f32>(tY.x, 0.0, tY.y) + w.z * vec3<f32>(tZ.x, tZ.y, 0.0));
    var nm = nl;
    if (mk.x < 0.0) { nm = nl / v.sc; }
    sf.n = normalize(v.r0 * nm.x + v.r1 * nm.y + v.r2 * nm.z);
    if (dot(sf.n, n) < 0.0) { sf.n = n; }
    sf.alb = (a.r * w.x + b.r * w.y + c.r * w.z) * 1.5;
    sf.rough = a.g * w.x + b.g * w.y + c.g * w.z;
    if (v.mat == 17u) {
      let fc = face(base.rgb, fp, smoothstep(-0.1, -0.3, v.lp.z), v.tex, v.misc.xy, faa);
      base = vec4<f32>(mix(base.rgb, fc.rgb, fc.a), base.a);
    }
    base = vec4<f32>(base.rgb * sf.alb, base.a);
    n = sf.n;
  }
  let lp = g.lvp * vec4<f32>(v.wp + normalize(v.n) * 0.05, 1.0);
  let suv = clamp(vec2<f32>(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5), vec2<f32>(0.0), vec2<f32>(1.0));
  var sh = textureSampleCompareLevel(smap, ssamp, suv, lp.z - 0.002);
  // Outside the map (never, with the box fitted round what's drawn): lit,
  // never the map's edge stretched across as streaks.
  let inside = abs(lp.x) < 1.0 && abs(lp.y) < 1.0 && lp.z > 0.0 && lp.z < 1.0;
  if (g.look.x > 0.5) {
    // 0.4.1: a soft edge in both looks, eight taps on a disk turned per
    // small patch of floor (steady from frame to frame).
    let t = 1.8 / f32(textureDimensions(smap).x);
    let r = hash2(floor(v.wp.xz * 9.0)) * 6.2832;
    let cs = cos(r);
    let sn = sin(r);
    var acc = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
      let o = DISK[i];
      let ro = vec2<f32>(o.x * cs - o.y * sn, o.x * sn + o.y * cs) * t;
      acc = acc + textureSampleCompareLevel(smap, ssamp, suv + ro, lp.z - 0.002);
    }
    sh = acc / 8.0;
  } else if (g.params.w > 0.5) {
    // Full resolution (D-80): a softer shadow edge, four more taps around
    // it (in Pixel art the big pixels hide the shadow map's steps).
    let t = 1.5 / f32(textureDimensions(smap).x);
    sh = sh + textureSampleCompareLevel(smap, ssamp, suv + vec2<f32>(t, 0.0), lp.z - 0.002);
    sh = sh + textureSampleCompareLevel(smap, ssamp, suv - vec2<f32>(t, 0.0), lp.z - 0.002);
    sh = sh + textureSampleCompareLevel(smap, ssamp, suv + vec2<f32>(0.0, t), lp.z - 0.002);
    sh = sh + textureSampleCompareLevel(smap, ssamp, suv - vec2<f32>(0.0, t), lp.z - 0.002);
    sh = sh / 5.0;
  }
  if (!inside) { sh = 1.0; }
  let ndl = max(dot(n, g.sun.xyz), 0.0);
  // the sun in three bands, for a painted look (four, softer, in 0.4.1's)
  var sun = floor(ndl * sh * 3.0 + 0.4) / 3.0 * g.sun.w;
  if (g.look.x > 0.5) { sun = floor(ndl * sh * 4.0 + 0.5) / 4.0 * g.sun.w; }
  if (g.mat.x > 0.5) {
    // the second pass: light wrapping round skin and leaves, the bands
    // softened so the surfaces' relief shows
    let wr = mk.z;
    let d = max((dot(n, g.sun.xyz) + wr) / (1.0 + wr), 0.0) * sh;
    sun = mix(d, floor(d * 4.0 + 0.5) / 4.0, g.mat.z) * g.sun.w;
  }
  let hemi = mix(g.ground.rgb, g.sky.rgb, n.y * 0.5 + 0.5);
  var light = hemi + g.sunc.rgb * sun;
  if (g.look.z > 0.5) {
    // a cool rim, so people and furniture stand off the floor
    let vd = normalize(g.eye.xyz - v.wp);
    let rim = pow(1.0 - max(dot(n, vd), 0.0), 3.0);
    light = light + vec3<f32>(0.55, 0.62, 0.9) * rim * 0.16;
  }
  // dust on what's long unused: a film on upward faces, more with time
  let dust = f32((flags >> 3u) & 3u);
  if (g.look.w > 0.5 && dust > 0.0 && n.y > 0.6) {
    let k = clamp(dust * 0.17 + hash2(floor(v.wp.xz * 14.0)) * 0.1 * dust, 0.0, 0.65);
    base = vec4<f32>(mix(base.rgb, vec3<f32>(0.68, 0.64, 0.58), k), base.a);
  }
  let nl = u32(g.params.z);
  for (var i = 0u; i < nl; i = i + 1u) {
    let L = lights[i];
    let d = L.p.xyz - v.wp;
    let dist = length(d);
    if (dist < L.p.w) {
      let att = (1.0 - dist / L.p.w) * select(1.0, g.look.y, L.c.a < 0.5);
      light = light + L.c.rgb * att * att * (0.4 + 0.6 * max(dot(n, d / max(dist, 0.001)), 0.0)) * (1.0 + g.look.y * 0.9);
    }
  }
  var c = base.rgb * light;
  if (g.mat.x > 0.5) {
    // the second pass's highlights: the sun's (GGX), the sky's
    // reflection, a fill from the other side, cloth's sheen and hair's
    // strands
    let vd = normalize(g.eye.xyz - v.wp);
    let metal = mk.y;
    let rough = clamp(sf.rough, 0.06, 1.0);
    let f0 = mix(vec3<f32>(0.04), base.rgb, metal);
    let hv = normalize(g.sun.xyz + vd);
    let nh = max(dot(n, hv), 0.0);
    let vh = max(dot(vd, hv), 0.0);
    let nv = max(dot(n, vd), 0.001);
    let a2 = rough * rough * rough * rough;
    let dd = nh * nh * (a2 - 1.0) + 1.0;
    let dist = min(a2 / (3.14159 * dd * dd), 40.0);
    let fr = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vh, 5.0);
    let lh = max(dot(g.sun.xyz, hv), 0.1);
    var spec = dist * fr * (0.25 / (lh * lh)) * ndl * sh * g.sun.w * g.sunc.rgb;
    if (MANISO[min(v.mat, 17u)] > 0.5) {
      // strands along the part's length (Kajiya-Kay)
      let tg = normalize(v.r1);
      let th = dot(tg, hv);
      let st = pow(sqrt(max(1.0 - th * th, 0.0)), 60.0) * 0.35 + pow(sqrt(max(1.0 - th * th, 0.0)), 12.0) * 0.12;
      spec = vec3<f32>(st) * mix(vec3<f32>(1.0), base.rgb * 2.0, 0.5) * ndl * sh * g.sun.w;
    }
    let rv = reflect(-vd, n);
    let env = mix(g.ground.rgb, g.sky.rgb * 1.15, smoothstep(-0.3, 0.6, rv.y));
    let fe = f0 + (max(vec3<f32>(1.0 - rough), f0) - f0) * pow(1.0 - nv, 5.0);
    let gloss = (1.0 - rough) * (1.0 - rough);
    let fill_dir = normalize(vec3<f32>(-g.sun.x, 0.55, -g.sun.z));
    let fill = max(dot(n, fill_dir), 0.0) * g.sky.rgb * 0.16;
    let sheen = pow(1.0 - nv, 4.0) * mk.w * 0.35 * (g.sky.rgb + g.sunc.rgb * sun);
    c = base.rgb * (1.0 - metal) * (light + fill) + base.rgb * metal * (light * 0.25) + spec * g.mat.w + env * fe * (0.25 + 0.75 * gloss) * g.mat.w + sheen * base.rgb;
  }
  // a NIGHT part (street lamp, lit window, the sign) glows only at night
  var gl = v.misc.z;
  if ((flags & 32u) != 0u) { gl = gl * g.look.y; }
  var lit_as = base.rgb * 1.1;
  if ((flags & 64u) != 0u) { lit_as = vec3<f32>(1.0, 0.86, 0.58); }
  c = mix(c, lit_as, clamp(gl, 0.0, 1.0));
  return vec4<f32>(c, base.a);
}
@fragment fn fs(v: VO) -> FO { var o: FO; o.c = shade(v); o.id = v.id; return o; }
@fragment fn fs_glass(v: VO) -> FO { var o: FO; let c = shade(v); if (c.a < 0.01) { discard; } o.c = c; o.id = 0u; return o; }
";

/// The post pass.
pub const POST_WGSL: &str = r"
struct P { a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, top: vec4<f32>, bot: vec4<f32>, hov: vec4<u32>, look: vec4<f32> };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var col: texture_2d<f32>;
@group(0) @binding(2) var ids: texture_2d<u32>;
@group(0) @binding(3) var dep: texture_depth_2d;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  let x = f32((i << 1u) & 2u);
  let y = f32(i & 2u);
  return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}
fn lin(d: f32) -> f32 { let n = p.b.z; let f = p.b.w; return n * f / (f - d * (f - n)); }
fn luma(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }
const RING = array<vec2<f32>, 8>(
  vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, -1.0),
  vec2<f32>(0.7, 0.7), vec2<f32>(-0.7, 0.7), vec2<f32>(-0.7, -0.7), vec2<f32>(0.7, -0.7));
// 0.4.1: ambient occlusion from the depth alone. Each sample is compared
// with the plane the pixel sits on (its one-sided slopes, the smaller,
// so an edge isn't read as a slope): nearer than the plane, it occludes.
fn occlusion(q: vec2<i32>, lim: vec2<i32>, d0: f32, rad: f32) -> f32 {
  let ex = lin(textureLoad(dep, clamp(q + vec2<i32>(1, 0), vec2<i32>(0), lim), 0)) - d0;
  let wx = d0 - lin(textureLoad(dep, clamp(q - vec2<i32>(1, 0), vec2<i32>(0), lim), 0));
  let ny = lin(textureLoad(dep, clamp(q + vec2<i32>(0, 1), vec2<i32>(0), lim), 0)) - d0;
  let sy = d0 - lin(textureLoad(dep, clamp(q - vec2<i32>(0, 1), vec2<i32>(0), lim), 0));
  let gx = select(wx, ex, abs(ex) < abs(wx));
  let gy = select(sy, ny, abs(ny) < abs(sy));
  var occ = 0.0;
  for (var k = 0; k < 8; k = k + 1) {
    for (var s = 1; s <= 2; s = s + 1) {
      let o = RING[k] * rad * f32(s) * 0.5 + vec2<f32>(f32(k % 2) * 0.5, 0.0);
      let oi = vec2<i32>(round(o));
      let dr = textureLoad(dep, clamp(q + oi, vec2<i32>(0), lim), 0);
      if (dr >= 1.0) { continue; }
      let diff = d0 + gx * f32(oi.x) + gy * f32(oi.y) - lin(dr);
      occ = occ + smoothstep(0.02, 0.18, diff) * (1.0 - smoothstep(0.5, 1.0, diff));
    }
  }
  return occ / 16.0;
}
// What a scene pixel shows: the scene, or the sky where nothing was drawn.
fn seen(q: vec2<i32>, skyc: vec3<f32>) -> vec3<f32> {
  if (textureLoad(dep, q, 0) >= 1.0) { return skyc; }
  return textureLoad(col, q, 0).rgb;
}
@fragment fn fs(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let size = vec2<i32>(i32(p.a.x), i32(p.a.y));
  let q = clamp(vec2<i32>(floor(fc.xy / p.a.z)), vec2<i32>(0), size - vec2<i32>(1));
  let draw = textureLoad(dep, q, 0);
  let id0 = textureLoad(ids, q, 0).r;
  let skyc = mix(p.top.rgb, p.bot.rgb, smoothstep(0.0, 1.0, fc.y / p.c.y));
  var c = seen(q, skyc);
  let hov = p.hov.x;
  let lim = size - vec2<i32>(1);
  // Full resolution (D-80): edges smoothed where the picture's contrast
  // jumps (an FXAA-like blend over the four neighbours, the sky included).
  if (p.c.z > 0.5) {
    let n = seen(clamp(q + vec2<i32>(0, -1), vec2<i32>(0), lim), skyc);
    let s = seen(clamp(q + vec2<i32>(0, 1), vec2<i32>(0), lim), skyc);
    let e = seen(clamp(q + vec2<i32>(1, 0), vec2<i32>(0), lim), skyc);
    let w = seen(clamp(q + vec2<i32>(-1, 0), vec2<i32>(0), lim), skyc);
    let lc = luma(c);
    let hi = max(lc, max(max(luma(n), luma(s)), max(luma(e), luma(w))));
    let lo = min(lc, min(min(luma(n), luma(s)), min(luma(e), luma(w))));
    let range = hi - lo;
    if (range > max(0.04, hi * 0.12)) {
      let k = clamp(range * 2.0, 0.0, 0.75);
      c = mix(c, (n + s + e + w + c * 2.0) / 6.0, k);
    }
  }
  if (draw < 1.0) {
    let d0 = lin(draw);
    var edge = false;
    var hl = false;
    // The outline's width, in scene pixels: one in Pixel art, about half
    // its unit in Full resolution, so it reads at the same weight.
    let ow = max(i32(p.a.w), 1);
    let hw = max(i32(p.c.w), 1);
    let offs = array<vec2<i32>, 4>(vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1));
    for (var k = 0; k < 4; k = k + 1) {
      let qq = clamp(q + offs[k] * ow, vec2<i32>(0), lim);
      let idn = textureLoad(ids, qq, 0).r;
      let dn = textureLoad(dep, qq, 0);
      if (idn != id0 && dn < 1.0 && lin(dn) < d0 * 0.97) { edge = true; }
      let qh = clamp(q + offs[k] * hw, vec2<i32>(0), lim);
      if (hov != 0u && id0 == hov && textureLoad(ids, qh, 0).r != hov) { hl = true; }
    }
    if (p.look.x > 0.5) {
      let rad = clamp(0.28 * p.look.w / d0, 2.0, 16.0);
      c = c * (1.0 - 0.75 * occlusion(q, lim, d0, rad));
    }
    if (edge) { c = c * 0.45 + vec3<f32>(0.04, 0.03, 0.07); }
    if (hl) { c = vec3<f32>(1.0, 0.93, 0.6); }
  }
  if (p.look.y > 0.5 && draw < 1.0) {
    // 0.4.1's grade: cool shadows, warm lights, a gentle S-curve, a little
    // more colour (the night keeps it cooler)
    let l = luma(c);
    let warm = mix(vec3<f32>(0.03, 0.015, -0.02), vec3<f32>(-0.01, 0.0, 0.03), p.look.z);
    c = c + mix(vec3<f32>(-0.012, 0.0, 0.03), warm, smoothstep(0.2, 0.8, l));
    let k = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    c = mix(c, k * k * (3.0 - 2.0 * k), 0.22);
    c = max(mix(vec3<f32>(luma(c)), c, 1.1), vec3<f32>(0.0));
  }
  if (p.b.y > 0.5) { c = mix(c, vec3<f32>(0.95, 0.12, 0.1), 0.1 + 0.07 * sin(p.b.x * 3.1416)); }
  let uv = fc.xy / p.c.xy;
  let v = smoothstep(0.95, 0.4, length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.8)));
  c = c * mix(0.72, 1.0, v);
  return vec4<f32>(c, 1.0);
}
";

/// The overlay pass.
pub const OVERLAY_WGSL: &str = r"
struct U { s: vec2<f32>, o: vec2<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) tint: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32, @location(0) dst: vec4<f32>, @location(1) src: vec4<f32>, @location(2) tint: vec4<f32>) -> VOut {
  var c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let k = c[vi];
  let p = dst.xy + k * dst.zw;
  var o: VOut;
  o.pos = vec4(p.x * u.s.x + u.o.x, p.y * u.s.y + u.o.y, 0.0, 1.0);
  o.uv = src.xy + k * src.zw;
  o.tint = tint;
  return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, v.uv) * v.tint;
  if (c.a < 0.02) { discard; }
  return c;
}
";
