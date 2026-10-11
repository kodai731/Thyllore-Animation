use imgui::{Context, FontConfig, FontSource, StbTrueTypeFontData};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::path::Path;
use winit::dpi::LogicalSize;
use winit::event_loop::{EventLoop, EventLoopBuilder};

use crate::hooks::external_command::{ExternalCommand, ExternalCommandSender};
use crate::platform::ui::theme::UiFonts;
use winit::window::{Window, WindowBuilder};

use super::clipboard;

const INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
const INTER_SEMI_BOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
const MPLUS_1P_REGULAR: &[u8] = include_bytes!("../../assets/fonts/mplus-1p-regular.ttf");
const LUCIDE: &[u8] = include_bytes!("../../assets/fonts/lucide.ttf");

// Sizes are winit logical pixels; winit applies the display scale factor.
const BODY_FONT_SIZE: f32 = 13.0;
const HEADING_FONT_SIZE: f32 = 15.0;
const FONT_OVERSAMPLE: i8 = 4;

// Inter maps 745 of its own glyphs into the private-use area that the Lucide icons occupy.
const LUCIDE_GLYPH_RANGE: (u32, u32) = (0xE038, 0xE786);

pub struct System {
    pub event_loop: EventLoop<ExternalCommand>,
    pub window: Window,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub ui_fonts: UiFonts,
}

pub fn init(title: &str, take_focus: bool) -> System {
    let title = match Path::new(&title).file_name() {
        Some(file_name) => file_name.to_str().unwrap_or(title),
        None => title,
    };
    let event_loop = EventLoopBuilder::<ExternalCommand>::with_user_event()
        .build()
        .expect("Failed to create EventLoop");

    #[allow(unused_mut)]
    let mut builder = WindowBuilder::new()
        .with_title(title)
        .with_active(take_focus)
        .with_inner_size(LogicalSize::new(2560, 1440));
    // winit 0.29 ignores `with_active` on X11; an override-redirect window is
    // unmanaged by the WM and therefore can never steal input focus. It is also
    // always-on-top, so park it outside the visible screen — batch/MCP runs must
    // never cover the desktop or intercept the user's clicks.
    #[cfg(target_os = "linux")]
    if !take_focus {
        use winit::dpi::PhysicalPosition;
        use winit::platform::x11::WindowBuilderExtX11;
        builder = builder
            .with_override_redirect(true)
            .with_position(PhysicalPosition::new(10000, 10000));
    }
    let window = builder.build(&event_loop).expect("Failed to create window");

    let mut imgui = Context::create();
    super::ui::theme::style::apply_thyllore_style(imgui.style_mut());
    imgui
        .set_ini_filename(None::<&Path>)
        .expect("no ini filename to validate");

    let io = imgui.io_mut();
    io.set_config_flags(io.config_flags() | imgui::ConfigFlags::DOCKING_ENABLE);
    io.set_backend_flags(
        io.backend_flags()
            | imgui::BackendFlags::RENDERER_HAS_VTX_OFFSET
            | imgui::BackendFlags::RENDERER_HAS_TEXTURES,
    );

    if let Some(backend) = clipboard::init() {
        imgui.set_clipboard_backend(backend);
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let dpi_mode = if let Ok(factor) = std::env::var("IMGUI_EXAMPLE_FORCE_DPI_FACTOR") {
            // Allow forcing of HiDPI factor for debugging purposes
            match factor.parse::<f64>() {
                Ok(f) => HiDpiMode::Locked(f),
                Err(e) => {
                    log_warn!("Invalid scaling factor '{}': {}, using default", factor, e);
                    HiDpiMode::Default
                }
            }
        } else {
            HiDpiMode::Default
        };

        platform.attach_window(imgui.io_mut(), &window, dpi_mode);
    }

    let atlas = imgui.font_atlas();
    let body = atlas.add_font(&[
        text_font_source(INTER_REGULAR, BODY_FONT_SIZE),
        ttf_source(MPLUS_1P_REGULAR, BODY_FONT_SIZE),
        ttf_source(LUCIDE, BODY_FONT_SIZE),
    ]);
    let heading = atlas.add_font(&[
        text_font_source(INTER_SEMI_BOLD, HEADING_FONT_SIZE),
        ttf_source(MPLUS_1P_REGULAR, HEADING_FONT_SIZE),
        ttf_source(LUCIDE, HEADING_FONT_SIZE),
    ]);
    let ui_fonts = UiFonts { body, heading };

    System {
        event_loop,
        window,
        imgui,
        platform,
        ui_fonts,
    }
}

fn text_font_source(data: &[u8], size_pixels: f32) -> FontSource<'static> {
    font_source(
        data,
        size_pixels,
        FontConfig::new().glyph_exclude_ranges(&[LUCIDE_GLYPH_RANGE]),
    )
}

fn ttf_source(data: &[u8], size_pixels: f32) -> FontSource<'static> {
    font_source(data, size_pixels, FontConfig::new())
}

fn font_source(data: &[u8], size_pixels: f32, config: FontConfig) -> FontSource<'static> {
    let data =
        StbTrueTypeFontData::from_slice(data).expect("embedded font is a valid TrueType file");
    let config = config
        .oversample_h(FONT_OVERSAMPLE)
        .oversample_v(FONT_OVERSAMPLE);
    FontSource::stb_truetype_with_size(data, size_pixels).with_config(config)
}
