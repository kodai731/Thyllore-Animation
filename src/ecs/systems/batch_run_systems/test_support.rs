use std::path::Path;

use crate::ecs::events::UiCommandQueue;
use crate::ecs::world::World;

pub(super) fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

pub(super) fn write_test_png(path: &Path, width: u32, height: u32, value: u8) {
    let file = std::fs::File::create(path).unwrap();
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    let pixels = vec![value; (width * height * 3) as usize];
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
}

pub(super) fn drained_command_names(world: &World) -> Vec<String> {
    world
        .resource_mut::<UiCommandQueue>()
        .drain()
        .map(|command| format!("{:?}", command))
        .collect()
}
