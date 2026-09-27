use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct AvatarStats {
    pub triangles: u32,
    pub bones: u32,
    pub materials: u32,
    pub skinned_meshes: u32,
    pub meshes: u32,
    pub morph_meshes: u32,
    pub spring_chains: u32,
    pub spring_transforms: u32,
    pub spring_colliders: u32,
    pub texture_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_zeroed() {
        let stats = AvatarStats::default();
        assert_eq!(stats.triangles, 0);
        assert_eq!(stats.bones, 0);
        assert_eq!(stats.materials, 0);
        assert_eq!(stats.skinned_meshes, 0);
        assert_eq!(stats.meshes, 0);
        assert_eq!(stats.morph_meshes, 0);
        assert_eq!(stats.spring_chains, 0);
        assert_eq!(stats.spring_transforms, 0);
        assert_eq!(stats.spring_colliders, 0);
        assert_eq!(stats.texture_bytes, 0);
    }

    #[test]
    fn test_clone() {
        let stats = AvatarStats {
            triangles: 100,
            bones: 50,
            materials: 4,
            skinned_meshes: 2,
            meshes: 5,
            morph_meshes: 1,
            spring_chains: 1,
            spring_transforms: 8,
            spring_colliders: 2,
            texture_bytes: 1024,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.triangles, stats.triangles);
        assert_eq!(cloned.bones, stats.bones);
        assert_eq!(cloned.materials, stats.materials);
        assert_eq!(cloned.skinned_meshes, stats.skinned_meshes);
        assert_eq!(cloned.meshes, stats.meshes);
        assert_eq!(cloned.morph_meshes, stats.morph_meshes);
        assert_eq!(cloned.spring_chains, stats.spring_chains);
        assert_eq!(cloned.spring_transforms, stats.spring_transforms);
        assert_eq!(cloned.spring_colliders, stats.spring_colliders);
        assert_eq!(cloned.texture_bytes, stats.texture_bytes);
    }
}
