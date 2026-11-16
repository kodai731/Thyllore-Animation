#![cfg(test)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use syn::{Item, UseTree, Visibility};

use super::SourceFile;

const CRATE_SOURCE_DIRECTORY: &str = "src";
const CRATE_ROOT_FILE_STEM: &str = "lib";
const MODULE_DIRECTORY_FILE_STEM: &str = "mod";
const UNNAMED_IMPORT: &str = "_";
const GENERATED_ITEMS_NAME: &str = "*";
const SOURCE_INCLUSION_MACRO: &str = "include";

type ModulePath = Vec<String>;
type Scope = BTreeMap<String, BTreeSet<Target>>;
type ScopeKey = (ModulePath, Audience);
type ScopeCache = BTreeMap<ScopeKey, Rc<Scope>>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    Module(ModulePath),
    Definition { module: ModulePath, name: String },
    GeneratedItems { module: ModulePath },
    Dependency { path: Vec<String> },
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

struct ModuleImport<'a> {
    visibility: &'a Visibility,
    import: Import,
}

struct CrateModules<'a> {
    items_by_module: BTreeMap<ModulePath, &'a [Item]>,
    imports_by_module: BTreeMap<ModulePath, Vec<ModuleImport<'a>>>,
    declared_names_by_module: BTreeMap<ModulePath, Scope>,
    settled_scopes: RefCell<ScopeCache>,
    provisional_scopes: RefCell<ScopeCache>,
    scopes_in_progress: RefCell<BTreeSet<ScopeKey>>,
    reentered_scopes: RefCell<BTreeSet<ScopeKey>>,
    imported_names_in_progress: RefCell<BTreeSet<(ModulePath, String)>>,
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

fn includes_generated_source(item: &Item) -> bool {
    matches!(item, Item::Macro(invocation) if invocation.mac.path.is_ident(SOURCE_INCLUSION_MACRO))
}

fn name_generated_items(generated_items: BTreeSet<Target>, name: &str) -> BTreeSet<Target> {
    generated_items
        .into_iter()
        .filter_map(|target| match target {
            Target::GeneratedItems { module } => Some(Target::Definition {
                module,
                name: name.to_string(),
            }),
            Target::Module(_) | Target::Definition { .. } | Target::Dependency { .. } => None,
        })
        .collect()
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
        Target::GeneratedItems { module } => {
            format!("glob of generated items in `{}`", describe_module(module))
        }
        Target::Dependency { path } => format!("dependency item `{}`", path.join("::")),
    }
}

fn add_generated_item_paths(public_paths: &mut BTreeMap<Target, BTreeSet<String>>) {
    let glob_paths_by_module: BTreeMap<ModulePath, BTreeSet<String>> = public_paths
        .iter()
        .filter_map(|(target, paths)| match target {
            Target::GeneratedItems { module } => Some((module.clone(), paths.clone())),
            _ => None,
        })
        .collect();

    for (target, paths) in public_paths.iter_mut() {
        let Target::Definition { module, name } = target else {
            continue;
        };
        let Some(glob_paths) = glob_paths_by_module.get(module) else {
            continue;
        };
        paths.extend(glob_paths.iter().map(|glob_path| {
            format!("{}{name}", glob_path.trim_end_matches(GENERATED_ITEMS_NAME))
        }));
    }
}

fn names_module_or_function(dependency_path: &[String]) -> bool {
    dependency_path
        .last()
        .is_some_and(|leaf| leaf.starts_with(char::is_lowercase))
}

fn describe_indirect_dependency_reexport(
    module: &ModulePath,
    import: &Import,
    dependency_path: &[String],
) -> Option<String> {
    let written_path = import.path.join("::");
    let dependency_path_text = dependency_path.join("::");
    let module_text = describe_module(module);

    match &import.bound_name {
        None => Some(format!(
            "R3: `pub use {written_path}::*` in `{module_text}` is a glob re-export of dependency \
             module `{dependency_path_text}`"
        )),
        Some(_) if names_module_or_function(dependency_path) => Some(format!(
            "R4: `pub use {written_path}` in `{module_text}` re-exports module or function \
             `{dependency_path_text}` of a dependency"
        )),
        Some(_) => None,
    }
}

