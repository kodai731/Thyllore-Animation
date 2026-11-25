use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct TorusProjection {
    pub point: Vec3,
    pub normal: Vec3,
    pub u: f32,
    pub v: f32,
}

pub fn project_to_torus(p: Vec3, major_radius: f32, r: f32) -> TorusProjection {
    let q = Vec3::new(p.x, 0.0, p.z);
    let q_mag = q.length();

    let q_normalized = if q_mag < 1e-6 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        q / q_mag
    };

    let n = (p - major_radius * q_normalized).normalize();
    let point = major_radius * q_normalized + r * n;

    let u = q_normalized.z.atan2(q_normalized.x);
    let v = n.y.atan2(n.dot(q_normalized));

    TorusProjection {
        point,
        normal: n,
        u,
        v,
    }
}

pub fn torus_surface_point(u: f32, v: f32, major_radius: f32, r: f32) -> Vec3 {
    let cos_v = v.cos();
    let sin_v = v.sin();
    let cos_u = u.cos();
    let sin_u = u.sin();

    let radius = major_radius + r * cos_v;
    Vec3::new(radius * cos_u, r * sin_v, radius * sin_u)
}

pub fn torus_surface_normal(u: f32, v: f32) -> Vec3 {
    let cos_v = v.cos();
    let sin_v = v.sin();
    let cos_u = u.cos();
    let sin_u = u.sin();

    Vec3::new(cos_v * cos_u, sin_v, cos_v * sin_u)
}
