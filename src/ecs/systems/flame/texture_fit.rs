use std::io::Cursor;

use serde_json::json;
use thyllore_effect_core::TextureFitGroups;

use super::{resolve_selected_flame, write_flame_transform};
use crate::ecs::component::{FlameBaked, FlameEffect};
use crate::ecs::systems::flame_dump_systems::write_texture_fit_provenance;
use crate::ecs::world::World;

/// Run a texture fit against the selected flame and write the results into
/// its parameter and baked components. The heavy fit itself stays in
/// render-core / texture-fit-core; this system owns the component I/O.
pub fn apply_flame_texture_fit_to_selected(
    world: &mut World,
    path: &str,
    blend: f32,
    groups: TextureFitGroups,
    profile: bool,
    route: &str,
) {
    let Some(target) = resolve_selected_flame(world) else {
        return;
    };
    let Some(mut effect) = world
        .get_component::<FlameEffect>(target)
        .map(|e| e.clone())
    else {
        return;
    };
    let mut baked = world
        .get_component::<FlameBaked>(target)
        .cloned()
        .unwrap_or_default();

    apply_texture_fit_from_path(&mut effect, &mut baked, path, blend, groups, profile, route);

    write_flame_transform(world, target, effect.position, effect.rotation);
    world.insert_component(target, effect);
    world.insert_component(target, baked);
}

/// Decodes the reference PNG, fits the flame to it and records the provenance either way.
pub fn apply_texture_fit_from_path(
    effect: &mut FlameEffect,
    baked: &mut FlameBaked,
    path: &str,
    blend: f32,
    groups: TextureFitGroups,
    profile: bool,
    route: &str,
) {
    let effect_before = effect.clone();
    let baked_before = *baked;
    let request = json!({
        "blend": blend,
        "profile": profile,
        "groups": {
            "silhouette": groups.silhouette,
            "color": groups.color,
            "turbulence": groups.turbulence,
            "tilt": groups.tilt,
        },
    });
    let dump = |source_bytes: Option<&[u8]>,
                result: serde_json::Value,
                effect_after: &FlameEffect,
                baked_after: &FlameBaked| {
        write_texture_fit_provenance(
            route,
            path,
            source_bytes,
            request.clone(),
            result,
            (&effect_before, &baked_before),
            (effect_after, baked_after),
        );
    };

    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "warning: failed to read texture fit image '{}': {}",
                path, e
            );
            dump(
                None,
                json!({"ok": false, "error": "not_found", "detail": e.to_string()}),
                effect,
                baked,
            );
            return;
        }
    };

    let decoder = png::Decoder::new(Cursor::new(&bytes));
    let mut reader = match decoder.read_info() {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "warning: failed to decode texture fit image '{}': {}",
                path, e
            );
            dump(
                Some(&bytes),
                json!({"ok": false, "error": "decode_failed", "detail": e.to_string()}),
                effect,
                baked,
            );
            return;
        }
    };

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = match reader.next_frame(&mut buf) {
        Ok(i) => i,
        Err(e) => {
            eprintln!(
                "warning: failed to read texture fit image frame '{}': {}",
                path, e
            );
            dump(
                Some(&bytes),
                json!({"ok": false, "error": "decode_failed", "detail": e.to_string()}),
                effect,
                baked,
            );
            return;
        }
    };

    let width = info.width as usize;
    let height = info.height as usize;
    let png_json = json!({
        "width": width,
        "height": height,
        "color_type": format!("{:?}", info.color_type),
        "bit_depth": format!("{:?}", info.bit_depth),
    });
    let bytes_per_pixel = match info.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => {
            eprintln!(
                "warning: unsupported PNG color type in texture fit image '{}'",
                path
            );
            dump(
                Some(&bytes),
                json!({"ok": false, "error": "unsupported_color_type", "png": png_json}),
                effect,
                baked,
            );
            return;
        }
    };

    let buf = &buf[..info.buffer_size()];
    let total_pixels = width * height;
    let mut pixels: Vec<[f32; 3]> = Vec::with_capacity(total_pixels);
    for i in (0..buf.len()).step_by(bytes_per_pixel) {
        let r = buf[i] as f32 / 255.0;
        let g = buf[i + 1] as f32 / 255.0;
        let b = buf[i + 2] as f32 / 255.0;
        pixels.push([
            thyllore_effect_core::flame_fit::srgb_to_linear(r),
            thyllore_effect_core::flame_fit::srgb_to_linear(g),
            thyllore_effect_core::flame_fit::srgb_to_linear(b),
        ]);
    }
    let mut max_luminance = 0.0f32;
    let mut luminance_sum = 0.0f64;
    for pixel in &pixels {
        let luminance = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        max_luminance = max_luminance.max(luminance);
        luminance_sum += luminance as f64;
    }
    let decode_json = json!({
        "max_luminance": max_luminance,
        "mean_luminance": luminance_sum / total_pixels.max(1) as f64,
    });

    let fit = match thyllore_effect_core::fit_flame_texture(&pixels, width, height, effect, baked) {
        Some(f) => f,
        None => {
            eprintln!("warning: texture fit failed for image '{}'", path);
            dump(
                Some(&bytes),
                json!({
                    "ok": false,
                    "error": "mask_empty",
                    "png": png_json,
                    "decode": decode_json,
                }),
                effect,
                baked,
            );
            return;
        }
    };

    thyllore_effect_core::apply_texture_fit(effect, baked, &fit, groups, blend, profile);
    dump(
        Some(&bytes),
        json!({
            "ok": true,
            "png": png_json,
            "decode": decode_json,
            "fit": {
                "envelope_peak": fit.envelope_peak,
                "envelope_base": fit.envelope_base,
                "envelope_tail": fit.envelope_tail,
                "radius": fit.radius,
                "radius_tip_ratio": fit.radius_tip_ratio,
                "taper_power": fit.taper_power,
                "use_blackbody": fit.use_blackbody,
                "temperature_base_k": fit.temperature_base_k,
                "temperature_tip_k": fit.temperature_tip_k,
                "noise_amplitude": fit.noise_amplitude,
                "suggested_instances": fit.suggested_instances,
            },
        }),
        effect,
        baked,
    );
}
