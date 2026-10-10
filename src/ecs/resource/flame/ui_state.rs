#[derive(Clone, Debug)]
pub struct FlameUIState {
    pub texture_fit_path: String,
    pub texture_fit_blend: f32,
    pub texture_fit_groups: [bool; 4],
    pub texture_fit_profile: bool,
    pub texture_fit_scan: Vec<String>,
    pub texture_fit_scan_done: bool,
    pub texture_fit_browser_open: bool,
    pub texture_fit_browser_dir: String,
    pub texture_fit_browser_selected: String,
    pub texture_fit_browser_show_all: bool,
    pub texture_fit_browser_show_hidden: bool,
    pub texture_fit_path_validated: String,
    pub texture_fit_path_info: String,
    pub style_index: usize,
    pub style_scan: Vec<String>,
    pub style_scan_done: bool,
    pub style_groups: [bool; 3],
    pub style_save_name: String,
}

impl Default for FlameUIState {
    fn default() -> Self {
        Self {
            texture_fit_path: String::new(),
            texture_fit_blend: 1.0,
            texture_fit_groups: [true; 4],
            texture_fit_profile: true,
            texture_fit_scan: Vec::new(),
            texture_fit_scan_done: false,
            texture_fit_browser_open: false,
            texture_fit_browser_dir: String::new(),
            texture_fit_browser_selected: String::new(),
            texture_fit_browser_show_all: false,
            texture_fit_browser_show_hidden: false,
            texture_fit_path_validated: String::new(),
            texture_fit_path_info: String::new(),
            style_index: 0,
            style_scan: Vec::new(),
            style_scan_done: false,
            style_groups: [true; 3],
            style_save_name: String::new(),
        }
    }
}
