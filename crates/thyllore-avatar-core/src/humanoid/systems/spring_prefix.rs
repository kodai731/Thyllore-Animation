use crate::humanoid::components::skeleton_input::BoneInput;
use crate::humanoid::components::spring_chain::PrefixChain;

pub fn find_prefix_chain_roots(bones: &[BoneInput], prefix: &str) -> Vec<PrefixChain> {
    let mut roots = Vec::new();

    for (i, bone) in bones.iter().enumerate() {
        if !bone.name.starts_with(prefix) {
            continue;
        }

        let is_parent_prefixed = bone
            .parent
            .and_then(|parent_index| bones.get(parent_index))
            .is_some_and(|parent| parent.name.starts_with(prefix));
        if is_parent_prefixed {
            continue;
        }

        let length = count_chain_length(bones, i, prefix);
        roots.push(PrefixChain { root: i, length });
    }

    roots
}

fn count_chain_length(bones: &[BoneInput], root: usize, prefix: &str) -> u32 {
    let mut length = 1;
    let mut current = root;

    loop {
        let prefixed_children: Vec<usize> = bones
            .iter()
            .enumerate()
            .filter(|(_, bone)| bone.parent == Some(current) && bone.name.starts_with(prefix))
            .map(|(index, _)| index)
            .collect();
        let [only_child] = prefixed_children[..] else {
            return length;
        };
        length += 1;
        current = only_child;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &str, parent: Option<usize>) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent,
            rest_position: [0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn test_braid_chain() {
        let bones = vec![
            bone("Hips", None),
            bone("Left_braid_long", Some(0)),
            bone("Left_braid_long.001", Some(1)),
            bone("Left_braid_long.002", Some(2)),
        ];

        let roots = find_prefix_chain_roots(&bones, "Left_braid_long");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 1);
        assert_eq!(roots[0].length, 3);
    }

    #[test]
    fn test_two_side_chains() {
        let bones = vec![
            bone("Hips", None),
            bone("Right_side_x", Some(0)),
            bone("Right_side_x.001", Some(1)),
            bone("Left_side_x", Some(0)),
            bone("Left_side_x.001", Some(3)),
            bone("Left_side_x.002", Some(4)),
        ];

        let roots = find_prefix_chain_roots(&bones, "Right_side_x");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 1);
        assert_eq!(roots[0].length, 2);

        let roots = find_prefix_chain_roots(&bones, "Left_side_x");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 3);
        assert_eq!(roots[0].length, 3);
    }

    #[test]
    fn test_non_root_skipped() {
        let bones = vec![
            bone("Hips", None),
            bone("Left_braid_long", Some(0)),
            bone("Left_braid_long.001", Some(1)),
        ];

        let roots = find_prefix_chain_roots(&bones, "Left_braid_long");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 1);
    }

    #[test]
    fn test_fork_stops_at_root() {
        let bones = vec![
            bone("Hips", None),
            bone("Left_braid_long", Some(0)),
            bone("Left_braid_long.001", Some(1)),
            bone("Left_braid_long.002", Some(1)),
        ];

        let roots = find_prefix_chain_roots(&bones, "Left_braid_long");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 1);
        assert_eq!(roots[0].length, 1);
    }

    #[test]
    fn test_no_match() {
        let bones = vec![bone("Hips", None), bone("Spine", Some(0))];
        let roots = find_prefix_chain_roots(&bones, "Left_braid_long");
        assert!(roots.is_empty());
    }

    #[test]
    fn test_root_only() {
        let bones = vec![bone("Hips", None), bone("Left_braid_long", Some(0))];
        let roots = find_prefix_chain_roots(&bones, "Left_braid_long");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, 1);
        assert_eq!(roots[0].length, 1);
    }
}
