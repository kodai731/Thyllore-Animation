pub const CONVENTIONS: [&str; 8] = [
    "blender",
    "maya",
    "max_biped",
    "mixamo",
    "unreal",
    "vrm_normalized",
    "blender_apose",
    "adversarial",
];

fn left_role(role: &str) -> Option<&str> {
    match role {
        "LeftShoulder" | "LeftUpperArm" | "LeftLowerArm" | "LeftHand" | "LeftUpperLeg"
        | "LeftLowerLeg" | "LeftFoot" => Some(role),
        "RightShoulder" => Some("LeftShoulder"),
        "RightUpperArm" => Some("LeftUpperArm"),
        "RightLowerArm" => Some("LeftLowerArm"),
        "RightHand" => Some("LeftHand"),
        "RightUpperLeg" => Some("LeftUpperLeg"),
        "RightLowerLeg" => Some("LeftLowerLeg"),
        "RightFoot" => Some("LeftFoot"),
        _ => None,
    }
}

fn mirror_name(name: &str) -> String {
    let mut out = name.replace("Left", "Right");
    if out.contains(" L ") {
        out = out.replace(" L ", " R ");
    }
    if out.ends_with(".L") {
        out = out.trim_end_matches(".L").to_string() + ".R";
    }
    if out.ends_with("_l") {
        out = out.trim_end_matches("_l").to_string() + "_r";
    }
    if out.contains("_L") {
        out = out.replace("_L", "_R");
    }
    out
}

fn resolve_naming_convention(convention: &str) -> &str {
    match convention {
        "blender_apose" => "blender",
        "adversarial" => "mixamo",
        other => other,
    }
}

fn bone_name_for_convention(convention: &str, left_role: &str) -> &'static str {
    match (convention, left_role) {
        ("blender", "Hips") => "Hips",
        ("maya", "Hips") => "Hips",
        ("max_biped", "Hips") => "Bip001 Pelvis",
        ("mixamo", "Hips") => "mixamorig:Hips",
        ("unreal", "Hips") => "pelvis",
        ("vrm_normalized", "Hips") => "J_Bip_C_Hips",

        ("blender", "Spine") => "Spine",
        ("maya", "Spine") => "Spine",
        ("max_biped", "Spine") => "Bip001 Spine",
        ("mixamo", "Spine") => "mixamorig:Spine",
        ("unreal", "Spine") => "spine_01",
        ("vrm_normalized", "Spine") => "J_Bip_C_Spine",

        ("blender", "Chest") => "Chest",
        ("maya", "Chest") => "Spine1",
        ("max_biped", "Chest") => "Bip001 Spine1",
        ("mixamo", "Chest") => "mixamorig:Spine1",
        ("unreal", "Chest") => "spine_02",
        ("vrm_normalized", "Chest") => "J_Bip_C_Chest",

        ("blender", "Neck") => "Neck",
        ("maya", "Neck") => "Neck",
        ("max_biped", "Neck") => "Bip001 Neck",
        ("mixamo", "Neck") => "mixamorig:Neck",
        ("unreal", "Neck") => "neck_01",
        ("vrm_normalized", "Neck") => "J_Bip_C_Neck",

        ("blender", "Head") => "Head",
        ("maya", "Head") => "Head",
        ("max_biped", "Head") => "Bip001 Head",
        ("mixamo", "Head") => "mixamorig:Head",
        ("unreal", "Head") => "head",
        ("vrm_normalized", "Head") => "J_Bip_C_Head",

        ("blender", "LeftShoulder") => "Shoulder.L",
        ("maya", "LeftShoulder") => "LeftShoulder",
        ("max_biped", "LeftShoulder") => "Bip001 L Clavicle",
        ("mixamo", "LeftShoulder") => "mixamorig:LeftShoulder",
        ("unreal", "LeftShoulder") => "clavicle_l",
        ("vrm_normalized", "LeftShoulder") => "J_Bip_L_Shoulder",

        ("blender", "LeftUpperArm") => "Upper_arm.L",
        ("maya", "LeftUpperArm") => "LeftArm",
        ("max_biped", "LeftUpperArm") => "Bip001 L UpperArm",
        ("mixamo", "LeftUpperArm") => "mixamorig:LeftArm",
        ("unreal", "LeftUpperArm") => "upperarm_l",
        ("vrm_normalized", "LeftUpperArm") => "J_Bip_L_UpperArm",

        ("blender", "LeftLowerArm") => "Lower_arm.L",
        ("maya", "LeftLowerArm") => "LeftForeArm",
        ("max_biped", "LeftLowerArm") => "Bip001 L Forearm",
        ("mixamo", "LeftLowerArm") => "mixamorig:LeftForeArm",
        ("unreal", "LeftLowerArm") => "lowerarm_l",
        ("vrm_normalized", "LeftLowerArm") => "J_Bip_L_LowerArm",

        ("blender", "LeftHand") => "Hand.L",
        ("maya", "LeftHand") => "LeftHand",
        ("max_biped", "LeftHand") => "Bip001 L Hand",
        ("mixamo", "LeftHand") => "mixamorig:LeftHand",
        ("unreal", "LeftHand") => "hand_l",
        ("vrm_normalized", "LeftHand") => "J_Bip_L_Hand",

        ("blender", "LeftUpperLeg") => "Upper_leg.L",
        ("maya", "LeftUpperLeg") => "LeftUpLeg",
        ("max_biped", "LeftUpperLeg") => "Bip001 L Thigh",
        ("mixamo", "LeftUpperLeg") => "mixamorig:LeftUpLeg",
        ("unreal", "LeftUpperLeg") => "thigh_l",
        ("vrm_normalized", "LeftUpperLeg") => "J_Bip_L_UpperLeg",

        ("blender", "LeftLowerLeg") => "Lower_leg.L",
        ("maya", "LeftLowerLeg") => "LeftLeg",
        ("max_biped", "LeftLowerLeg") => "Bip001 L Calf",
        ("mixamo", "LeftLowerLeg") => "mixamorig:LeftLeg",
        ("unreal", "LeftLowerLeg") => "calf_l",
        ("vrm_normalized", "LeftLowerLeg") => "J_Bip_L_LowerLeg",

        ("blender", "LeftFoot") => "Foot.L",
        ("maya", "LeftFoot") => "LeftFoot",
        ("max_biped", "LeftFoot") => "Bip001 L Foot",
        ("mixamo", "LeftFoot") => "mixamorig:LeftFoot",
        ("unreal", "LeftFoot") => "foot_l",
        ("vrm_normalized", "LeftFoot") => "J_Bip_L_Foot",

        _ => panic!(
            "unknown convention {:?} or role {:?}",
            convention, left_role
        ),
    }
}

pub fn bone_name(convention: &str, role: &str) -> String {
    let left = match left_role(role) {
        Some(l) => l,
        None => {
            if matches!(role, "Hips" | "Spine" | "Chest" | "Neck" | "Head") {
                role
            } else {
                panic!("unknown role {:?}", role);
            }
        }
    };
    let name = bone_name_for_convention(resolve_naming_convention(convention), left);
    if role.starts_with("Right") {
        mirror_name(name)
    } else {
        name.to_string()
    }
}
