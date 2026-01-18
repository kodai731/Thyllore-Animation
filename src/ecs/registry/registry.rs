use std::any::TypeId;
use std::collections::HashMap;

use super::component_info::ComponentInfo;

#[derive(Debug, Default)]
pub struct ComponentRegistry {
    infos: HashMap<TypeId, ComponentInfo>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            infos: HashMap::new(),
        }
    }

    pub fn register<T: 'static>(&mut self) -> &ComponentInfo {
        let type_id = TypeId::of::<T>();
        self.infos
            .entry(type_id)
            .or_insert_with(ComponentInfo::new::<T>)
    }

    pub fn is_registered<T: 'static>(&self) -> bool {
        self.infos.contains_key(&TypeId::of::<T>())
    }

    pub fn get_info<T: 'static>(&self) -> Option<&ComponentInfo> {
        self.infos.get(&TypeId::of::<T>())
    }

    pub fn get_info_by_id(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        self.infos.get(&type_id)
    }

    pub fn all_infos(&self) -> impl Iterator<Item = &ComponentInfo> {
        self.infos.values()
    }

    pub fn component_count(&self) -> usize {
        self.infos.len()
    }
}
