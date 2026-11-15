#![cfg(test)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::{Item, UseTree, Visibility};

use super::SourceFile;

const CRATE_SOURCE_DIRECTORY: &str = "src";
const CRATE_ROOT_FILE_STEM: &str = "lib";
const MODULE_DIRECTORY_FILE_STEM: &str = "mod";
const UNNAMED_IMPORT: &str = "_";

type ModulePath = Vec<String>;
type Scope = BTreeMap<String, BTreeSet<Target>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    Module(ModulePath),
    Definition { module: ModulePath, name: String },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Audience {
    OtherCrates,
    SameCrate,
}

struct Import {
    path: Vec<String>,
    bound_name: Option<String>,
}

struct CrateModules<'a> {
    items_by_module: BTreeMap<ModulePath, &'a [Item]>,
    resolved_scopes: RefCell<BTreeMap<(ModulePath, Audience), Scope>>,
    scopes_in_progress: RefCell<BTreeSet<(ModulePath, Audience)>>,
}

fn is_visible_to(visibility: &Visibility, audience: Audience) -> bool {
    match audience {
        Audience::OtherCrates => matches!(visibility, Visibility::Public(_)),
        Audience::SameCrate => true,
    }
}

fn definition_name(item: &Item) -> Option<(&Visibility, String)> {
    let (visibility, ident) = match item {
        Item::Struct(definition) => (&definition.vis, &definition.ident),
        Item::Enum(definition) => (&definition.vis, &definition.ident),
        Item::Union(definition) => (&definition.vis, &definition.ident),
        Item::Trait(definition) => (&definition.vis, &definition.ident),
        Item::Type(definition) => (&definition.vis, &definition.ident),
        Item::Const(definition) => (&definition.vis, &definition.ident),
        Item::Static(definition) => (&definition.vis, &definition.ident),
        Item::Fn(definition) => (&definition.vis, &definition.sig.ident),
        _ => return None,
    };
    Some((visibility, ident.to_string()))
}

fn collect_imports(tree: &UseTree, prefix: &[String], imports: &mut Vec<Import>) {
    let mut push_named = |source: String, bound_name: String| {
        let mut path = prefix.to_vec();
        let bound_name = match (source.as_str(), prefix.last()) {
            ("self", Some(module_name)) if bound_name == "self" => module_name.clone(),
            ("self", _) => bound_name,
            _ => {
                path.push(source);
                bound_name
            }
        };
        if bound_name != UNNAMED_IMPORT {
            imports.push(Import {
                path,
                bound_name: Some(bound_name),
            });
        }
    };

    match tree {
        UseTree::Path(path) => {
            let mut nested_prefix = prefix.to_vec();
            nested_prefix.push(path.ident.to_string());
            collect_imports(&path.tree, &nested_prefix, imports);
        }
        UseTree::Group(group) => {
            for nested in &group.items {
                collect_imports(nested, prefix, imports);
            }
        }
        UseTree::Glob(_) => imports.push(Import {
            path: prefix.to_vec(),
            bound_name: None,
        }),
        UseTree::Name(name) => push_named(name.ident.to_string(), name.ident.to_string()),
        UseTree::Rename(rename) => push_named(rename.ident.to_string(), rename.rename.to_string()),
    }
}

fn split_crate_directory(relative_path: &Path) -> Option<(PathBuf, ModulePath)> {
    let components: Vec<String> = relative_path
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let source_directory_index = components
        .iter()
        .position(|component| component == CRATE_SOURCE_DIRECTORY)?;

    let crate_directory = components[..source_directory_index].iter().collect();
    let mut module_path = components[source_directory_index + 1..].to_vec();
    if module_path
        .last()
        .is_some_and(|stem| stem == CRATE_ROOT_FILE_STEM || stem == MODULE_DIRECTORY_FILE_STEM)
    {
        module_path.pop();
    }
    Some((crate_directory, module_path))
}

