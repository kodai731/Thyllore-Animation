use crate::asset::AssetStorage;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

/// Draw order of the editor: side panels, the viewport, its overlay, the bottom row, then floating windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiPanel {
    Side,
    Viewport,
    Overlay,
    Bottom,
    Floating,
}

pub type UiWindowInitFn = fn(&mut World);

pub type UiWindowBuildFn = fn(&imgui::Ui, &World, &AssetStorage, &GraphicsResources);

#[derive(Clone, Copy)]
pub struct UiWindowHook {
    pub name: &'static str,
    pub panel: UiPanel,
    pub order: u16,
    pub init: UiWindowInitFn,
    pub build: UiWindowBuildFn,
}

inventory::collect!(UiWindowHook);

pub fn no_window_state(_: &mut World) {}

pub fn init_window_state<T: Default + 'static>(world: &mut World) {
    if !world.contains_resource::<T>() {
        world.insert_resource(T::default());
    }
}

/// Registers a window drawn every frame at `panel` / `order`; `init` inserts its window-local state once.
#[macro_export]
macro_rules! ui_window {
    ($name:literal, $panel:ident, $order:literal, $build:path) => {
        $crate::ui_window!(
            $name,
            $panel,
            $order,
            init = $crate::hooks::ui_window::no_window_state,
            build = $build
        );
    };
    ($name:literal, $panel:ident, $order:literal, init = $init:path, build = $build:path) => {
        inventory::submit! {
            $crate::hooks::ui_window::UiWindowHook {
                name: $name,
                panel: $crate::hooks::ui_window::UiPanel::$panel,
                order: $order,
                init: $init,
                build: $build,
            }
        }
    };
}

/// Every window submitted at link time, sorted by panel, order, then name.
pub struct UiWindows {
    entries: Vec<UiWindowHook>,
}

impl UiWindows {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<UiWindowHook> = inventory::iter::<UiWindowHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| (hook.panel, hook.order, hook.name));
        for pair in entries.windows(2) {
            anyhow::ensure!(
                pair[0].name != pair[1].name,
                "ui window {} registered twice",
                pair[0].name
            );
            anyhow::ensure!(
                (pair[0].panel, pair[0].order) != (pair[1].panel, pair[1].order),
                "ui windows {} and {} share {:?} order {}",
                pair[0].name,
                pair[1].name,
                pair[0].panel,
                pair[0].order
            );
        }
        Ok(Self { entries })
    }

    pub fn init_window_state(&self, world: &mut World) {
        for hook in &self.entries {
            (hook.init)(world);
        }
    }

    pub fn ordered(&self) -> Vec<UiWindowHook> {
        self.entries.clone()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.name).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registered_windows_are_unique_and_ordered() {
        let windows = UiWindows::collect().expect("ui windows collected");

        let keys: Vec<(UiPanel, u16, &str)> = windows
            .ordered()
            .iter()
            .map(|hook| (hook.panel, hook.order, hook.name))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert!(windows.names().contains(&"viewport"));
    }

    #[test]
    fn test_init_window_state_inserts_once() {
        #[derive(Default)]
        struct ProbeWindowState(u32);

        let mut world = World::new();
        init_window_state::<ProbeWindowState>(&mut world);
        world.resource_mut::<ProbeWindowState>().0 = 7;
        init_window_state::<ProbeWindowState>(&mut world);

        assert_eq!(world.resource::<ProbeWindowState>().0, 7);
    }
}
