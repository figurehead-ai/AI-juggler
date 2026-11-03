//! The little linear algebra the Floor needs: column-major 4×4 matrices and 3-vectors.

pub type V3 = [f32; 3];

pub fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
pub fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
pub fn scale(a: V3, k: f32) -> V3 { [a[0] * k, a[1] * k, a[2] * k] }
pub fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
pub fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
pub fn len(a: V3) -> f32 { dot(a, a).sqrt() }
pub fn norm(a: V3) -> V3 { let l = len(a).max(1e-9); scale(a, 1.0 / l) }

/// Column-major: `m[c][r]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M4(pub [[f32; 4]; 4]);

impl M4 {
    pub const I: M4 = M4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
    pub fn mul(&self, o: &M4) -> M4 {
        let mut r = [[0.0f32; 4]; 4];
        for c in 0..4 { for row in 0..4 { r[c][row] = (0..4).map(|k| self.0[k][row] * o.0[c][k]).sum(); } }
        M4(r)
    }
    pub fn t(x: f32, y: f32, z: f32) -> M4 { let mut m = M4::I; m.0[3] = [x, y, z, 1.0]; m }
    pub fn s(x: f32, y: f32, z: f32) -> M4 { M4([[x, 0.0, 0.0, 0.0], [0.0, y, 0.0, 0.0], [0.0, 0.0, z, 0.0], [0.0, 0.0, 0.0, 1.0]]) }
    pub fn ry(a: f32) -> M4 { let (s, c) = a.sin_cos(); M4([[c, 0.0, -s, 0.0], [0.0, 1.0, 0.0, 0.0], [s, 0.0, c, 0.0], [0.0, 0.0, 0.0, 1.0]]) }
    pub fn rx(a: f32) -> M4 { let (s, c) = a.sin_cos(); M4([[1.0, 0.0, 0.0, 0.0], [0.0, c, s, 0.0], [0.0, -s, c, 0.0], [0.0, 0.0, 0.0, 1.0]]) }
    pub fn rz(a: f32) -> M4 { let (s, c) = a.sin_cos(); M4([[c, s, 0.0, 0.0], [-s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]) }
    pub fn point(&self, p: V3) -> V3 {
        let m = &self.0;
        [m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0], m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1], m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2]]
    }
    pub fn project(&self, p: V3) -> [f32; 4] {
        let m = &self.0;
        let mut r = [0.0; 4];
        for (row, v) in r.iter_mut().enumerate() { *v = m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row]; }
        r
    }
    pub fn look_at(eye: V3, at: V3, up: V3) -> M4 {
        let f = norm(sub(at, eye));
        let s = norm(cross(f, up));
        let u = cross(s, f);
        M4([[s[0], u[0], -f[0], 0.0], [s[1], u[1], -f[1], 0.0], [s[2], u[2], -f[2], 0.0], [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0]])
    }
    /// Right-handed perspective with depth 0..1 (wgpu).
    pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> M4 {
        let f = 1.0 / (fovy / 2.0).tan();
        M4([[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far / (near - far), -1.0], [0.0, 0.0, near * far / (near - far), 0.0]])
    }
    pub fn ortho(l: f32, r: f32, b: f32, t: f32, near: f32, far: f32) -> M4 {
        M4([[2.0 / (r - l), 0.0, 0.0, 0.0], [0.0, 2.0 / (t - b), 0.0, 0.0], [0.0, 0.0, 1.0 / (near - far), 0.0], [-(r + l) / (r - l), -(t + b) / (t - b), near / (near - far), 1.0]])
    }
}

/// Builds a transform from a chain: `T::new().t(..).ry(..).s(..)` applies right to left like a scene graph.
#[derive(Clone, Copy)]
pub struct T(pub M4);
impl T {
    pub fn new() -> T { T(M4::I) }
    pub fn t(self, x: f32, y: f32, z: f32) -> T { T(self.0.mul(&M4::t(x, y, z))) }
    pub fn s(self, x: f32, y: f32, z: f32) -> T { T(self.0.mul(&M4::s(x, y, z))) }
    pub fn ry(self, a: f32) -> T { T(self.0.mul(&M4::ry(a))) }
    pub fn rx(self, a: f32) -> T { T(self.0.mul(&M4::rx(a))) }
    pub fn rz(self, a: f32) -> T { T(self.0.mul(&M4::rz(a))) }
    pub fn then(self, o: T) -> T { T(self.0.mul(&o.0)) }
}
