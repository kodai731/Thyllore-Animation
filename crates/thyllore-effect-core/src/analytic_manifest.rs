use std::fs;
use std::path::{Path, PathBuf};

pub fn shader_include_dir(include_dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../shaders")
        .join(include_dir)
}

pub fn shader_source(include_dir: &str, relative_path: &str) -> String {
    let path = shader_include_dir(include_dir).join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e))
}

/// Value of `[public] static const <ty> <name> = <value>;` wherever it is declared (module scope or struct).
fn constant_text<'a>(source: &'a str, ty: &str, name: &str) -> Option<&'a str> {
    let prefix = format!("static const {ty} {name} = ");
    source
        .lines()
        .map(|line| line.trim().trim_start_matches("public ").trim_start())
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|rest| rest.trim_end_matches(';'))
}

pub fn int_constant(source: &str, name: &str) -> i64 {
    constant_text(source, "int", name)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("{name} declared as an int constant"))
}

pub fn float_constant(source: &str, name: &str) -> f32 {
    constant_text(source, "float", name)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("{name} declared as a float constant"))
}

pub fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// (name, body) per top-level Slang function; handles multi-line signatures and struct returns.
pub fn parse_functions(source: &str) -> Vec<(String, String)> {
    const NON_TYPES: [&str; 12] = [
        "if", "for", "while", "return", "else", "switch", "const", "struct", "layout", "module",
        "import", "static",
    ];
    let mut functions = Vec::new();
    let mut depth: i32 = 0;
    let mut current: Option<(String, String, bool)> = None;
    for line in source.lines() {
        if depth == 0 && current.is_none() {
            let trimmed = line.trim_start().trim_start_matches("public ");
            let mut words = trimmed.split_whitespace();
            if let (Some(ty), Some(rest)) = (words.next(), words.next()) {
                let is_type =
                    ty.chars().all(|c| c.is_alphanumeric() || c == '_') && !NON_TYPES.contains(&ty);
                if is_type {
                    if let Some(paren) = rest.find('(') {
                        let name = &rest[..paren];
                        if paren > 0 && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                            current = Some((name.to_string(), String::new(), false));
                        }
                    }
                }
            }
        }
        if let Some((_, body, opened)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
            if line.contains('{') {
                *opened = true;
            }
            if !*opened && line.contains(';') {
                current = None;
            }
        }
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth == 0 {
            if let Some((name, body, opened)) = current.take() {
                if opened {
                    functions.push((name, body));
                } else {
                    current = Some((name, body, opened));
                }
            }
        }
    }
    functions
}

pub fn assert_anchors_exist(include_dir: &str, expected: &[&str]) {
    let dir = shader_include_dir(include_dir);
    let mut combined = String::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("slang") {
            let content = fs::read_to_string(&path).unwrap();
            combined.push_str(&content);
            combined.push('\n');
        }
    }
    let functions = parse_functions(&combined);
    let function_names: Vec<&str> = functions.iter().map(|(name, _)| name.as_str()).collect();
    for name in expected {
        assert!(
            function_names.contains(name),
            "function \"{}\" not found in {:?}/*.slang (found: {:?})",
            name,
            include_dir,
            function_names
        );
    }
}

#[test]
fn strip_line_comments_keeps_line_boundaries_for_parse_functions() {
    let source = "float a() { // x\n    return 1.0;\n}\npublic float b() {\n    return 2.0;\n}";
    let stripped = strip_line_comments(source);

    assert_eq!(stripped.lines().count(), source.lines().count());
    let names: Vec<String> = parse_functions(&stripped)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["a", "b"]);
}