fn describe_module(module: &ModulePath) -> String {
    std::iter::once("crate")
        .chain(module.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join("::")
}

fn describe_target(target: &Target) -> String {
    match target {
        Target::Module(module) => format!("module `{}`", describe_module(module)),
        Target::Definition { module, name } => {
            format!("`{name}` defined in `{}`", describe_module(module))
        }
    }
}

impl<'a> CrateModules<'a> {
    fn new() -> Self {
        Self {
            items_by_module: BTreeMap::new(),
            resolved_scopes: RefCell::new(BTreeMap::new()),
            scopes_in_progress: RefCell::new(BTreeSet::new()),
        }
    }

    fn register_module(&mut self, module: ModulePath, items: &'a [Item]) {
        for item in items {
            if let Item::Mod(inline_module) = item {
                if let Some((_, inline_items)) = &inline_module.content {
                    let mut inline_path = module.clone();
                    inline_path.push(inline_module.ident.to_string());
                    self.register_module(inline_path, inline_items);
                }
            }
        }
        self.items_by_module.insert(module, items);
    }

    fn items(&self, module: &ModulePath) -> &'a [Item] {
        self.items_by_module.get(module).copied().unwrap_or(&[])
    }

    fn collect_declared_names(&self, module: &ModulePath, audience: Audience) -> Scope {
        let mut scope = Scope::new();
        for item in self.items(module) {
            let (visibility, name, target) = match item {
                Item::Mod(child) => {
                    let mut child_path = module.clone();
                    child_path.push(child.ident.to_string());
                    (
                        &child.vis,
                        child.ident.to_string(),
                        Target::Module(child_path),
                    )
                }
                other => match definition_name(other) {
                    Some((visibility, name)) => {
                        let definition = Target::Definition {
                            module: module.clone(),
                            name: name.clone(),
                        };
                        (visibility, name, definition)
                    }
                    None => continue,
                },
            };
            if is_visible_to(visibility, audience) {
                scope.entry(name).or_default().insert(target);
            }
        }
        scope
    }

    fn enter_segment(&self, target: &Target, segment: &str) -> BTreeSet<Target> {
        let Target::Module(module) = target else {
            return BTreeSet::new();
        };
        if segment == "super" {
            return module
                .split_last()
                .map(|(_, parent)| Target::Module(parent.to_vec()))
                .into_iter()
                .collect();
        }
        self.resolve_scope(module, Audience::SameCrate)
            .remove(segment)
            .unwrap_or_default()
    }

    fn resolve_path(&self, from: &ModulePath, path: &[String]) -> BTreeSet<Target> {
        let Some((first_segment, later_segments)) = path.split_first() else {
            return BTreeSet::new();
        };
        let here = Target::Module(from.clone());
        let mut targets = match first_segment.as_str() {
            "crate" => BTreeSet::from([Target::Module(ModulePath::new())]),
            "self" => BTreeSet::from([here]),
            "super" => self.enter_segment(&here, "super"),
            name => self
                .collect_declared_names(from, Audience::SameCrate)
                .remove(name)
                .unwrap_or_default(),
        };

        for segment in later_segments {
            targets = targets
                .iter()
                .flat_map(|target| self.enter_segment(target, segment))
                .collect();
        }
        targets
    }

    fn bind_import(
        &self,
        from: &ModulePath,
        import: &Import,
        audience: Audience,
        scope: &mut Scope,
    ) {
        let targets = self.resolve_path(from, &import.path);
        match &import.bound_name {
            Some(bound_name) if !targets.is_empty() => {
                scope.entry(bound_name.clone()).or_default().extend(targets);
            }
            Some(_) => {}
            None => {
                for target in targets {
                    if let Target::Module(glob_source) = target {
                        for (name, glob_targets) in self.resolve_scope(&glob_source, audience) {
                            scope.entry(name).or_default().extend(glob_targets);
                        }
                    }
                }
            }
        }
    }

    fn resolve_scope(&self, module: &ModulePath, audience: Audience) -> Scope {
        let key = (module.clone(), audience);
        if let Some(scope) = self.resolved_scopes.borrow().get(&key) {
            return scope.clone();
        }
        if !self.scopes_in_progress.borrow_mut().insert(key.clone()) {
            return Scope::new();
        }

        let mut scope = self.collect_declared_names(module, audience);
        for item in self.items(module) {
            if let Item::Use(item_use) = item {
                if is_visible_to(&item_use.vis, audience) {
                    let mut imports = Vec::new();
                    collect_imports(&item_use.tree, &[], &mut imports);
                    for import in &imports {
                        self.bind_import(module, import, audience, &mut scope);
                    }
                }
            }
        }

        self.scopes_in_progress.borrow_mut().remove(&key);
        self.resolved_scopes.borrow_mut().insert(key, scope.clone());
        scope
    }

    fn collect_public_paths(
        &self,
        module: &ModulePath,
        public_prefix: &str,
        entered_modules: &mut BTreeSet<ModulePath>,
        public_paths: &mut BTreeMap<Target, BTreeSet<String>>,
    ) {
        for (name, targets) in self.resolve_scope(module, Audience::OtherCrates) {
            let public_path = format!("{public_prefix}::{name}");
            for target in targets {
                public_paths
                    .entry(target.clone())
                    .or_default()
                    .insert(public_path.clone());
                if let Target::Module(child) = target {
                    if entered_modules.insert(child.clone()) {
                        self.collect_public_paths(
                            &child,
                            &public_path,
                            entered_modules,
                            public_paths,
                        );
                    }
                }
            }
        }
    }

    fn find_several_public_paths(&self) -> Vec<String> {
        let mut public_paths = BTreeMap::new();
        let root = ModulePath::new();
        self.collect_public_paths(
            &root,
            "crate",
            &mut BTreeSet::from([root.clone()]),
            &mut public_paths,
        );

        public_paths
            .iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|(target, paths)| {
                let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
                format!(
                    "R6: {} is public as {}",
                    describe_target(target),
                    paths.join(" and ")
                )
            })
            .collect()
    }
}

