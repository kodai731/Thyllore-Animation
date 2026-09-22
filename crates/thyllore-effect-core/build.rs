use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use thyllore_shader_manifest::slang_root;

fn generate_cpp(slang_root: &Path, shader_root: &Path, out_dir: &Path, filename: &str) -> PathBuf {
    let stem = filename.strip_suffix(".slang").unwrap_or(filename);
    let generated = out_dir.join(format!("{}.cpp", stem));
    let status = Command::new(slang_root.join("bin/slangc"))
        .arg(shader_root.join(format!("cpu/{}", filename)))
        .arg("-I")
        .arg(shader_root.join("include"))
        .arg("-I")
        .arg(&shader_root)
        .args(["-target", "cpp", "-o"])
        .arg(&generated)
        .status()
        .expect("slangc runs (SLANG_ROOT/bin/slangc, default ~/.local/slang)");
    assert!(status.success(), "slangc failed on cpu/{}", filename);
    generated
}

fn extract_export_signatures(content: &str) -> Vec<String> {
    let mut signatures = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for i in 0..lines.len() {
        if lines[i].trim() == "SLANG_PRELUDE_EXPORT" && i + 1 < lines.len() {
            let sig = lines[i + 1].trim().to_string();
            if !sig.starts_with("static ") {
                signatures.push(sig);
            }
        }
    }
    signatures
}

fn parse_kernel_context_layout(content: &str) -> KernelLayout {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_kernel = false;
    let mut brace_depth = 0;
    let mut fields = String::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("struct KernelContext_0") {
            in_kernel = true;
            continue;
        }
        if in_kernel {
            brace_depth += trimmed.matches('{').count();
            brace_depth -= trimmed.matches('}').count();
            if brace_depth <= 0 {
                break;
            }
            fields.push_str(trimmed);
            fields.push('\n');
        }
    }

    if fields.contains("GlobalParams") {
        KernelLayout::Indirect
    } else {
        KernelLayout::Direct
    }
}

