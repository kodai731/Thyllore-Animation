use crate::lightning::{
    LightningBranching, LightningLook, LightningParameterOwner, LightningShape, LightningSurround,
    LightningTiming,
};
use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(Clone, Debug, PartialEq, thyllore_scene_core::SceneFields)]
#[scene(key = "lightning", tag = LightningParameterOwner, owner = Frame, tags = LIGHTNING_PARAMETER_OWNERSHIP, snapshot = lightning_parameter_snapshot, scalars = LIGHTNING_SCALAR_PARAMS, ui = LIGHTNING_UI_PARAMS, overwrite = overwrite_lightning_persisted_fields)]
pub struct LightningEffect {
    #[persist(as = [f32; 3])]
    pub position: Vector3<f32>,
    #[persist(as = [f32; 4], with = crate::scene_convert::quaternion_wxyz)]
    pub rotation: Quaternion<f32>,
    #[runtime(ui(min = 0.0, max = 100.0))]
    pub time: f32,
    #[runtime(ui(primary, min = 0.0, max = 4.0))]
    pub time_scale: f32,
    #[runtime(ui(min = -100.0, max = 100.0))]
    pub time_offset: f32,
    #[nested]
    pub shape: LightningShape,
    #[nested]
    pub branch: LightningBranching,
    #[nested]
    pub look: LightningLook,
    #[nested]
    pub timing: LightningTiming,
    #[nested]
    pub surround: LightningSurround,
}

impl Default for LightningEffect {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            time: 0.0,
            time_scale: 1.0,
            time_offset: 0.0,
            shape: LightningShape::default(),
            branch: LightningBranching::default(),
            look: LightningLook::default(),
            timing: LightningTiming::default(),
            surround: LightningSurround::default(),
        }
    }
}

pub fn build_lightning_model_matrix(effect: &LightningEffect) -> Matrix4<f32> {
    Matrix4::from_translation(effect.position) * Matrix4::from(effect.rotation)
}

impl crate::EffectPresets for LightningEffect {
    const PRESET_NAMES: &'static [&'static str] = crate::LIGHTNING_PRESET_NAMES;

    fn apply_preset(&mut self, name: &str) -> bool {
        crate::apply_lightning_preset(self, name)
    }
}

impl crate::Placement for LightningEffect {
    fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
        self.time = time;
        self.position = Vector3::from(position);
        crate::scene_convert::quaternion_wxyz::set(&mut self.rotation, rotation);
    }
}
