//! The little linear algebra the Floor needs: 3-vectors and column-major
//! 4×4 matrices (`m[column][row]`), right-handed, depth 0–1 as wgpu wants.

/// A 3-vector.
pub type V3 = [f32; 3];

/// `a + b`.
#[must_use]
pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a - b`.
#[must_use]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `a × k`.
#[must_use]
pub fn scale(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// The dot product.
#[must_use]
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product.
#[must_use]
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The length.
#[must_use]
pub fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}

/// `a` at length 1 (a zero vector stays tiny, never NaN).
#[must_use]
pub fn norm(a: V3) -> V3 {
    scale(a, 1.0 / len(a).max(1e-9))
}

/// A column-major 4×4 matrix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct M4(pub [[f32; 4]; 4]);

impl M4 {
    /// The identity.
    pub const I: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// `self × o`.
    #[must_use]
    pub fn mul(&self, o: &Self) -> Self {
        let mut r = [[0.0f32; 4]; 4];
        for (c, column) in r.iter_mut().enumerate() {
            for (row, cell) in column.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.0[k][row] * o.0[c][k]).sum();
            }
        }
        Self(r)
    }

    /// A translation.
    #[must_use]
    pub fn t(x: f32, y: f32, z: f32) -> Self {
        let mut m = Self::I;
        m.0[3] = [x, y, z, 1.0];
        m
    }

    /// A scale.
    #[must_use]
    pub fn s(x: f32, y: f32, z: f32) -> Self {
        Self([
            [x, 0.0, 0.0, 0.0],
            [0.0, y, 0.0, 0.0],
            [0.0, 0.0, z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// A turn about y.
    #[must_use]
    pub fn ry(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            [c, 0.0, -s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// A turn about x.
    #[must_use]
    pub fn rx(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, s, 0.0],
            [0.0, -s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// A turn about z.
    #[must_use]
    pub fn rz(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self([
            [c, s, 0.0, 0.0],
            [-s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Where it takes the point `p`.
    #[must_use]
    pub fn point(&self, p: V3) -> V3 {
        let m = &self.0;
        [
            m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0],
            m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1],
            m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2],
        ]
    }

    /// `p` in clip space (before the divide).
    #[must_use]
    pub fn project(&self, p: V3) -> [f32; 4] {
        let m = &self.0;
        let mut r = [0.0; 4];
        for (row, v) in r.iter_mut().enumerate() {
            *v = m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row];
        }
        r
    }

    /// A camera at `eye` looking at `at`.
    #[must_use]
    pub fn look_at(eye: V3, at: V3, up: V3) -> Self {
        let f = norm(sub(at, eye));
        let s = norm(cross(f, up));
        let u = cross(s, f);
        Self([
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
        ])
    }

    /// A right-handed perspective, depth 0–1.
    #[must_use]
    pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fovy / 2.0).tan();
        Self([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    /// An orthographic box, depth 0–1 (the sun's shadow).
    #[must_use]
    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        Self([
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, 1.0 / (near - far), 0.0],
            [
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                near / (near - far),
                1.0,
            ],
        ])
    }
}

/// A transform built as a chain, `T::new().t(..).ry(..).s(..)`, applied
/// right to left like a scene graph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct T(pub M4);

impl Default for T {
    fn default() -> Self {
        Self(M4::I)
    }
}

impl T {
    /// The identity.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Then a translation.
    #[must_use]
    pub fn t(self, x: f32, y: f32, z: f32) -> Self {
        Self(self.0.mul(&M4::t(x, y, z)))
    }
    /// Then a scale.
    #[must_use]
    pub fn s(self, x: f32, y: f32, z: f32) -> Self {
        Self(self.0.mul(&M4::s(x, y, z)))
    }
    /// Then a turn about y.
    #[must_use]
    pub fn ry(self, a: f32) -> Self {
        Self(self.0.mul(&M4::ry(a)))
    }
    /// Then a turn about x.
    #[must_use]
    pub fn rx(self, a: f32) -> Self {
        Self(self.0.mul(&M4::rx(a)))
    }
    /// Then a turn about z.
    #[must_use]
    pub fn rz(self, a: f32) -> Self {
        Self(self.0.mul(&M4::rz(a)))
    }
    /// Then `o`.
    #[must_use]
    pub fn then(self, o: Self) -> Self {
        Self(self.0.mul(&o.0))
    }
}

#[cfg(test)]
mod tests {
    use super::{M4, T, cross, len, norm};

    #[test]
    fn transforms_compose_right_to_left() {
        let t = T::new().t(1.0, 0.0, 0.0).ry(std::f32::consts::FRAC_PI_2);
        let p = t.0.point([1.0, 0.0, 0.0]);
        assert!(
            (p[0] - 1.0).abs() < 1e-5 && (p[2] + 1.0).abs() < 1e-5,
            "{p:?}"
        );
        assert_eq!(M4::I.mul(&M4::t(1.0, 2.0, 3.0)), M4::t(1.0, 2.0, 3.0));
        assert!((len(norm([3.0, 4.0, 0.0])) - 1.0).abs() < 1e-6);
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_point_in_front_of_the_camera_lands_inside_the_depth_range() {
        let view = M4::look_at([0.0, 5.0, 10.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let proj = M4::perspective(0.8, 1.5, 0.5, 200.0);
        let c = proj.mul(&view).project([0.0, 0.0, 0.0]);
        let depth = c[2] / c[3];
        assert!((0.0..=1.0).contains(&depth), "{depth}");
    }
}
