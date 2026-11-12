use std::path::{Path, PathBuf};

fn main() {
    let src_dir = PathBuf::from("src");

    let mut files: Vec<PathBuf> = std::fs::read_dir(&src_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file()
                && p.extension().map_or(false, |ext| ext == "rs")
                && p.file_stem().map_or(false, |s| s != "lib")
        })
        .map(|e| e.path())
        .collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_os_string());

    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=src");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("local_jobs.rs");

    let mut buf = String::new();
    for path in &files {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let abs = path.canonicalize().unwrap();
        buf.push_str(&format!("#[path = \"{}\"] mod {};\n", abs.display(), stem));
    }

    buf.push_str("fn make_local_job(name: &str, args: serde_json::Value) -> Option<anyhow::Result<Box<dyn ScratchJob>>> {\n");
    buf.push_str("    match name {\n");
    for path in &files {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        buf.push_str(&format!(
            "        \"{}\" => Some({}::make(args)),\n",
            stem, stem
        ));
    }
    buf.push_str("        _ => None,\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");

    std::fs::write(&out_path, buf).unwrap();
}
