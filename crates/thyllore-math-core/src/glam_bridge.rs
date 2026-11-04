use cgmath::{Matrix3, Matrix4, Point3, Quaternion, Vector2, Vector3, Vector4};

pub trait ToGlam {
    type Output;
    fn to_glam(&self) -> Self::Output;
}

pub trait ToCgmath {
    type Output;
    fn to_cgmath(&self) -> Self::Output;
}

impl ToGlam for Vector2<f32> {
    type Output = glam::Vec2;
    fn to_glam(&self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }
}

impl ToCgmath for glam::Vec2 {
    type Output = Vector2<f32>;
    fn to_cgmath(&self) -> Vector2<f32> {
        Vector2::new(self.x, self.y)
    }
}

impl ToGlam for Vector3<f32> {
    type Output = glam::Vec3;
    fn to_glam(&self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }
}

impl ToCgmath for glam::Vec3 {
    type Output = Vector3<f32>;
    fn to_cgmath(&self) -> Vector3<f32> {
        Vector3::new(self.x, self.y, self.z)
    }
}

impl ToGlam for Vector4<f32> {
    type Output = glam::Vec4;
    fn to_glam(&self) -> glam::Vec4 {
        glam::Vec4::new(self.x, self.y, self.z, self.w)
    }
}

impl ToCgmath for glam::Vec4 {
    type Output = Vector4<f32>;
    fn to_cgmath(&self) -> Vector4<f32> {
        Vector4::new(self.x, self.y, self.z, self.w)
    }
}

impl ToGlam for Point3<f32> {
    type Output = glam::Vec3;
    fn to_glam(&self) -> glam::Vec3 {
        glam::Vec3::new(self.x, self.y, self.z)
    }
}

impl ToGlam for Matrix3<f32> {
    type Output = glam::Mat3;
    fn to_glam(&self) -> glam::Mat3 {
        glam::Mat3::from_cols(self.x.to_glam(), self.y.to_glam(), self.z.to_glam())
    }
}

impl ToCgmath for glam::Mat3 {
    type Output = Matrix3<f32>;
    fn to_cgmath(&self) -> Matrix3<f32> {
        Matrix3::from_cols(
            self.x_axis.to_cgmath(),
            self.y_axis.to_cgmath(),
            self.z_axis.to_cgmath(),
        )
    }
}

impl ToGlam for Matrix4<f32> {
    type Output = glam::Mat4;
    fn to_glam(&self) -> glam::Mat4 {
        glam::Mat4::from_cols(
            self.x.to_glam(),
            self.y.to_glam(),
            self.z.to_glam(),
            self.w.to_glam(),
        )
    }
}

impl ToCgmath for glam::Mat4 {
    type Output = Matrix4<f32>;
    fn to_cgmath(&self) -> Matrix4<f32> {
        Matrix4::from_cols(
            self.x_axis.to_cgmath(),
            self.y_axis.to_cgmath(),
            self.z_axis.to_cgmath(),
            self.w_axis.to_cgmath(),
        )
    }
}

impl ToGlam for Quaternion<f32> {
    type Output = glam::Quat;
    fn to_glam(&self) -> glam::Quat {
        glam::Quat::from_xyzw(self.v.x, self.v.y, self.v.z, self.s)
    }
}

impl ToCgmath for glam::Quat {
    type Output = Quaternion<f32>;
    fn to_cgmath(&self) -> Quaternion<f32> {
        Quaternion::new(self.w, self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{InnerSpace, Rad, Rotation3};

    #[test]
    fn vector2_round_trip() {
        let v = Vector2::new(1.0, 2.0);
        let g: glam::Vec2 = v.to_glam();
        let rt: Vector2<f32> = g.to_cgmath();
        assert!((v - rt).dot(v - rt) < 1e-12);
    }

    #[test]
    fn vector3_round_trip() {
        let v = Vector3::new(1.0, 2.0, 3.0);
        let g: glam::Vec3 = v.to_glam();
        let rt: Vector3<f32> = g.to_cgmath();
        assert!((v - rt).dot(v - rt) < 1e-12);
    }

    #[test]
    fn vector4_round_trip() {
        let v = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let g: glam::Vec4 = v.to_glam();
        let rt: Vector4<f32> = g.to_cgmath();
        assert!((v - rt).dot(v - rt) < 1e-12);
    }

    #[test]
    fn quaternion_keeps_the_real_part_in_w() {
        let q_cg = Quaternion::from_axis_angle(Vector3::unit_y(), Rad(std::f32::consts::FRAC_PI_2));
        let q_gl: glam::Quat = q_cg.to_glam();

        let v = Vector3::new(1.0, 0.0, 0.0);
        let rotated_cg = q_cg * v;

        let v_gl = glam::Vec3::new(1.0, 0.0, 0.0);
        let rotated_gl = q_gl * v_gl;

        let diff = (rotated_cg.x - rotated_gl.x).abs()
            + (rotated_cg.y - rotated_gl.y).abs()
            + (rotated_cg.z - rotated_gl.z).abs();
        assert!(
            diff < 1e-6,
            "cgmath {:?} vs glam {:?}",
            rotated_cg,
            rotated_gl
        );
    }

    #[test]
    fn matrix4_round_trips_column_major() {
        let m = Matrix4::from_cols(
            Vector4::new(1.0, 0.1, 0.2, 0.3),
            Vector4::new(0.4, 1.0, 0.5, 0.6),
            Vector4::new(0.7, 0.8, 1.0, 0.9),
            Vector4::new(1.0, 2.0, 3.0, 1.0),
        );

        let g: glam::Mat4 = m.to_glam();
        let rt: Matrix4<f32> = g.to_cgmath();

        assert_eq!(m.x, rt.x);
        assert_eq!(m.y, rt.y);
        assert_eq!(m.z, rt.z);
        assert_eq!(m.w, rt.w);

        let p = Vector4::new(1.0, 2.0, 3.0, 1.0);
        let transformed_cg = m * p;
        let transformed_gl = g * glam::Vec4::new(1.0, 2.0, 3.0, 1.0);

        assert!((transformed_cg.x - transformed_gl.x).abs() < 1e-6);
        assert!((transformed_cg.y - transformed_gl.y).abs() < 1e-6);
        assert!((transformed_cg.z - transformed_gl.z).abs() < 1e-6);
    }
}
