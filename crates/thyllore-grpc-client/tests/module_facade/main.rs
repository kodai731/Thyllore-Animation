use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::{Item, UseTree, Visibility};

mod public_paths;

const SOURCE_ROOTS: &[&str] = &["src", "crates"];
const MEMBER_ROOTS: &[&str] = &["crates", "scratch"];
const DOMAIN_CRATES_ROOT: &str = "crates";
const DATA_LOGIC_SPLIT_MODULE_NAMES: &[&str] = &["components", "systems"];
const BUILD_OUTPUT_DIRECTORY: &str = "target";
const MODULE_ROOT_FILE_NAMES: &[&str] = &["mod.rs", "lib.rs"];
const PATH_ROOT_KEYWORDS: &[&str] = &["crate", "super"];
const WORKSPACE_LINTS_OPT_IN: &str = "[lints]\nworkspace = true";
const CHECKED_ROOT_VARIABLE: &str = "THYLLORE_MODULE_FACADE_ROOT";

struct SourceFile {
    relative_path: PathBuf,
    items: Vec<Item>,
}

enum ReexportLeaf {
    Glob,
    Name(String),
}

struct Reexport {
    first_segment: String,
    leaf: ReexportLeaf,
}

fn workspace_root() -> PathBuf {
    if let Some(checked_root) = std::env::var_os(CHECKED_ROOT_VARIABLE) {
        return PathBuf::from(checked_root);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn collect_files_named(dir: &Path, matches: &dyn Fn(&Path) -> bool, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name != BUILD_OUTPUT_DIRECTORY)
            {
                collect_files_named(&path, matches, files);
            }
        } else if matches(&path) {
            files.push(path);
        }
    }
}

fn collect_workspace_files(roots: &[&str], matches: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let workspace_root = workspace_root();
    let mut files = Vec::new();
    for root in roots {
        collect_files_named(&workspace_root.join(root), matches, &mut files);
    }
    files.sort();
    files
}

fn parse_workspace_sources() -> Vec<SourceFile> {
    let root = workspace_root();
    let is_rust_source = |path: &Path| path.extension().is_some_and(|extension| extension == "rs");

    collect_workspace_files(SOURCE_ROOTS, &is_rust_source)
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).expect("read rust source");
            let syntax = syn::parse_file(&source)
                .unwrap_or_else(|error| panic!("{} does not parse: {error}", path.display()));
            SourceFile {
                relative_path: path
                    .strip_prefix(&root)
                    .expect("under workspace root")
                    .to_path_buf(),
                items: syntax.items,
            }
        })
        .collect()
}

fn is_module_root(file: &SourceFile) -> bool {
    file.relative_path.file_name().is_some_and(|name| {
        MODULE_ROOT_FILE_NAMES
            .iter()
            .any(|root_name| name == *root_name)
    })
}

fn is_proc_macro_entry(function: &syn::ItemFn) -> bool {
    function.attrs.iter().any(|attribute| {
        attribute
            .path()
            .get_ident()
            .is_some_and(|name| name.to_string().starts_with("proc_macro"))
    })
}

fn describe_definition(item: &Item) -> Option<String> {
    let description = match item {
        Item::Use(_) | Item::ExternCrate(_) => return None,
        Item::Mod(module) if module.content.is_none() => return None,
        Item::Fn(function) if is_proc_macro_entry(function) => return None,
        Item::Mod(module) => format!("inline `mod {}`", module.ident),
        Item::Struct(definition) => format!("`struct {}`", definition.ident),
        Item::Enum(definition) => format!("`enum {}`", definition.ident),
        Item::Trait(definition) => format!("`trait {}`", definition.ident),
        Item::Type(definition) => format!("`type {}`", definition.ident),
        Item::Const(definition) => format!("`const {}`", definition.ident),
        Item::Static(definition) => format!("`static {}`", definition.ident),
        Item::Fn(definition) => format!("`fn {}`", definition.sig.ident),
        Item::Impl(_) => "`impl` block".to_string(),
        Item::Macro(_) => "macro invocation".to_string(),
        _ => "definition".to_string(),
    };
    Some(description)
}

fn collect_reexports(tree: &UseTree, first_segment: Option<&str>, reexports: &mut Vec<Reexport>) {
    let mut push_leaf = |own_name: Option<String>, leaf: ReexportLeaf| {
        if let Some(first_segment) = first_segment.map(str::to_string).or(own_name) {
            reexports.push(Reexport {
                first_segment,
                leaf,
            });
        }
    };

    match tree {
        UseTree::Path(path) => {
            let segment = path.ident.to_string();
            let is_leading_self = first_segment.is_none() && segment == "self";
            let first_segment = match first_segment {
                Some(first_segment) => Some(first_segment),
                None if is_leading_self => None,
                None => Some(segment.as_str()),
            };
            collect_reexports(&path.tree, first_segment, reexports);
        }
        UseTree::Group(group) => {
            for nested in &group.items {
                collect_reexports(nested, first_segment, reexports);
            }
        }
        UseTree::Glob(_) => push_leaf(None, ReexportLeaf::Glob),
        UseTree::Name(name) => {
            let name = name.ident.to_string();
            push_leaf(Some(name.clone()), ReexportLeaf::Name(name));
        }
        UseTree::Rename(rename) => {
            let name = rename.ident.to_string();
            push_leaf(Some(name.clone()), ReexportLeaf::Name(name));
        }
    }
}

