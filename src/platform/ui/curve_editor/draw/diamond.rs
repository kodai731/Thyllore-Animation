#[derive(Clone, Copy)]
pub(in crate::platform::ui::curve_editor) enum DiamondStyle {
    Filled,
    Outline { thickness: f32 },
}

#[derive(Clone, Copy)]
pub(in crate::platform::ui::curve_editor) struct KeyDiamond {
    pub center: [f32; 2],
    pub half_size: f32,
}

pub(in crate::platform::ui::curve_editor) fn diamond_corners(diamond: KeyDiamond) -> [[f32; 2]; 4] {
    let [x, y] = diamond.center;
    let half_size = diamond.half_size;
    [
        [x, y - half_size],
        [x + half_size, y],
        [x, y + half_size],
        [x - half_size, y],
    ]
}

pub(in crate::platform::ui::curve_editor) fn draw_key_diamond(
    draw_list: &imgui::DrawListMut,
    diamond: KeyDiamond,
    color: [f32; 4],
    style: DiamondStyle,
) {
    let corners = diamond_corners(diamond);

    match style {
        DiamondStyle::Filled => {
            let [top, right, bottom, left] = corners;
            draw_list
                .add_triangle(top, right, bottom, color)
                .filled(true)
                .build();
            draw_list
                .add_triangle(top, bottom, left, color)
                .filled(true)
                .build();
        }
        DiamondStyle::Outline { thickness } => {
            for (index, corner) in corners.iter().enumerate() {
                let next_corner = corners[(index + 1) % corners.len()];
                draw_list
                    .add_line(*corner, next_corner, color)
                    .thickness(thickness)
                    .build();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_run_clockwise_from_the_top_at_half_size_from_the_center() {
        let corners = diamond_corners(KeyDiamond {
            center: [10.0, 20.0],
            half_size: 5.0,
        });

        assert_eq!(
            corners,
            [[10.0, 15.0], [15.0, 20.0], [10.0, 25.0], [5.0, 20.0]]
        );
    }
}