fn collect_module_imports(items: &[Item]) -> Vec<ModuleImport<'_>> {
    let mut module_imports = Vec::new();
    for item in items {
        let Item::Use(item_use) = item else {
            continue;
        };
        let mut imports = Vec::new();
        collect_imports(&item_use.tree, &[], &mut imports);
        module_imports.extend(imports.into_iter().map(|import| ModuleImport {
            visibility: &item_use.vis,
            import,
        }));
    }
    module_imports
}

impl<'a> CrateModules<'a> {
    fn new() -> Self {
        Self {
            items_by_module: BTreeMap::new(),
            imports_by_module: BTreeMap::new(),
            declared_names_by_module: BTreeMap::new(),
            settled_scopes: RefCell::new(ScopeCache::new()),
            provisional_scopes: RefCell::new(ScopeCache::new()),
            scopes_in_progress: RefCell::new(BTreeSet::new()),
            reentered_scopes: RefCell::new(BTreeSet::new()),
            imported_names_in_progress: RefCell::new(BTreeSet::new()),
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
        self.items_by_module.insert(module.clone(), items);
        self.imports_by_module
            .insert(module.clone(), collect_module_imports(items));
        let declared_names = self.collect_declared_names(&module, Audience::SameCrate);
        self.declared_names_by_module.insert(module, declared_names);
    }

    fn items(&self, module: &ModulePath) -> &'a [Item] {
        self.items_by_module.get(module).copied().unwrap_or(&[])
    }

    fn imports(&self, module: &ModulePath) -> &[ModuleImport<'a>] {
        self.imports_by_module
            .get(module)
            .map_or(&[], Vec::as_slice)
    }

    fn find_declared_name(&self, module: &ModulePath, name: &str) -> Option<BTreeSet<Target>> {
        self.declared_names_by_module
            .get(module)?
            .get(name)
            .cloned()
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

        if self.items(module).iter().any(includes_generated_source) {
            scope
                .entry(GENERATED_ITEMS_NAME.to_string())
                .or_default()
                .insert(Target::GeneratedItems {
                    module: module.clone(),
                });
        }
        scope
    }

    fn enter_segment(&self, target: &Target, segment: &str) -> BTreeSet<Target> {
        let module = match target {
            Target::Module(module) => module,
            Target::Definition { .. } | Target::GeneratedItems { .. } => return BTreeSet::new(),
            Target::Dependency { path } => {
                let mut longer_path = path.clone();
                longer_path.push(segment.to_string());
                return BTreeSet::from([Target::Dependency { path: longer_path }]);
            }
        };
        if segment == "super" {
            return module
                .split_last()
                .map(|(_, parent)| Target::Module(parent.to_vec()))
                .into_iter()
                .collect();
        }

        let scope = self.resolve_scope(module, Audience::SameCrate);
        if let Some(targets) = scope.get(segment) {
            return targets.clone();
        }
        let generated_items = scope.get(GENERATED_ITEMS_NAME).cloned().unwrap_or_default();
        name_generated_items(generated_items, segment)
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
            name => match self.resolve_name_in_module(from, name) {
                Some(local_targets) => local_targets,
                None => {
                    return BTreeSet::from([Target::Dependency {
                        path: path.to_vec(),
                    }])
                }
            },
        };

