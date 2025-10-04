pub use glam::{Mat3, Mat4};
pub use glam::{Quat, Vec2, Vec3, Vec4};

pub fn rad(angle: f32) -> f32 {
    angle / 180.0 * std::f32::consts::PI
}

pub fn vec3_from_array(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

pub fn array3_from_vec(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

pub fn vec2_from_array(a: [f32; 2]) -> Vec2 {
    Vec2::new(a[0], a[1])
}

pub fn array2_from_vec(v: Vec2) -> [f32; 2] {
    [v.x, v.y]
}

pub unsafe fn mat3_all_elements(
    m00: f32,
    m01: f32,
    m02: f32,
    m10: f32,
    m11: f32,
    m12: f32,
    m20: f32,
    m21: f32,
    m22: f32,
) -> Mat3 {
    let m0 = Vec3::new(m00, m01, m02);
    let m1 = Vec3::new(m10, m11, m12);
    let m2 = Vec3::new(m20, m21, m22);
    Mat3::from_cols(m0, m1, m2)
}

pub unsafe fn rodrigues(rotate: &mut Mat3, c: f32, s: f32, n: &Vec3) -> anyhow::Result<()> {
    let ac = 1.0f32 - c;
    let xyac = n.x * n.y * ac;
    let yzac = n.y * n.z * ac;
    let zxac = n.x * n.z * ac;
    let xs = n.x * s;
    let ys = n.y * s;
    let zs = n.z * s;
    // rotate = glm::mat3(c + n.x * n.x * ac, n.x * n.y * ac + n.z * s, n.z * n.x * ac - n.y * s,
    //     n.x * n.y * ac - n.z * s, c + n.y * n.y * ac, n.y * n.z * ac + n.x * s,
    //     n.z * n.x * ac + n.y * s, n.y * n.z * ac - n.x * s, c + n.z * n.z * ac);
    *rotate = mat3_all_elements(
        c + n.x * n.x * ac,
        xyac + zs,
        zxac - ys,
        xyac - zs,
        c + n.y * n.y * ac,
        yzac + xs,
        zxac + ys,
        yzac - xs,
        c + n.z * n.z * ac,
    );
    Ok(())
}

pub fn mat4_all_elements(
    m00: f32,
    m01: f32,
    m02: f32,
    m03: f32,
    m10: f32,
    m11: f32,
    m12: f32,
    m13: f32,
    m20: f32,
    m21: f32,
    m22: f32,
    m23: f32,
    m30: f32,
    m31: f32,
    m32: f32,
    m33: f32,
) -> Mat4 {
    let m0 = Vec4::new(m00, m01, m02, m03);
    let m1 = Vec4::new(m10, m11, m12, m13);
    let m2 = Vec4::new(m20, m21, m22, m23);
    let m3 = Vec4::new(m30, m31, m32, m33);
    Mat4::from_cols(m0, m1, m2, m3)
}

pub unsafe fn view(camera_pos: Vec3, direction: Vec3, up: Vec3) -> Mat4 {
    let n_z = Vec3::normalize(direction);
    let n_x = Vec3::normalize(Vec3::cross(up, n_z));
    let n_y = Vec3::cross(n_x, n_z);
    let orientation = mat4_all_elements(
        n_x.x, n_y.x, n_z.x, 0.0, n_x.y, n_y.y, n_z.y, 0.0, n_x.z, n_y.z, n_z.z, 0.0, 0.0, 0.0,
        0.0, 1.0,
    );
    let translate = mat4_all_elements(
        1.0f32,
        0.0f32,
        0.0f32,
        0.0f32,
        0.0f32,
        1.0f32,
        0.0f32,
        0.0f32,
        0.0f32,
        0.0f32,
        1.0f32,
        0.0f32,
        -camera_pos.x,
        -camera_pos.y,
        -camera_pos.z,
        1.0f32,
    );
    orientation * translate
}
