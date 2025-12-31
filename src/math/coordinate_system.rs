use cgmath::{Matrix4, SquareMatrix};

/// 座標系の統一管理: ワールド空間は右手系Y-up、Vulkan NDCはY-down

/// FBX Z-up → ワールド Y-up 変換（90度X軸回転）
pub fn fbx_to_world() -> Matrix4<f32> {
    Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// glTF Y-up → ワールド Y-up 変換（恒等変換）
pub fn gltf_to_world() -> Matrix4<f32> {
    Matrix4::identity()
}

/// Blender Z-up → ワールド Y-up 変換（FBXと同じ）
pub fn blender_to_world() -> Matrix4<f32> {
    fbx_to_world()
}

/// ワールド座標系のY軸（上向き）
pub fn world_y_axis() -> cgmath::Vector3<f32> {
    cgmath::vec3(0.0, 1.0, 0.0)
}

/// ワールド座標系のY軸負方向（下向き）
pub fn world_y_down() -> cgmath::Vector3<f32> {
    cgmath::vec3(0.0, -1.0, 0.0)
}

/// Vulkan NDC用Y軸反転（Y-up → Y-down）
pub fn projection_y_flip() -> Matrix4<f32> {
    Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// Vulkan深度範囲変換（OpenGL [-1,1] → Vulkan [0,1]）
pub fn depth_correction() -> Matrix4<f32> {
    Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0 / 2.0, 0.0,
        0.0, 0.0, 1.0 / 2.0, 1.0,
    )
}

/// Vulkan用Projection補正（Y反転 + 深度変換）
pub fn vulkan_projection_correction() -> Matrix4<f32> {
    depth_correction() * projection_y_flip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{vec3, vec4, Matrix4, SquareMatrix};

    #[test]
    fn test_fbx_to_world() {
        let transform = fbx_to_world();
        let fbx_up = vec4(0.0, 0.0, 1.0, 0.0);
        let world_up = transform * fbx_up;
        assert_eq!(world_up, vec4(0.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn test_gltf_to_world() {
        assert_eq!(gltf_to_world(), Matrix4::identity());
    }

    #[test]
    fn test_world_axes() {
        assert_eq!(world_y_axis(), vec3(0.0, 1.0, 0.0));
        assert_eq!(world_y_down(), vec3(0.0, -1.0, 0.0));
    }

    #[test]
    fn test_projection_y_flip() {
        let flip = projection_y_flip();
        let world_up = vec4(0.0, 1.0, 0.0, 0.0);
        let ndc_down = flip * world_up;
        assert_eq!(ndc_down, vec4(0.0, -1.0, 0.0, 0.0));
    }

    #[test]
    fn test_right_handed() {
        let x = vec3(1.0, 0.0, 0.0);
        let y = world_y_axis();
        let z = vec3(0.0, 0.0, 1.0);
        assert!((x.cross(y) - z).magnitude() < 1e-5);
    }
}