fn collect_public_reexports(items: &[Item]) -> Vec<Reexport> {
    let mut reexports = Vec::new();
    for item in items {
        if let Item::Use(item_use) = item {
            if matches!(item_use.vis, Visibility::Public(_)) {
                collect_reexports(&item_use.tree, None, &mut reexports);
            }
        }
    }
    reexports
}

fn collect_bound_names(tree: &UseTree, names: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(path) => collect_bound_names(&path.tree, names),
        UseTree::Group(group) => {
            for nested in &group.items {
                collect_bound_names(nested, names);
            }
        }
        UseTree::Name(name) => {
            names.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) => {
            names.insert(rename.rename.to_string());
        }
        UseTree::Glob(_) => {}
    }
}

fn collect_private_scope_names(items: &[Item]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in items {
        match item {
            Item::Mod(module) => {
                names.insert(module.ident.to_string());
            }
            Item::Enum(definition) => {
                names.insert(definition.ident.to_string());
            }
            Item::Use(item_use) if !matches!(item_use.vis, Visibility::Public(_)) => {
                collect_bound_names(&item_use.tree, &mut names);
            }
            _ => {}
        }
    }
    names
}

fn collect_public_module_names(items: &[Item]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if matches!(module.vis, Visibility::Public(_)) => {
                Some(module.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

fn find_dual_paths(items: &[Item]) -> Vec<String> {
    let public_modules = collect_public_module_names(items);
    let dual_path_modules: BTreeSet<String> = collect_public_reexports(items)
        .into_iter()
        .map(|reexport| reexport.first_segment)
        .filter(|first_segment| public_modules.contains(first_segment))
        .collect();

    dual_path_modules
        .into_iter()
        .map(|module| {
            format!("R2: `pub mod {module}` is also re-exported with `pub use {module}::..`")
        })
        .collect()
}

fn find_public_data_logic_split(items: &[Item]) -> Vec<String> {
    collect_public_module_names(items)
        .into_iter()
        .filter(|module| DATA_LOGIC_SPLIT_MODULE_NAMES.contains(&module.as_str()))
        .map(|module| {
            format!("R5: `pub mod {module}` puts the data / logic split on the public path")
        })
        .collect()
}

fn find_dependency_reexports(items: &[Item]) -> Vec<String> {
    let scope_names = collect_private_scope_names(items);
    let comes_from_dependency = |reexport: &Reexport| {
        let first_segment = reexport.first_segment.as_str();
        !PATH_ROOT_KEYWORDS.contains(&first_segment) && !scope_names.contains(first_segment)
    };

    collect_public_reexports(items)
        .iter()
        .filter(|reexport| comes_from_dependency(reexport))
        .filter_map(|reexport| match &reexport.leaf {
            ReexportLeaf::Glob => Some(format!(
                "R3: glob re-export of dependency `{}`",
                reexport.first_segment
            )),
            ReexportLeaf::Name(name) if name.starts_with(char::is_lowercase) => Some(format!(
                "R4: module or function `{name}` re-exported from dependency `{}`",
                reexport.first_segment
            )),
            ReexportLeaf::Name(_) => None,
        })
        .collect()
}

fn find_violations_in_every_scope(
    items: &[Item],
    find_violations: &dyn Fn(&[Item]) -> Vec<String>,
) -> Vec<String> {
    let mut violations = find_violations(items);
    for item in items {
        if let Item::Mod(module) = item {
            if let Some((_, inline_items)) = &module.content {
                violations.extend(find_violations_in_every_scope(
                    inline_items,
                    find_violations,
                ));
            }
        }
    }
    violations
}

fn find_syntax_violations(file: &SourceFile) -> Vec<String> {
    let mut violations = Vec::new();

    if is_module_root(file) {
        violations.extend(
            file.items
                .iter()
                .filter_map(describe_definition)
                .map(|definition| format!("R1: {definition}")),
        );
    }

    violations.extend(find_violations_in_every_scope(
        &file.items,
        &find_dual_paths,
    ));
    violations.extend(find_violations_in_every_scope(
        &file.items,
        &find_dependency_reexports,
    ));

    if file.relative_path.starts_with(DOMAIN_CRATES_ROOT) {
        violations.extend(find_violations_in_every_scope(
            &file.items,
            &find_public_data_logic_split,
        ));
    }

    violations
        .into_iter()
        .map(|violation| format!("{}: {violation}", file.relative_path.display()))
        .collect()
}

#[test]
fn workspace_sources_follow_the_module_rules() {
    let files = parse_workspace_sources();

    let mut violations: Vec<String> = files.iter().flat_map(find_syntax_violations).collect();
    violations.extend(public_paths::find_public_path_violations(&files));

    assert!(
        violations.is_empty(),
        "module-structure.md violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn every_workspace_member_opts_into_the_workspace_lints() {
    let root = workspace_root();
    let is_manifest = |path: &Path| path.file_name().is_some_and(|name| name == "Cargo.toml");
    let mut manifests = collect_workspace_files(MEMBER_ROOTS, &is_manifest);
    manifests.push(root.join("Cargo.toml"));

    let manifests_without_lints: Vec<String> = manifests
        .iter()
        .filter(|manifest| {
            !fs::read_to_string(manifest)
                .expect("read manifest")
                .replace("\r\n", "\n")
                .contains(WORKSPACE_LINTS_OPT_IN)
        })
        .map(|manifest| manifest.display().to_string())
        .collect();

    assert!(
        manifests_without_lints.is_empty(),
        "manifests without `[lints] workspace = true`:\n{}",
        manifests_without_lints.join("\n")
    );
}
