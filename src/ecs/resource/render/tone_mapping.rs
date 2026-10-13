pub use thyllore_render_core::{ToneMapOperator, ToneMapping};

crate::scene_resource!(ToneMapping);
crate::startup_resource!(ToneMapping, PostProcessing);
