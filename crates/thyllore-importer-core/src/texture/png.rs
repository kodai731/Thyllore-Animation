use anyhow::{bail, Result};
use std::fs::File;

const OPAQUE: u8 = 255;

/// Decodes a PNG of any colour type and bit depth into 8 bit RGBA pixels.
pub fn load_png_image(path: &str) -> Result<(Vec<u8>, u32, u32)> {
    let image_file = File::open(path)?;
    let mut decoder = png::Decoder::new(image_file);
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info()?;
    let mut decoded = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut decoded)?;
    decoded.truncate(frame.buffer_size());

    let pixels = convert_to_rgba(decoded, frame.color_type)?;
    Ok((pixels, frame.width, frame.height))
}

fn convert_to_rgba(decoded: Vec<u8>, color_type: png::ColorType) -> Result<Vec<u8>> {
    let pixels = match color_type {
        png::ColorType::Rgba => decoded,
        png::ColorType::Rgb => decoded
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], OPAQUE])
            .collect(),
        png::ColorType::GrayscaleAlpha => decoded
            .chunks_exact(2)
            .flat_map(|gray_alpha| [gray_alpha[0], gray_alpha[0], gray_alpha[0], gray_alpha[1]])
            .collect(),
        png::ColorType::Grayscale => decoded
            .iter()
            .flat_map(|&gray| [gray, gray, gray, OPAQUE])
            .collect(),
        png::ColorType::Indexed => bail!("PNG palette was not expanded by the decoder"),
    };
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_png(path: &std::path::Path, color_type: png::ColorType, pixels: &[u8]) {
        let mut encoder = png::Encoder::new(File::create(path).unwrap(), 2, 1);
        encoder.set_color(color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
    }

    fn load_pixels(color_type: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let dir = tempdir().unwrap();
        let path = dir.path().join("image.png");
        write_png(&path, color_type, pixels);

        let (rgba, width, height) = load_png_image(path.to_str().unwrap()).unwrap();

        assert_eq!((width, height), (2, 1));
        rgba
    }

    #[test]
    fn test_rgba_is_unchanged() {
        let pixels = [10, 20, 30, 40, 50, 60, 70, 80];
        assert_eq!(load_pixels(png::ColorType::Rgba, &pixels), pixels);
    }

    #[test]
    fn test_rgb_gains_opaque_alpha() {
        assert_eq!(
            load_pixels(png::ColorType::Rgb, &[10, 20, 30, 50, 60, 70]),
            [10, 20, 30, 255, 50, 60, 70, 255]
        );
    }

    #[test]
    fn test_grayscale_alpha_spreads_to_rgb() {
        assert_eq!(
            load_pixels(png::ColorType::GrayscaleAlpha, &[10, 40, 50, 80]),
            [10, 10, 10, 40, 50, 50, 50, 80]
        );
    }

    #[test]
    fn test_grayscale_gains_opaque_alpha() {
        assert_eq!(
            load_pixels(png::ColorType::Grayscale, &[10, 50]),
            [10, 10, 10, 255, 50, 50, 50, 255]
        );
    }
}
