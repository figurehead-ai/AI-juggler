//! The Floor's shaders, in WGSL: the scene (with the sun's shadow, a pick
//! id per pixel, banded sunlight for a painted look, point lights, glow
//! and the cutaway of walls between the camera and the room), the post
//! pass (hard-edged upscale, outlines, the hovered thing outlined, sky,
//! the red-alert wash, a soft vignette) and the overlay. In Full
//! resolution (D-80) the scene is at device pixels: textures sharp-
//! bilinear, edges smoothed, outlines as wide as Pixel art's look.

/// The scene and shadow passes.
pub const SCENE_WGSL: &str = r"
struct G { vp: mat4x4<f32>, lvp: mat4x4<f32>, eye: vec4<f32>, sun: vec4<f32>, sky: vec4<f32>, ground: vec4<f32>, params: vec4<f32>, sunc: vec4<f32>, look: vec4<f32> };
struct Lt { p: vec4<f32>, c: vec4<f32> };
@group(0) @binding(0) var<uniform> g: G;
@group(0) @binding(1) var<storage, read> lights: array<Lt>;
@group(0) @binding(2) var atlas: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var smap: texture_depth_2d;
@group(0) @binding(5) var ssamp: sampler_comparison;
struct VI {
  @location(0) pos: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
  @location(7) col: vec4<f32>, @location(8) tex: vec4<f32>, @location(9) misc: vec4<f32>, @location(10) off: vec2<f32>, @location(11) id: u32,
};
struct VO {
  @builtin(position) pos: vec4<f32>,
  @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) col: vec4<f32>, @location(4) tex: vec4<f32>, @location(5) misc: vec4<f32>, @location(6) off: vec2<f32>,
  @location(7) @interpolate(flat) id: u32,
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
fn shade(v: VO) -> vec4<f32> {
  var base = v.col;
  let flags = u32(v.misc.w + 0.5);
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
  let n = normalize(v.n);
  let lp = g.lvp * vec4<f32>(v.wp + n * 0.05, 1.0);
  let suv = clamp(vec2<f32>(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5), vec2<f32>(0.0), vec2<f32>(1.0));
  var sh = textureSampleCompareLevel(smap, ssamp, suv, lp.z - 0.002);
  if (g.look.x > 0.5) {
    // 0.4.2: a soft edge in both looks, eight taps on a disk turned per
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
  let ndl = max(dot(n, g.sun.xyz), 0.0);
  // the sun in three bands, for a painted look (four, softer, in 0.4.2's)
  var sun = floor(ndl * sh * 3.0 + 0.4) / 3.0 * g.sun.w;
  if (g.look.x > 0.5) { sun = floor(ndl * sh * 4.0 + 0.5) / 4.0 * g.sun.w; }
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
      let att = 1.0 - dist / L.p.w;
      light = light + L.c.rgb * att * att * (0.4 + 0.6 * max(dot(n, d / max(dist, 0.001)), 0.0)) * (1.0 + g.look.y * 0.9);
    }
  }
  var c = base.rgb * light;
  c = mix(c, base.rgb * 1.1, clamp(v.misc.z, 0.0, 1.0));
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
// 0.4.2: ambient occlusion from the depth alone. Each sample is compared
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
    // 0.4.2's grade: cool shadows, warm lights, a gentle S-curve, a little
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
