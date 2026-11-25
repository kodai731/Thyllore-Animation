use crate::ecs::world::World;

pub type DroppedFileHandler = fn(&mut World, &str);

#[derive(Clone, Copy)]
pub struct DroppedFileHook {
    pub file_name_suffix: &'static str,
    pub apply: DroppedFileHandler,
}

#[macro_export]
macro_rules! dropped_file_hook {
    ($file_name_suffix:literal, $handler:path) => {
        inventory::submit! {
            $crate::hooks::dropped_file::DroppedFileHook {
                file_name_suffix: $file_name_suffix,
                apply: $handler,
            }
        }
    };
}

inventory::collect!(DroppedFileHook);

/// A hook names the whole ending of the files it takes, so `.anim.ron` leaves other `.ron` files alone.
pub fn find_dropped_file_hook(file_path: &str) -> Option<DroppedFileHandler> {
    let lowercase_path = file_path.to_ascii_lowercase();

    inventory::iter::<DroppedFileHook>
        .into_iter()
        .copied()
        .find(|hook| lowercase_path.ends_with(hook.file_name_suffix))
        .map(|hook| hook.apply)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_probe_drop(_world: &mut World, _path: &str) {}

    crate::dropped_file_hook!(".probe.test", record_probe_drop);

    #[test]
    fn hook_takes_the_files_ending_in_its_suffix() {
        assert!(find_dropped_file_hook("dir/Clip.PROBE.test").is_some());
        assert!(find_dropped_file_hook("dir/clip.other.test").is_none());
        assert!(find_dropped_file_hook("dir/no-such-extension").is_none());
    }
}