        for segment in later_segments {
            targets = targets
                .iter()
                .flat_map(|target| self.enter_segment(target, segment))
                .collect();
        }
        targets
    }

    fn resolve_name_in_module(&self, module: &ModulePath, name: &str) -> Option<BTreeSet<Target>> {
        if let Some(declared) = self.find_declared_name(module, name) {
            return Some(declared);
        }

        let key = (module.clone(), name.to_string());
        if !self
            .imported_names_in_progress
            .borrow_mut()
            .insert(key.clone())
        {
            return None;
        }
        let targets = self.resolve_imported_name(module, name);
        self.imported_names_in_progress.borrow_mut().remove(&key);
        targets
    }

    fn resolve_imported_name(&self, module: &ModulePath, name: &str) -> Option<BTreeSet<Target>> {
        let imports = self.imports(module).iter().map(|declared| &declared.import);

        let importing_paths: Vec<&Vec<String>> = imports
            .clone()
            .filter(|import| import.bound_name.as_deref() == Some(name))
            .map(|import| &import.path)
            .collect();
        if !importing_paths.is_empty() {
            let named_targets = importing_paths
                .into_iter()
                .flat_map(|path| self.resolve_path(module, path))
                .collect();
            return Some(named_targets);
        }

        let glob_targets: BTreeSet<Target> = imports
            .filter(|import| import.bound_name.is_none())
            .flat_map(|import| self.resolve_path(module, &import.path))
            .flat_map(|glob_source| self.resolve_name_lifted_from(&glob_source, name))
            .collect();
        (!glob_targets.is_empty()).then_some(glob_targets)
    }

    fn resolve_name_lifted_from(&self, glob_source: &Target, name: &str) -> BTreeSet<Target> {
        let Target::Module(source_module) = glob_source else {
            return BTreeSet::new();
        };
        if let Some(declared) = self.find_declared_name(source_module, name) {
            return declared;
        }
        self.resolve_scope(source_module, Audience::SameCrate)
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    fn bind_import(
        &self,
        from: &ModulePath,
        import: &Import,
        audience: Audience,
        scope: &mut Scope,
    ) {
        let targets = self.resolve_path(from, &import.path);
        let Some(bound_name) = &import.bound_name else {
            self.bind_glob_import(targets, audience, scope);
            return;
        };
        if !targets.is_empty() {
            scope.entry(bound_name.clone()).or_default().extend(targets);
        }
    }

    fn bind_glob_import(
        &self,
        glob_sources: BTreeSet<Target>,
        audience: Audience,
        scope: &mut Scope,
    ) {
        for glob_source in glob_sources {
            let Target::Module(source_module) = glob_source else {
                continue;
            };
            for (name, lifted_targets) in self.resolve_scope(&source_module, audience).iter() {
                scope
                    .entry(name.clone())
                    .or_default()
                    .extend(lifted_targets.iter().cloned());
            }
        }
    }

    fn collect_scope(&self, module: &ModulePath, audience: Audience) -> Scope {
        let mut scope = self.collect_declared_names(module, audience);
        for declared in self.imports(module) {
            if is_visible_to(declared.visibility, audience) {
                self.bind_import(module, &declared.import, audience, &mut scope);
            }
        }
        scope
    }

    fn find_cached_scope(&self, key: &ScopeKey) -> Option<Rc<Scope>> {
        if let Some(settled) = self.settled_scopes.borrow().get(key) {
            return Some(settled.clone());
        }
        self.provisional_scopes.borrow().get(key).cloned()
    }

    fn cache_scope(&self, key: ScopeKey, scope: Rc<Scope>) {
        let misses_names_of_an_unfinished_scope = !self.reentered_scopes.borrow().is_empty();
        if misses_names_of_an_unfinished_scope {
            self.provisional_scopes.borrow_mut().insert(key, scope);
            return;
        }
        self.settled_scopes.borrow_mut().insert(key, scope);
        self.provisional_scopes.borrow_mut().clear();
    }

    fn resolve_scope(&self, module: &ModulePath, audience: Audience) -> Rc<Scope> {
        let key = (module.clone(), audience);
        if let Some(scope) = self.find_cached_scope(&key) {
            return scope;
        }
        if !self.scopes_in_progress.borrow_mut().insert(key.clone()) {
            self.reentered_scopes.borrow_mut().insert(key);
            return Rc::new(Scope::new());
        }

        let scope = Rc::new(self.collect_scope(module, audience));

        self.scopes_in_progress.borrow_mut().remove(&key);
        self.reentered_scopes.borrow_mut().remove(&key);
        self.cache_scope(key, scope.clone());
        scope
    }

    fn collect_public_paths(
        &self,
        module: &ModulePath,
        public_prefix: &str,
        entered_modules: &mut BTreeSet<ModulePath>,
        public_paths: &mut BTreeMap<Target, BTreeSet<String>>,
    ) {
        let scope = self.resolve_scope(module, Audience::OtherCrates);
        for (name, targets) in scope.iter() {
            let public_path = format!("{public_prefix}::{name}");
            for target in targets {
                public_paths
                    .entry(target.clone())
                    .or_default()
                    .insert(public_path.clone());
                let Target::Module(child) = target else {
                    continue;
                };
                if entered_modules.insert(child.clone()) {
                    self.collect_public_paths(child, &public_path, entered_modules, public_paths);
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
        add_generated_item_paths(&mut public_paths);

        public_paths
            .iter()
            .filter(|(target, _)| !matches!(target, Target::Dependency { .. }))
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

    fn find_indirect_dependency_reexports_in(&self, module: &ModulePath) -> Vec<String> {
        self.imports(module)
            .iter()
            .filter(|declared| matches!(declared.visibility, Visibility::Public(_)))
            .map(|declared| &declared.import)
            .flat_map(|import| {
                self.resolve_path(module, &import.path)
                    .into_iter()
                    .filter_map(move |target| match target {
                        Target::Dependency { path } if path != import.path => {
                            describe_indirect_dependency_reexport(module, import, &path)
                        }
                        _ => None,
                    })
            })
            .collect()
    }

    fn find_indirect_dependency_reexports(&self) -> Vec<String> {
        self.items_by_module
            .keys()
            .flat_map(|module| self.find_indirect_dependency_reexports_in(module))
            .collect()
    }
}

fn describe_crate(crate_directory: &Path) -> String {
    if crate_directory.as_os_str().is_empty() {
        return String::from("workspace root crate");
    }
    crate_directory.display().to_string()
}

fn find_violations_per_crate(
    files: &[SourceFile],
    find_violations: &dyn Fn(&CrateModules) -> Vec<String>,
) -> Vec<String> {
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
            find_violations(modules)
                .into_iter()
                .map(move |violation| format!("{}: {violation}", describe_crate(crate_directory)))
        })
        .collect()
}

fn find_definitions_with_several_public_paths(files: &[SourceFile]) -> Vec<String> {
    find_violations_per_crate(files, &|modules| modules.find_several_public_paths())
}

fn find_indirect_dependency_reexports(files: &[SourceFile]) -> Vec<String> {
    find_violations_per_crate(files, &|modules| {
        modules.find_indirect_dependency_reexports()
    })
}

pub(crate) fn find_public_path_violations(files: &[SourceFile]) -> Vec<String> {
    find_violations_per_crate(files, &|modules| {
        let mut violations = modules.find_several_public_paths();
        violations.extend(modules.find_indirect_dependency_reexports());
        violations
    })
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
fn a_violation_in_the_workspace_root_crate_names_that_crate() {
    let files = [
        parse_source_file("src/lib.rs", "pub mod resource; pub mod systems;"),
        parse_source_file("src/resource.rs", "pub enum Edit {}"),
        parse_source_file("src/systems.rs", "pub use crate::resource::Edit;"),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "workspace root crate: R6: `Edit` defined in `crate::resource` is public as \
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

#[test]
fn a_dependency_function_reexported_through_an_imported_dependency_module_is_reported() {
    let files = [
        parse_source_file("crates/demo/src/lib.rs", "pub mod tools;"),
        parse_source_file(
            "crates/demo/src/tools.rs",
            "use dep::search; pub use search::find;",
        ),
    ];

    assert_eq!(
        find_indirect_dependency_reexports(&files),
        [
            "crates/demo: R4: `pub use search::find` in `crate::tools` re-exports module or \
          function `dep::search::find` of a dependency"
        ]
    );
}

#[test]
fn a_dependency_function_reexported_through_a_renamed_dependency_is_reported() {
    let files = [parse_source_file(
        "crates/demo/src/lib.rs",
        "use dep as vendor; pub use vendor::find;",
    )];

    assert_eq!(
        find_indirect_dependency_reexports(&files),
        [
            "crates/demo: R4: `pub use vendor::find` in `crate` re-exports module or function \
          `dep::find` of a dependency"
        ]
    );
}

#[test]
fn a_dependency_module_imported_by_a_child_and_reexported_from_its_parent_is_reported() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod child; pub use child::search::find;",
        ),
        parse_source_file("crates/demo/src/child.rs", "pub(crate) use dep::search;"),
    ];

    assert_eq!(
        find_indirect_dependency_reexports(&files),
        [
            "crates/demo: R4: `pub use child::search::find` in `crate` re-exports module or \
          function `dep::search::find` of a dependency"
        ]
    );
}

#[test]
fn a_glob_of_an_imported_dependency_module_is_reported() {
    let files = [parse_source_file(
        "crates/demo/src/lib.rs",
        "use dep::search; pub use search::*;",
    )];

    assert_eq!(
        find_indirect_dependency_reexports(&files),
        [
            "crates/demo: R3: `pub use search::*` in `crate` is a glob re-export of dependency \
          module `dep::search`"
        ]
    );
}

#[test]
fn a_dependency_type_reexported_through_an_imported_dependency_module_is_allowed() {
    let files = [parse_source_file(
        "crates/demo/src/lib.rs",
        "use dep::search; pub use search::Finder;",
    )];

    assert!(find_indirect_dependency_reexports(&files).is_empty());
}

#[test]
fn a_direct_dependency_reexport_is_left_to_the_syntax_check() {
    let files = [parse_source_file(
        "crates/demo/src/lib.rs",
        "pub use dep::find; pub use dep::*;",
    )];

    assert!(find_indirect_dependency_reexports(&files).is_empty());
}

#[test]
fn a_facade_reexport_of_an_own_function_is_not_a_dependency_reexport() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod shape; pub use shape::{area as circle_area, tools::*};",
        ),
        parse_source_file(
            "crates/demo/src/shape.rs",
            "pub mod tools; pub fn area() {}",
        ),
        parse_source_file("crates/demo/src/shape/tools.rs", "pub fn measure() {}"),
    ];

    assert!(find_indirect_dependency_reexports(&files).is_empty());
}

