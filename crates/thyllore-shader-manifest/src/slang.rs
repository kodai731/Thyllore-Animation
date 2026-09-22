use std::env;
use std::path::{Path, PathBuf};

pub fn slang_root() -> PathBuf {
    if let Ok(root) = env::var("SLANG_ROOT") {
        return PathBuf::from(root);
    }
    let home = env::var("HOME").expect("HOME set");
    Path::new(&home).join(".local/slang")
}
