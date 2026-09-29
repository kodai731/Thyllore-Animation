mod support;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use support::fbx_ascii::write_rig_fbx;
use support::rig_convention::rig_convention;
use support::rig_names::CONVENTIONS;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rigs")
}

#[test]
#[ignore]
fn generate_rig_fixtures() {
    let dir = fixture_dir();
    fs::create_dir_all(&dir).expect("create rig fixtures dir");
    for id in CONVENTIONS.iter().copied() {
        let convention = rig_convention(id);
        let content = write_rig_fbx(&convention);
        let path = dir.join(format!("{id}.fbx.txt"));
        fs::write(&path, &content).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    }
}

#[test]
fn rig_fixtures_match_generator() {
    let dir = fixture_dir();
    for id in CONVENTIONS.iter().copied() {
        let path = dir.join(format!("{id}.fbx.txt"));
        let stored = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "read {}: {e}\nHint: run `cargo test -p thyllore-avatar-core --test generate_rig_fixtures -- --ignored` to generate",
                path.display()
            )
        });
        let convention = rig_convention(id);
        let expected = write_rig_fbx(&convention);
        assert_eq!(
            stored, expected,
            "fixture {path:?} does not match generator output for convention {id}\n\
             Hint: run `cargo test -p thyllore-avatar-core --test generate_rig_fixtures -- --ignored` to regenerate",
        );
    }
}
