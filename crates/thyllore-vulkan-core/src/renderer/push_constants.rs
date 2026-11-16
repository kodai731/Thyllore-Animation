#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GBufferPushConstants {
    pub object_id: u32,
    pub heatmap_mode: u32,
}

impl GBufferPushConstants {
    pub fn new(object_id: u32, heatmap_mode: u32) -> Self {
        Self {
            object_id,
            heatmap_mode,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OnionSkinPushConstants {
    pub ghost_tint_r: f32,
    pub ghost_tint_g: f32,
    pub ghost_tint_b: f32,
    pub ghost_opacity: f32,
    pub debug_mode: i32,
    pub _pad: [f32; 3],
}

impl OnionSkinPushConstants {
    pub fn new(tint_color: [f32; 3], opacity: f32) -> Self {
        Self {
            ghost_tint_r: tint_color[0],
            ghost_tint_g: tint_color[1],
            ghost_tint_b: tint_color[2],
            ghost_opacity: opacity,
            debug_mode: 0,
            _pad: [0.0; 3],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

/// Shading settings every effect overlay hands its fragment shader: the shading mode, the
/// reference step count and the debug view.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShadingPushConstants {
    pub mode: i32,
    pub step_count: i32,
    pub debug_view: i32,
}

impl ShadingPushConstants {
    pub fn new(mode: i32, step_count: i32, debug_view: i32) -> Self {
        Self {
            mode,
            step_count,
            debug_view,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbuffer_push_constants_size() {
        assert_eq!(std::mem::size_of::<GBufferPushConstants>(), 8);
    }

    #[test]
    fn test_gbuffer_push_constants_as_bytes() {
        let pc = GBufferPushConstants::new(42, 1);
        let bytes = pc.as_bytes();
        assert_eq!(bytes.len(), 8);
        let object_id = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(object_id, 42);
        let heatmap_mode = u32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(heatmap_mode, 1);
    }

    #[test]
    fn test_onion_skin_push_constants_size() {
        assert_eq!(std::mem::size_of::<OnionSkinPushConstants>(), 32);
    }

    #[test]
    fn test_onion_skin_push_constants_values() {
        let pc = OnionSkinPushConstants::new([0.2, 0.4, 1.0], 0.5);
        assert!((pc.ghost_tint_r - 0.2).abs() < f32::EPSILON);
        assert!((pc.ghost_tint_g - 0.4).abs() < f32::EPSILON);
        assert!((pc.ghost_tint_b - 1.0).abs() < f32::EPSILON);
        assert!((pc.ghost_opacity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_onion_skin_push_constants_as_bytes() {
        let pc = OnionSkinPushConstants::new([1.0, 2.0, 3.0], 4.0);
        let bytes = pc.as_bytes();
        assert_eq!(bytes.len(), 32);
        let tint_r = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert!((tint_r - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn shading_push_constants_are_three_ints() {
        assert_eq!(std::mem::size_of::<ShadingPushConstants>(), 12);
    }
}
