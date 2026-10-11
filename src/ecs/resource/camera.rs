use cgmath::{Deg, InnerSpace, Vector3};
use thyllore_scene_core::declare_scene_format;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub pivot: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraTransition {
    pub from: CameraPose,
    pub to: CameraPose,
    pub elapsed_seconds: f32,
    pub duration_seconds: f32,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub pivot: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: Deg<f32>,
    pub near_plane: f32,

    pub initial_pivot: Vector3<f32>,
    pub initial_yaw: f32,
    pub initial_pitch: f32,
    pub initial_distance: f32,

    pub transition: Option<CameraTransition>,
    pub fly_speed_scale: f32,
    pub fly_speed_indicator_seconds: f32,
}

impl Camera {
    pub const DEFAULT_FLY_SPEED_SCALE: f32 = 1.5;

    pub fn pose(&self) -> CameraPose {
        CameraPose {
            pivot: self.pivot,
            yaw: self.yaw,
            pitch: self.pitch,
            distance: self.distance,
        }
    }

    pub fn set_pose(&mut self, pose: CameraPose) {
        self.pivot = pose.pivot;
        self.yaw = pose.yaw;
        self.pitch = pose.pitch;
        self.distance = pose.distance;
    }
}

impl Default for Camera {
    fn default() -> Self {
        let initial_pos = Vector3::new(5.0_f32, 5.0, 5.0);
        let pivot = Vector3::new(0.0_f32, 0.0, 0.0);
        let diff = initial_pos - pivot;
        let distance: f32 = diff.magnitude();
        let yaw: f32 = diff.x.atan2(diff.z);
        let pitch: f32 = (diff.y / distance).asin();

        Self {
            pivot,
            yaw,
            pitch,
            distance,
            fov_y: Deg(45.0),
            near_plane: 0.1,
            initial_pivot: pivot,
            initial_yaw: yaw,
            initial_pitch: pitch,
            initial_distance: distance,
            transition: None,
            fly_speed_scale: Self::DEFAULT_FLY_SPEED_SCALE,
            fly_speed_indicator_seconds: 0.0,
        }
    }
}

declare_scene_format! {
    component: Camera,
    record: CameraSceneRecord,
    items {
        key: "camera",
        snapshot: camera_parameter_snapshot,
        scalars: CAMERA_SCALAR_PARAMS,
        ui: CAMERA_UI_PARAMS,
        overwrite: overwrite_camera_persisted_fields,
    },
    persisted {
        pivot: [f32; 3] {
            get: |c| [c.pivot.x, c.pivot.y, c.pivot.z],
            set: |c, v| {
                c.pivot = Vector3::new(v[0], v[1], v[2]);
                c.initial_pivot = c.pivot;
            },
        },
        yaw: f32 {
            get: |c| c.yaw,
            set: |c, v| {
                c.yaw = v;
                c.initial_yaw = v;
            },
        },
        pitch: f32 {
            get: |c| c.pitch,
            set: |c, v| {
                c.pitch = v;
                c.initial_pitch = v;
            },
        },
        distance: f32 {
            get: |c| c.distance,
            set: |c, v| {
                c.distance = v;
                c.initial_distance = v;
            },
        },
        fov_y: f32 { get: |c| c.fov_y.0, set: |c, v| c.fov_y = Deg(v) },
        fly_speed_scale: f32 { get: |c| c.fly_speed_scale, set: |c, v| c.fly_speed_scale = v },
    },
    runtime {},
}

crate::scene_resource!(Camera);
