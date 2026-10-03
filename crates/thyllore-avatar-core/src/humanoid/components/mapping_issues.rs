use super::role::HumanoidRole;

#[derive(Clone, Debug)]
pub enum MappingIssue {
    MissingRequired(HumanoidRole),
    HierarchyOrder {
        child: HumanoidRole,
        expected_ancestor: HumanoidRole,
    },
}
