use std::path::Path;

use thyllore_avatar_core::vrchat::sidecar_csharp::{sidecar_csharp, sidecar_json_schema};

fn main() -> anyhow::Result<()> {
    let editor_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../unity/com.thyllore.avatar/Editor");
    std::fs::write(editor_dir.join("Sidecar.cs"), sidecar_csharp())?;
    let schema = serde_json::to_string_pretty(&sidecar_json_schema())?;
    std::fs::write(
        editor_dir.join("sidecar.schema.json"),
        format!("{schema}\n"),
    )?;
    Ok(())
}