fn parse_global_params(content: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_struct = false;
    let mut brace_depth = 0;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("struct GlobalParams_0") {
            in_struct = true;
            continue;
        }
        if in_struct {
            brace_depth += trimmed.matches('{').count();
            brace_depth -= trimmed.matches('}').count();
            if brace_depth <= 0 {
                break;
            }
            let field = trimmed.strip_suffix(';').unwrap_or(trimmed);
            let parts: Vec<&str> = field.split_whitespace().collect();
            if parts.len() >= 2 {
                let type_name = parts[0];
                let var_name = parts[1].strip_suffix('*').unwrap_or(parts[1]);
                let base = var_name.strip_suffix("_0").unwrap_or(var_name);
                return Some((base.to_string(), type_name.to_string()));
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
enum KernelLayout {
    Direct,
    Indirect,
}

fn used_vector_types(signatures: &[String]) -> (bool, bool, bool) {
    let text = signatures.join(" ");
    let has_v2 = text.contains("Vector<float, 2>");
    let has_v3 = text.contains("Vector<float, 3>");
    let has_v4 = text.contains("Vector<float, 4>");
    (has_v2, has_v3, has_v4)
}

const KERNEL_CONTEXT_TYPE: &str = "*const KernelContext";

fn cpp_sig_to_rust(sig: &str) -> String {
    let sig = sig.trim().trim_end_matches(';');
    let paren_start = sig.find('(').unwrap();
    let (return_type, fn_name) =
        split_trailing_identifier(&sig[..paren_start]).expect("export signature has a name");
    let params_str = &sig[paren_start + 1..sig.len() - 1];

    let rust_params: Vec<String> = split_params(params_str)
        .iter()
        .filter_map(|param| split_trailing_identifier(param))
        .map(|(cpp_type, cpp_name)| rust_param(cpp_type, cpp_name))
        .collect();
    let rust_params = rust_params.join(", ");

    let rust_return = convert_type(return_type);
    if rust_return == "()" {
        format!("fn {}({});", fn_name, rust_params)
    } else {
        format!("fn {}({}) -> {};", fn_name, rust_params, rust_return)
    }
}

fn split_trailing_identifier(text: &str) -> Option<(&str, &str)> {
    let text = text.trim();
    let name_start = text.rfind(|c: char| !c.is_alphanumeric() && c != '_')? + 1;
    let (before_name, name) = (text[..name_start].trim(), &text[name_start..]);
    if before_name.is_empty() || name.is_empty() {
        return None;
    }
    Some((before_name, name))
}

fn rust_param(cpp_type: &str, cpp_name: &str) -> String {
    let rust_type = convert_type(cpp_type);
    if rust_type == KERNEL_CONTEXT_TYPE {
        return format!("ctx: {}", rust_type);
    }
    format!(
        "{}: {}",
        snake_case(strip_trailing_digits(cpp_name)),
        rust_type
    )
}

fn split_params(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        result.push(&s[start..]);
    }
    result
}

fn convert_type(cpp_type: &str) -> String {
    let t = cpp_type.trim();
    if let Some(pointee) = t.strip_suffix('*') {
        let pointee = pointee.trim();
        if pointee.starts_with("KernelContext") {
            return KERNEL_CONTEXT_TYPE.to_string();
        }
        return format!("*mut {}", convert_type(pointee));
    }
    match t {
        "float" => "f32".to_string(),
        "int32_t" => "i32".to_string(),
        "bool" => "bool".to_string(),
        "void" => "()".to_string(),
        _ if t.starts_with("Vector<float, 2>") => "Float2".to_string(),
        _ if t.starts_with("Vector<float, 3>") => "Float3".to_string(),
        _ if t.starts_with("Vector<float, 4>") => "Float4".to_string(),
        _ => t.to_string(),
    }
}

fn strip_trailing_digits(s: &str) -> &str {
    if let Some(pos) = s.rfind('_') {
        let suffix = &s[pos + 1..];
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            return &s[..pos];
        }
    }
    s
}

fn snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;
    for c in name.chars() {
        if c.is_uppercase() && !result.is_empty() && !prev_upper {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
        prev_upper = c.is_uppercase();
    }
    result
}

fn generate_bindings(cpp_path: &Path, out_dir: &Path, stem: &str, ubo_rust_type: &str) {
    let content = fs::read_to_string(cpp_path).expect("read generated C++");
    let signatures = extract_export_signatures(&content);
    let layout = parse_kernel_context_layout(&content);
    let (has_v2, has_v3, has_v4) = used_vector_types(&signatures);

    let mut out = String::new();
    out.push_str("// Auto-generated by build.rs from ");
    out.push_str(stem);
    out.push_str(".cpp — do not edit.\n\n");

    match layout {
        KernelLayout::Indirect => {
            if let Some((ubo_field, _)) = parse_global_params(&content) {
                out.push_str("#[repr(C)]\n");
                out.push_str("struct GlobalParams {\n");
                out.push_str(&format!("    {}: *const {},\n", ubo_field, ubo_rust_type));
                out.push_str("}\n\n");
            }
            out.push_str("#[repr(C)]\n");
            out.push_str("struct KernelContext {\n");
            out.push_str("    global_params: *const GlobalParams,\n");
            out.push_str("}\n\n");
        }
        KernelLayout::Direct => {
            out.push_str("#[repr(C)]\n");
            out.push_str("struct KernelContext {\n");
            out.push_str(&format!("    {}: {},\n", "ubo", ubo_rust_type));
            out.push_str("}\n\n");
        }
    }

    if has_v2 || has_v3 || has_v4 {
        if has_v2 {
            out.push_str("#[repr(C)]\n");
            out.push_str("struct Float2 {\n    x: f32,\n    y: f32,\n}\n\n");
        }
        if has_v3 {
            out.push_str("#[repr(C)]\n");
            out.push_str("struct Float3 {\n    x: f32,\n    y: f32,\n    z: f32,\n}\n\n");
        }
        if has_v4 {
            out.push_str("#[repr(C)]\n");
            out.push_str(
                "struct Float4 {\n    x: f32,\n    y: f32,\n    z: f32,\n    w: f32,\n}\n\n",
            );
        }
    }

    out.push_str("extern \"C\" {\n");
    for sig in &signatures {
        let rust_decl = cpp_sig_to_rust(sig);
        out.push_str("    ");
        out.push_str(&rust_decl);
        out.push('\n');
    }
    out.push_str("}\n");

    let output_path = out_dir.join(format!("{}_bindings.rs", stem));
    fs::write(&output_path, &out).expect("write bindings");
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let shader_root = manifest_dir.join("../../shaders");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let slang_root = slang_root();

    println!("cargo:rerun-if-env-changed=SLANG_ROOT");
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("cpu").display()
    );

    let generated = generate_cpp(&slang_root, &shader_root, &out_dir, "exports.slang");
    let wind_generated = generate_cpp(&slang_root, &shader_root, &out_dir, "wind_exports.slang");
    let water_generated = generate_cpp(&slang_root, &shader_root, &out_dir, "water_exports.slang");
    let flame_generated = generate_cpp(&slang_root, &shader_root, &out_dir, "flame_exports.slang");

    generate_bindings(
        &wind_generated,
        &out_dir,
        "wind_exports",
        "crate::wind::WindUBO",
    );
    generate_bindings(
        &water_generated,
        &out_dir,
        "water_exports",
        "crate::water::WaterUBO",
    );
    generate_bindings(
        &flame_generated,
        &out_dir,
        "flame_exports",
        "crate::flame::FlameUBO",
    );

    for stem in ["wind_exports", "water_exports", "flame_exports"] {
        let bindings = fs::read_to_string(out_dir.join(format!("{}_bindings.rs", stem)))
            .expect("read bindings");
        assert!(
            !bindings.contains("Vector<"),
            "{} contains C++ Vector<> types — type conversion failed",
            stem
        );
        assert!(
            !bindings.contains("int32_t"),
            "{} contains C++ int32_t — type conversion failed",
            stem
        );
        assert!(
            !bindings.contains("KernelContext_0"),
            "{} contains C++ KernelContext_0 — type conversion failed",
            stem
        );
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-ffp-contract=off")
        .include(slang_root.join("include"))
        .file(generated)
        .file(wind_generated)
        .file(water_generated)
        .file(flame_generated)
        .compile("thyllore_shader_exports");
}
