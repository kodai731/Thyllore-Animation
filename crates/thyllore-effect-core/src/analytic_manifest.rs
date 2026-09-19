use std::fs;
use std::path::{Path, PathBuf};

pub fn glsl_include_dir(include_dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../shaders")
        .join(include_dir)
}

pub fn glsl_source(include_dir: &str, relative_path: &str) -> String {
    let path = glsl_include_dir(include_dir).join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e))
}

pub fn glsl_int_constant(source: &str, name: &str) -> i64 {
    let prefix = format!("const int {name} = ");
    source
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .and_then(|rest| rest.trim_end_matches(';').parse().ok())
        .unwrap_or_else(|| panic!("{name} not declared in the wind GLSL"))
}

pub fn glsl_float_constant(source: &str, name: &str) -> f32 {
    let prefix = format!("const float {name} = ");
    source
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .and_then(|rest| rest.trim_end_matches(';').parse().ok())
        .unwrap_or_else(|| panic!("{name} declared as a float constant"))
}

pub fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// (name, body) per top-level GLSL function; handles multi-line signatures and struct returns.
pub fn parse_functions(source: &str) -> Vec<(String, String)> {
    const NON_TYPES: [&str; 9] = [
        "if", "for", "while", "return", "else", "switch", "const", "struct", "layout",
    ];
    let mut functions = Vec::new();
    let mut depth: i32 = 0;
    let mut current: Option<(String, String, bool)> = None;
    for line in source.lines() {
        if depth == 0 && current.is_none() {
            let trimmed = line.trim_start();
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
    let dir = glsl_include_dir(include_dir);
    let mut combined = String::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("glsl") {
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
            "function \"{}\" not found in {:?}/*.glsl (found: {:?})",
            name,
            include_dir,
            function_names
        );
    }
}

#[test]
fn strip_line_comments_keeps_line_boundaries_for_parse_functions() {
    let source = "float a() { // x\n    return 1.0;\n}\nfloat b() {\n    return 2.0;\n}";
    let stripped = strip_line_comments(source);

    assert_eq!(stripped.lines().count(), source.lines().count());
    let names: Vec<String> = parse_functions(&stripped)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["a", "b"]);
}