pub(crate) fn find_definitions_with_several_public_paths(files: &[SourceFile]) -> Vec<String> {
    let mut crates: BTreeMap<PathBuf, CrateModules> = BTreeMap::new();
    for file in files {
        if let Some((crate_directory, module_path)) = split_crate_directory(&file.relative_path) {
            crates
                .entry(crate_directory)
                .or_insert_with(CrateModules::new)
                .register_module(module_path, &file.items);
        }
    }

    crates
        .iter()
        .flat_map(|(crate_directory, modules)| {
            modules
                .find_several_public_paths()
                .into_iter()
                .map(move |violation| format!("{}: {violation}", crate_directory.display()))
        })
        .collect()
}

fn parse_source_file(relative_path: &str, source: &str) -> SourceFile {
    SourceFile {
        relative_path: PathBuf::from(relative_path),
        items: syn::parse_file(source).expect("fixture parses").items,
    }
}

#[test]
fn a_reexport_from_another_module_is_a_second_public_path() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "pub mod resource; pub mod systems;",
        ),
        parse_source_file(
            "crates/demo/src/resource/mod.rs",
            "mod batch; pub use batch::Edit;",
        ),
        parse_source_file("crates/demo/src/resource/batch.rs", "pub enum Edit {}"),
        parse_source_file(
            "crates/demo/src/systems/mod.rs",
            "pub use crate::resource::Edit;",
        ),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "crates/demo: R6: `Edit` defined in `crate::resource::batch` is public as \
          crate::resource::Edit and crate::systems::Edit"
        ]
    );
}

#[test]
fn a_reexport_of_an_item_in_a_glob_lifted_module_is_a_second_public_path() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod timeline; pub use timeline::*; pub mod dispatch;",
        ),
        parse_source_file("crates/demo/src/timeline/mod.rs", "pub mod keyframe_edit;"),
        parse_source_file(
            "crates/demo/src/timeline/keyframe_edit.rs",
            "pub enum End {}",
        ),
        parse_source_file(
            "crates/demo/src/dispatch.rs",
            "pub use crate::timeline::keyframe_edit::End;",
        ),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "crates/demo: R6: `End` defined in `crate::timeline::keyframe_edit` is public as \
          crate::dispatch::End and crate::keyframe_edit::End"
        ]
    );
}

#[test]
fn a_facade_reexport_of_a_private_child_is_one_public_path() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod shape; pub use shape::{area as circle_area, Circle};",
        ),
        parse_source_file(
            "crates/demo/src/shape.rs",
            "pub struct Circle; pub fn area() {}",
        ),
    ];

    assert!(find_definitions_with_several_public_paths(&files).is_empty());
}