#[test]
fn a_glob_cycle_still_counts_the_second_public_path() {
    let files = [
        parse_source_file("crates/demo/src/lib.rs", "pub mod a; pub mod b;"),
        parse_source_file("crates/demo/src/a.rs", "pub use crate::b::*; pub struct A;"),
        parse_source_file("crates/demo/src/b.rs", "pub use crate::a::*;"),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        ["crates/demo: R6: `A` defined in `crate::a` is public as crate::a::A and crate::b::A"]
    );
}

#[test]
fn a_reexport_through_a_glob_imported_module_is_a_second_public_path() {
    let files = [
        parse_source_file("crates/demo/src/lib.rs", "pub mod helpers; pub mod child;"),
        parse_source_file("crates/demo/src/helpers.rs", "pub struct Thing;"),
        parse_source_file(
            "crates/demo/src/child.rs",
            "use super::*; pub use helpers::Thing;",
        ),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "crates/demo: R6: `Thing` defined in `crate::helpers` is public as \
          crate::child::Thing and crate::helpers::Thing"
        ]
    );
}

#[test]
fn a_dependency_function_reexported_through_a_glob_imported_dependency_module_is_reported() {
    let files = [
        parse_source_file("crates/demo/src/lib.rs", "use dep::search; pub mod child;"),
        parse_source_file(
            "crates/demo/src/child.rs",
            "use super::*; pub use search::find;",
        ),
    ];

    assert_eq!(
        find_indirect_dependency_reexports(&files),
        [
            "crates/demo: R4: `pub use search::find` in `crate::child` re-exports module or \
          function `dep::search::find` of a dependency"
        ]
    );
}

