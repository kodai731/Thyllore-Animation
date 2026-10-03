use thyllore_anim_core::editable::PropertyType;

use crate::ecs::component::scalar_channel_domains;

fn scalar_channel_table() -> String {
    let mut lines: Vec<String> = Vec::new();
    for domain in scalar_channel_domains() {
        for (i, channel) in domain.channels.iter().enumerate() {
            let code = domain.property_type_at(i);
            let PropertyType::Custom(code_value) = code else {
                unreachable!("property_type_at always returns Custom");
            };
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}",
                domain.name,
                code_value,
                channel.cli_name,
                channel.scene_name,
                channel.display_name,
                channel.debug_value_range.0,
                channel.debug_value_range.1,
            ));
        }
    }
    lines.join("\n")
}

mod tests {
    use super::*;

    #[test]
    fn test_scalar_channel_table_matches_golden() {
        let expected = include_str!("scalar_channel_table.golden.txt");
        let actual = scalar_channel_table();
        assert_eq!(
            actual,
            expected.trim(),
            "scalar channel table does not match golden file"
        );
    }
}
