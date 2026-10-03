use crate::ecs::world::World;

pub type DroppedFileHandler = fn(&mut World, &str);

#[derive(Clone, Copy)]
pub struct DroppedFileHook {
    pub extension: &'static str,
    pub apply: DroppedFileHandler,
}

#[macro_export]
macro_rules! dropped_file_hook {
    ($ext:literal, $handler:path) => {
        inventory::submit! {
            $crate::hooks::dropped_file::DroppedFileHook {
                extension: $ext,
                apply: $handler,
            }
        }
    };
}

inventory::collect!(DroppedFileHook);

pub fn find_dropped_file_hook(extension: &str) -> Option<DroppedFileHandler> {
    inventory::iter::<DroppedFileHook>
        .into_iter()
        .copied()
        .find(|hook| hook.extension == extension)
        .map(|hook| hook.apply)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_probe_drop(_world: &mut World, _path: &str) {}

    crate::dropped_file_hook!("probetest", record_probe_drop);

    #[test]
    fn test_find_hook_by_registered_extension() {
        assert!(find_dropped_file_hook("probetest").is_some());
        assert!(find_dropped_file_hook("no-such-extension").is_none());
    }
}
