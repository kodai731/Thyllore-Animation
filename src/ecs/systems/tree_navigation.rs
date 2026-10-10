#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMove {
    Next,
    Prev,
    First,
    Last,
    PageDown,
    PageUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeStep {
    Select(usize),
    Expand(usize),
    Collapse(usize),
}

pub fn move_row(
    rows: &[TreeRow],
    current: Option<usize>,
    tree_move: TreeMove,
    page_rows: usize,
) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    let last = rows.len() - 1;
    let page = page_rows.max(1);

    let target = match (tree_move, current) {
        (TreeMove::First, _) => 0,
        (TreeMove::Last, _) => last,
        (_, None) => 0,
        (TreeMove::Next, Some(index)) => (index + 1).min(last),
        (TreeMove::Prev, Some(index)) => index.saturating_sub(1),
        (TreeMove::PageDown, Some(index)) => (index + page).min(last),
        (TreeMove::PageUp, Some(index)) => index.saturating_sub(page),
    };
    Some(target)
}

pub fn expand_or_descend(rows: &[TreeRow], current: usize) -> Option<TreeStep> {
    let row = rows.get(current)?;
    if !row.has_children {
        return None;
    }
    if !row.expanded {
        return Some(TreeStep::Expand(current));
    }
    first_child_row(rows, current).map(TreeStep::Select)
}

pub fn collapse_or_ascend(rows: &[TreeRow], current: usize) -> Option<TreeStep> {
    let row = rows.get(current)?;
    if row.has_children && row.expanded {
        return Some(TreeStep::Collapse(current));
    }
    parent_row(rows, current).map(TreeStep::Select)
}

pub fn parent_row(rows: &[TreeRow], current: usize) -> Option<usize> {
    let depth = rows.get(current)?.depth;
    if depth == 0 {
        return None;
    }
    (0..current).rev().find(|&index| rows[index].depth < depth)
}

pub fn first_child_row(rows: &[TreeRow], current: usize) -> Option<usize> {
    let depth = rows.get(current)?.depth;
    rows.get(current + 1)
        .filter(|row| row.depth == depth + 1)
        .map(|_| current + 1)
}

pub fn sibling_rows(rows: &[TreeRow], current: usize) -> Vec<usize> {
    let Some(row) = rows.get(current) else {
        return Vec::new();
    };
    let depth = row.depth;
    let start = parent_row(rows, current).map_or(0, |parent| parent + 1);
    let end = (current + 1..rows.len())
        .find(|&index| rows[index].depth < depth)
        .unwrap_or(rows.len());

    (start..end)
        .filter(|&index| rows[index].depth == depth)
        .collect()
}

pub fn type_ahead_row<'a>(
    names: impl Iterator<Item = &'a str>,
    current: Option<usize>,
    prefix: &str,
) -> Option<usize> {
    let prefix_lower = prefix.to_lowercase();
    if prefix_lower.is_empty() {
        return None;
    }
    let matches: Vec<usize> = names
        .enumerate()
        .filter(|(_, name)| name.to_lowercase().starts_with(&prefix_lower))
        .map(|(index, _)| index)
        .collect();

    let after_current = current.map_or(0, |index| index + 1);
    matches
        .iter()
        .copied()
        .find(|&index| index >= after_current)
        .or_else(|| matches.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(depth: usize, has_children: bool, expanded: bool) -> TreeRow {
        TreeRow {
            depth,
            has_children,
            expanded,
        }
    }

    fn sample_rows() -> Vec<TreeRow> {
        vec![
            row(0, true, true),
            row(1, false, false),
            row(1, true, false),
            row(1, true, true),
            row(2, false, false),
            row(0, false, false),
        ]
    }

    #[test]
    fn next_and_prev_stop_at_the_ends() {
        let rows = sample_rows();
        assert_eq!(move_row(&rows, Some(5), TreeMove::Next, 1), Some(5));
        assert_eq!(move_row(&rows, Some(0), TreeMove::Prev, 1), Some(0));
        assert_eq!(move_row(&rows, Some(2), TreeMove::Next, 1), Some(3));
        assert_eq!(move_row(&rows, None, TreeMove::Next, 1), Some(0));
        assert_eq!(move_row(&[], None, TreeMove::Next, 1), None);
    }

    #[test]
    fn home_end_and_pages() {
        let rows = sample_rows();
        assert_eq!(move_row(&rows, Some(3), TreeMove::First, 1), Some(0));
        assert_eq!(move_row(&rows, Some(3), TreeMove::Last, 1), Some(5));
        assert_eq!(move_row(&rows, Some(1), TreeMove::PageDown, 3), Some(4));
        assert_eq!(move_row(&rows, Some(4), TreeMove::PageUp, 10), Some(0));
    }

    #[test]
    fn right_expands_then_descends() {
        let rows = sample_rows();
        assert_eq!(expand_or_descend(&rows, 2), Some(TreeStep::Expand(2)));
        assert_eq!(expand_or_descend(&rows, 3), Some(TreeStep::Select(4)));
        assert_eq!(expand_or_descend(&rows, 1), None);
    }

    #[test]
    fn left_collapses_then_ascends() {
        let rows = sample_rows();
        assert_eq!(collapse_or_ascend(&rows, 3), Some(TreeStep::Collapse(3)));
        assert_eq!(collapse_or_ascend(&rows, 4), Some(TreeStep::Select(3)));
        assert_eq!(collapse_or_ascend(&rows, 1), Some(TreeStep::Select(0)));
        assert_eq!(collapse_or_ascend(&rows, 5), None);
    }

    #[test]
    fn siblings_share_depth_and_parent() {
        let rows = sample_rows();
        assert_eq!(sibling_rows(&rows, 2), vec![1, 2, 3]);
        assert_eq!(sibling_rows(&rows, 0), vec![0, 5]);
        assert_eq!(sibling_rows(&rows, 4), vec![4]);
    }

    #[test]
    fn type_ahead_searches_after_current_then_wraps() {
        let names = ["Shinano", "Shinano_01", "Hips", "Shinano_02"];
        assert_eq!(type_ahead_row(names.into_iter(), Some(1), "sh"), Some(3));
        assert_eq!(type_ahead_row(names.into_iter(), Some(3), "sh"), Some(0));
        assert_eq!(type_ahead_row(names.into_iter(), None, "hi"), Some(2));
        assert_eq!(type_ahead_row(names.into_iter(), None, "zz"), None);
    }
}
