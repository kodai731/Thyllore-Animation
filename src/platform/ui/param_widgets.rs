use thyllore_scene_core::{find_ui_param, ScalarParam, UiParam};

use super::ui_rows::UiRow;

pub use super::ui_rows::EditedScalars;

/// `(group heading, names)`; `None` holds the ungrouped names and always comes last.
pub type ParamGroup = (Option<&'static str>, Vec<&'static str>);

pub fn determine_primary_order(ui_params: &[UiParam], hidden: &[&str]) -> Vec<&'static str> {
    ui_params
        .iter()
        .filter(|param| param.primary && !hidden.contains(&param.name))
        .map(|param| param.name)
        .collect()
}

pub fn group_remaining_params(ui_params: &[UiParam], hidden: &[&str]) -> Vec<ParamGroup> {
    let remaining: Vec<&UiParam> = ui_params
        .iter()
        .filter(|param| !param.primary && !hidden.contains(&param.name))
        .collect();

    let mut groups: Vec<ParamGroup> = Vec::new();
    for param in remaining.iter().filter(|param| !param.group.is_empty()) {
        match groups
            .iter_mut()
            .find(|(heading, _)| *heading == Some(param.group))
        {
            Some((_, names)) => names.push(param.name),
            None => groups.push((Some(param.group), vec![param.name])),
        }
    }

    let ungrouped: Vec<&'static str> = remaining
        .iter()
        .filter(|param| param.group.is_empty())
        .map(|param| param.name)
        .collect();
    if !ungrouped.is_empty() {
        groups.push((None, ungrouped));
    }

    groups
}

pub fn rows_from_params<C>(
    ui_params: &'static [UiParam],
    scalars: &'static [ScalarParam<C>],
    hidden: &[&str],
) -> Vec<UiRow<C>> {
    let primary_names = determine_primary_order(ui_params, hidden);
    let mut rows: Vec<UiRow<C>> = primary_names
        .iter()
        .filter_map(|name| find_ui_param(ui_params, name))
        .map(|param| UiRow::Scalar { param, scalars })
        .collect();

    let remaining_groups = group_remaining_params(ui_params, hidden);
    if remaining_groups.is_empty() {
        return rows;
    }

    let mut group_rows: Vec<UiRow<C>> = Vec::new();
    for (heading, names) in &remaining_groups {
        if let Some(group_heading) = heading {
            group_rows.push(UiRow::Heading(group_heading));
        }
        for name in names {
            if let Some(param) = find_ui_param(ui_params, name) {
                group_rows.push(UiRow::Scalar { param, scalars });
            }
        }
    }
    rows.push(UiRow::Group {
        title: "Advanced".into(),
        initially_open: false,
        rows: group_rows,
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use thyllore_scene_core::UiKind;

    fn make_ui_param(name: &'static str, group: &'static str, primary: bool) -> UiParam {
        UiParam {
            name,
            path: name,
            group,
            label: None,
            kind: UiKind::Scalar,
            min: 0.0,
            max: 1.0,
            format: "%.3f",
            tooltip: "",
            persisted: true,
            primary,
        }
    }

    fn mixed_params() -> [UiParam; 6] {
        [
            make_ui_param("a", "g1", true),
            make_ui_param("b", "g1", false),
            make_ui_param("c", "", false),
            make_ui_param("d", "g2", false),
            make_ui_param("e", "g1", false),
            make_ui_param("f", "g2", true),
        ]
    }

    #[test]
    fn primary_order_follows_declaration_order() {
        assert_eq!(determine_primary_order(&mixed_params(), &[]), ["a", "f"]);
    }

    #[test]
    fn primary_order_skips_hidden() {
        assert_eq!(determine_primary_order(&mixed_params(), &["a"]), ["f"]);
    }

    #[test]
    fn remaining_groups_keep_first_appearance_order_and_put_ungrouped_last() {
        assert_eq!(
            group_remaining_params(&mixed_params(), &[]),
            [
                (Some("g1"), vec!["b", "e"]),
                (Some("g2"), vec!["d"]),
                (None, vec!["c"]),
            ]
        );
    }

    #[test]
    fn remaining_groups_drop_a_group_whose_names_are_all_hidden() {
        assert_eq!(
            group_remaining_params(&mixed_params(), &["d", "c"]),
            [(Some("g1"), vec!["b", "e"])]
        );
    }

    #[test]
    fn remaining_groups_are_empty_when_every_param_is_primary() {
        let params = [make_ui_param("a", "g1", true), make_ui_param("b", "", true)];
        assert!(group_remaining_params(&params, &[]).is_empty());
    }

    #[test]
    fn primary_and_remaining_partition_every_param() {
        let params = mixed_params();
        let mut drawn: Vec<&str> = determine_primary_order(&params, &[]);
        for (_, names) in group_remaining_params(&params, &[]) {
            drawn.extend(names);
        }
        drawn.sort_unstable();
        assert_eq!(drawn, ["a", "b", "c", "d", "e", "f"]);
    }

    fn describe_rows<C>(rows: &[super::super::ui_rows::UiRow<C>]) -> Vec<String> {
        let mut out = Vec::new();
        for row in rows {
            match row {
                super::super::ui_rows::UiRow::Scalar { param, .. } => {
                    out.push(format!("Scalar({})", param.name));
                }
                super::super::ui_rows::UiRow::Heading(label) => {
                    out.push(format!("Heading({})", label));
                }
                super::super::ui_rows::UiRow::Group { title, rows, .. } => {
                    out.push(format!("Group({})", title));
                    out.extend(describe_rows(rows));
                }
                _ => {}
            }
        }
        out
    }

    #[test]
    fn rows_from_params_primary_then_advanced_group() {
        let params: &'static [UiParam] = Box::leak(mixed_params().into());
        let scalars: &'static [ScalarParam<()>] = &[];
        let rows = rows_from_params::<()>(params, scalars, &[]);
        let described = describe_rows(&rows);
        assert_eq!(
            described,
            [
                "Scalar(a)",
                "Scalar(f)",
                "Group(Advanced)",
                "Heading(g1)",
                "Scalar(b)",
                "Scalar(e)",
                "Heading(g2)",
                "Scalar(d)",
                "Scalar(c)",
            ]
        );
    }

    #[test]
    fn rows_from_params_hidden_excluded() {
        let params: &'static [UiParam] = Box::leak(mixed_params().into());
        let scalars: &'static [ScalarParam<()>] = &[];
        let rows = rows_from_params::<()>(params, scalars, &["a", "c"]);
        let described = describe_rows(&rows);
        assert_eq!(
            described,
            [
                "Scalar(f)",
                "Group(Advanced)",
                "Heading(g1)",
                "Scalar(b)",
                "Scalar(e)",
                "Heading(g2)",
                "Scalar(d)",
            ]
        );
    }

    #[test]
    fn rows_from_params_all_primary_has_no_group() {
        let params: &'static [UiParam] =
            Box::leak([make_ui_param("a", "g1", true), make_ui_param("b", "", true)].into());
        let scalars: &'static [ScalarParam<()>] = &[];
        let rows = rows_from_params::<()>(params, scalars, &[]);
        let described = describe_rows(&rows);
        assert_eq!(described, ["Scalar(a)", "Scalar(b)"]);
    }
}