#[test]
fn a_dependency_imported_under_its_own_name_settles() {
    let files = [parse_source_file(
        "crates/demo/src/lib.rs",
        "use paste::paste; pub use paste::Item;",
    )];

    assert!(find_indirect_dependency_reexports(&files).is_empty());
}

#[test]
fn a_named_reexport_of_a_generated_item_beside_its_glob_is_a_second_public_path() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod generated; pub use generated::*; pub mod dispatch;",
        ),
        parse_source_file(
            "crates/demo/src/generated.rs",
            r#"include!(concat!(env!("OUT_DIR"), "/blocks.rs"));"#,
        ),
        parse_source_file(
            "crates/demo/src/dispatch.rs",
            "pub use crate::generated::Block;",
        ),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "crates/demo: R6: `Block` defined in `crate::generated` is public as \
          crate::Block and crate::dispatch::Block"
        ]
    );
}

#[test]
fn two_globs_of_a_generated_module_are_two_public_paths() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod generated; pub use generated::*; pub mod dispatch;",
        ),
        parse_source_file(
            "crates/demo/src/generated.rs",
            r#"include!(concat!(env!("OUT_DIR"), "/blocks.rs"));"#,
        ),
        parse_source_file(
            "crates/demo/src/dispatch.rs",
            "pub use crate::generated::*;",
        ),
    ];

    assert_eq!(
        find_definitions_with_several_public_paths(&files),
        [
            "crates/demo: R6: glob of generated items in `crate::generated` is public as \
          crate::* and crate::dispatch::*"
        ]
    );
}

#[test]
fn a_glob_facade_of_a_generated_module_is_one_public_path() {
    let files = [
        parse_source_file(
            "crates/demo/src/lib.rs",
            "mod generated; pub use generated::*;",
        ),
        parse_source_file(
            "crates/demo/src/generated.rs",
            r#"include!(concat!(env!("OUT_DIR"), "/blocks.rs"));"#,
        ),
    ];

    assert!(find_definitions_with_several_public_paths(&files).is_empty());
}
