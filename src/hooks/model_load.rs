use crate::asset::AssetStorage;
use crate::ecs::world::{Entity, World};
use crate::loader::ModelLoadResult;

/// The entity a model load produced and the importer result it was built from.
pub struct LoadedModel<'a> {
    pub entity: Entity,
    pub load_result: &'a ModelLoadResult,
}

/// Rig hooks attach data the model carries; display hooks read the world afterwards, so rig always
/// runs first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelLoadStage {
    Rig,
    Display,
}

pub type ModelLoadApplyFn = fn(&mut World, &AssetStorage, &LoadedModel);

#[derive(Clone, Copy)]
pub struct ModelLoadHook {
    pub name: &'static str,
    pub stage: ModelLoadStage,
    pub apply: ModelLoadApplyFn,
}

inventory::collect!(ModelLoadHook);

/// Registers a function that runs once after a model replaced the scene model.
#[macro_export]
macro_rules! model_load_hook {
    ($name:literal, $stage:ident, $apply:path) => {
        inventory::submit! {
            $crate::hooks::model_load::ModelLoadHook {
                name: $name,
                stage: $crate::hooks::model_load::ModelLoadStage::$stage,
                apply: $apply,
            }
        }
    };
}

/// Every model load hook submitted at link time, sorted by stage then name.
pub struct ModelLoadHooks {
    entries: Vec<ModelLoadHook>,
}

impl ModelLoadHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<ModelLoadHook> = inventory::iter::<ModelLoadHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| (hook.stage, hook.name));
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].name != pair[1].name,
                "model load hook {} registered twice",
                pair[0].name
            );
        }
        Ok(Self { entries })
    }

    pub fn ordered(&self) -> impl Iterator<Item = &ModelLoadHook> {
        self.entries.iter()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.name).collect()
    }
}

impl std::fmt::Debug for ModelLoadHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelLoadHooks")
            .field("names", &self.names())
            .finish()
    }
}

pub fn run_model_load_hooks(world: &mut World, assets: &AssetStorage, loaded: &LoadedModel) {
    let hooks: Vec<ModelLoadHook> = match world.get_resource::<ModelLoadHooks>() {
        Some(hooks) => hooks.ordered().copied().collect(),
        None => {
            log_warn!("ModelLoadHooks resource missing: model load hooks not applied");
            return;
        }
    };

    for hook in hooks {
        (hook.apply)(world, assets, loaded);
    }
}
