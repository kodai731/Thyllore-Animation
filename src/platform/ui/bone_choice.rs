use crate::animation::BoneId;
use crate::asset::AssetStorage;

use crate::platform::ui::ui_rows::ChoiceOption;

pub fn collect_bone_list(assets: &AssetStorage) -> Vec<(BoneId, String)> {
    let skeleton = match assets.skeletons.values().next() {
        Some(skel_asset) => &skel_asset.skeleton,
        None => return Vec::new(),
    };

    skeleton
        .bones
        .iter()
        .map(|bone| (bone.id, bone.name.clone()))
        .collect()
}

pub fn bone_options(
    bones: &[(BoneId, String)],
    selected_bone: Option<BoneId>,
    current: BoneId,
) -> Vec<ChoiceOption> {
    bones
        .iter()
        .map(|(bone_id, name)| {
            let prefix = if selected_bone == Some(*bone_id) {
                "* "
            } else {
                ""
            };
            ChoiceOption {
                label: format!("{}{}", prefix, name),
                selected: *bone_id == current,
            }
        })
        .collect()
}
