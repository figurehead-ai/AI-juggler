//! The Floor's shaders, in WGSL: the scene (with the sun's shadow, a pick
//! id per pixel, banded sunlight for a painted look, point lights, glow
//! and the cutaway of walls between the camera and the room), the post
//! pass (hard-edged upscale, outlines, the hovered thing outlined, sky,
//! the red-alert wash, a soft vignette) and the overlay.

/// The scene and shadow passes.
pub const SCENE_WGSL: &str = r"
struct G { vp: mat4x4<f32>, lvp: mat4x4<f32>, eye: vec4<f32>, sun: vec4<f32>, sky: vec4<f32>, ground: vec4<f32>, params: vec4<f32> };
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
fn shade(v: VO) -> vec4<f32> {
  var base = v.col;
  let flags = u32(v.misc.w + 0.5);
  if ((flags & 1u) != 0u) {
    let uv = v.tex.xy + fract(v.uv * v.misc.xy + v.off) * v.tex.zw;
    base = base * textureSampleLevel(atlas, samp, uv, 0.0);
  }
  let n = normalize(v.n);
  let lp = g.lvp * vec4<f32>(v.wp + n * 0.05, 1.0);
  let suv = clamp(vec2<f32>(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5), vec2<f32>(0.0), vec2<f32>(1.0));
  let sh = textureSampleCompareLevel(smap, ssamp, suv, lp.z - 0.002);
  let ndl = max(dot(n, g.sun.xyz), 0.0);
  // the sun in three bands, for a painted look
  let sun = floor(ndl * sh * 3.0 + 0.4) / 3.0 * g.sun.w;
  let hemi = mix(g.ground.rgb, g.sky.rgb, n.y * 0.5 + 0.5);
  var light = hemi + vec3<f32>(1.0, 0.9, 0.76) * sun;
  let nl = u32(g.params.z);
  for (var i = 0u; i < nl; i = i + 1u) {
    let L = lights[i];
    let d = L.p.xyz - v.wp;
    let dist = length(d);
    if (dist < L.p.w) {
      let att = 1.0 - dist / L.p.w;
      light = light + L.c.rgb * att * att * (0.4 + 0.6 * max(dot(n, d / max(dist, 0.001)), 0.0));
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
struct P { a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, top: vec4<f32>, bot: vec4<f32>, hov: vec4<u32> };
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
@fragment fn fs(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let size = vec2<i32>(i32(p.a.x), i32(p.a.y));
  let q = clamp(vec2<i32>(floor(fc.xy / p.a.z)), vec2<i32>(0), size - vec2<i32>(1));
  let draw = textureLoad(dep, q, 0);
  let id0 = textureLoad(ids, q, 0).r;
  var c = textureLoad(col, q, 0).rgb;
  let hov = p.hov.x;
  if (draw >= 1.0) {
    let y = fc.y / p.c.y;
    c = mix(p.top.rgb, p.bot.rgb, smoothstep(0.0, 1.0, y));
  } else {
    let d0 = lin(draw);
    var edge = false;
    var hl = false;
    let offs = array<vec2<i32>, 4>(vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1));
    for (var k = 0; k < 4; k = k + 1) {
      let qq = clamp(q + offs[k], vec2<i32>(0), size - vec2<i32>(1));
      let idn = textureLoad(ids, qq, 0).r;
      let dn = textureLoad(dep, qq, 0);
      if (idn != id0 && dn < 1.0 && lin(dn) < d0 * 0.97) { edge = true; }
      if (hov != 0u && id0 == hov && idn != hov) { hl = true; }
    }
    if (edge) { c = c * 0.45 + vec3<f32>(0.04, 0.03, 0.07); }
    if (hl) { c = vec3<f32>(1.0, 0.93, 0.6); }
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
