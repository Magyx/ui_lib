use std::ops::Mul;

pub type Vec3 = [f32; 3];

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn normalize(v: Vec3) -> Vec3 {
    let len = dot(v, v).sqrt();
    if len <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

/// Column-major 4x4: `m[column][row]`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Mat4(pub [[f32; 4]; 4]);
impl std::ops::Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Self) -> Self {
        let a = self.0;
        let b = rhs.0;
        let mut out = [[0.0f32; 4]; 4];
        for (c, col) in out.iter_mut().enumerate() {
            for (r, cell) in col.iter_mut().enumerate() {
                *cell =
                    a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
            }
        }
        Self(out)
    }
}
impl Mat4 {
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.0[3] = [t[0], t[1], t[2], 1.0];
        m
    }

    pub fn scale(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.0[0][0] = s[0];
        m.0[1][1] = s[1];
        m.0[2][2] = s[2];
        m
    }

    pub fn uniform_scale(s: f32) -> Self {
        Self::scale([s, s, s])
    }

    pub fn rotation_x(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[1] = [0.0, c, s, 0.0];
        m.0[2] = [0.0, -s, c, 0.0];
        m
    }
    pub fn rotation_y(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = [c, 0.0, -s, 0.0];
        m.0[2] = [s, 0.0, c, 0.0];
        m
    }
    pub fn rotation_z(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = [c, s, 0.0, 0.0];
        m.0[1] = [-s, c, 0.0, 0.0];
        m
    }

    /// Y then X then Z, the usual "turntable plus tilt" ordering.
    pub fn rotation_euler(r: Vec3) -> Self {
        Self::rotation_y(r[1])
            .mul(Self::rotation_x(r[0]))
            .mul(Self::rotation_z(r[2]))
    }

    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = normalize(sub(target, eye));
        let s = normalize(cross(f, up));
        let u = cross(s, f);
        Self([
            [s[0], u[0], -f[0], 0.0],
            [s[1], u[1], -f[1], 0.0],
            [s[2], u[2], -f[2], 0.0],
            [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
        ])
    }

    /// Right-handed perspective with `z` mapped to `[0, 1]`.
    pub fn perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y * 0.5).tan();
        let aspect = if aspect.abs() < f32::EPSILON {
            1.0
        } else {
            aspect
        };
        Self([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, (far * near) / (near - far), 0.0],
        ])
    }

    /// Inverse-transpose of the upper-left 3x3, padded to three `vec4`s for
    /// the vertex attribute. Correct for non-uniform scale, unlike using the
    /// model matrix directly.
    pub fn normal_matrix(self) -> [[f32; 4]; 3] {
        let m = self.0;
        let a = [
            [m[0][0], m[0][1], m[0][2]],
            [m[1][0], m[1][1], m[1][2]],
            [m[2][0], m[2][1], m[2][2]],
        ];

        let det = a[0][0] * (a[1][1] * a[2][2] - a[2][1] * a[1][2])
            - a[1][0] * (a[0][1] * a[2][2] - a[2][1] * a[0][2])
            + a[2][0] * (a[0][1] * a[1][2] - a[1][1] * a[0][2]);

        if det.abs() < 1e-8 {
            // Degenerate model matrix: fall back to the rotation part as-is
            // rather than emitting NaNs into the vertex stream.
            return [
                [a[0][0], a[0][1], a[0][2], 0.0],
                [a[1][0], a[1][1], a[1][2], 0.0],
                [a[2][0], a[2][1], a[2][2], 0.0],
            ];
        }

        let inv_det = 1.0 / det;
        // Inverse, then transpose — written out directly as the transpose of
        // the adjugate divided by the determinant.
        let n = [
            [
                (a[1][1] * a[2][2] - a[2][1] * a[1][2]) * inv_det,
                (a[2][0] * a[1][2] - a[1][0] * a[2][2]) * inv_det,
                (a[1][0] * a[2][1] - a[2][0] * a[1][1]) * inv_det,
            ],
            [
                (a[2][1] * a[0][2] - a[0][1] * a[2][2]) * inv_det,
                (a[0][0] * a[2][2] - a[2][0] * a[0][2]) * inv_det,
                (a[2][0] * a[0][1] - a[0][0] * a[2][1]) * inv_det,
            ],
            [
                (a[0][1] * a[1][2] - a[1][1] * a[0][2]) * inv_det,
                (a[1][0] * a[0][2] - a[0][0] * a[1][2]) * inv_det,
                (a[0][0] * a[1][1] - a[1][0] * a[0][1]) * inv_det,
            ],
        ];

        [
            [n[0][0], n[0][1], n[0][2], 0.0],
            [n[1][0], n[1][1], n[1][2], 0.0],
            [n[2][0], n[2][1], n[2][2], 0.0],
        ]
    }
}

/// A camera, resolved against a widget's aspect ratio at paint time.
#[derive(Copy, Clone, Debug)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: [0.0, 0.8, 3.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            fov_y: std::f32::consts::FRAC_PI_4,
            near: 0.05,
            far: 200.0,
        }
    }
}

impl Camera {
    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }
    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far)
    }
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect).mul(self.view())
    }
    pub fn orbit(mut self, yaw: f32, pitch: f32, distance: f32) -> Self {
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        self.eye = [
            self.target[0] + distance * cp * sy,
            self.target[1] + distance * sp,
            self.target[2] + distance * cp * cy,
        ];
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_a_multiplicative_identity() {
        let m = Mat4::rotation_y(0.7).mul(Mat4::translation([1.0, 2.0, 3.0]));
        assert_eq!(m.mul(Mat4::IDENTITY), m);
        assert_eq!(Mat4::IDENTITY.mul(m), m);
    }

    #[test]
    fn perspective_maps_near_and_far_to_zero_and_one() {
        let p = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 1.0, 100.0);
        let project_z = |z: f32| {
            let clip_z = p.0[2][2] * z + p.0[3][2];
            let w = -z;
            clip_z / w
        };
        assert!((project_z(-1.0) - 0.0).abs() < 1e-4, "near plane -> 0");
        assert!((project_z(-100.0) - 1.0).abs() < 1e-4, "far plane -> 1");
    }

    #[test]
    fn normal_matrix_survives_non_uniform_scale() {
        // A plane scaled 2x in x: its normal (1,0,0) must stay (1,0,0)
        // in direction, which the inverse-transpose gives and the raw model
        // matrix would not.
        let m = Mat4::scale([2.0, 1.0, 1.0]);
        let n = m.normal_matrix();
        assert!((n[0][0] - 0.5).abs() < 1e-6);
        assert!((n[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn look_at_places_the_eye_at_the_origin() {
        let v = Mat4::look_at_rh([0.0, 0.0, 5.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        // eye transformed by the view matrix is the origin
        let e = [0.0f32, 0.0, 5.0];
        let x = v.0[0][0] * e[0] + v.0[1][0] * e[1] + v.0[2][0] * e[2] + v.0[3][0];
        let y = v.0[0][1] * e[0] + v.0[1][1] * e[1] + v.0[2][1] * e[2] + v.0[3][1];
        let z = v.0[0][2] * e[0] + v.0[1][2] * e[1] + v.0[2][2] * e[2] + v.0[3][2];
        assert!(x.abs() < 1e-5 && y.abs() < 1e-5 && z.abs() < 1e-5);
    }
}
