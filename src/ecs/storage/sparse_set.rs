use crate::ecs::world::Entity;

const PAGE_SIZE: usize = 4096;

struct SparsePages {
    pages: Vec<Option<Box<[Option<usize>; PAGE_SIZE]>>>,
}

impl SparsePages {
    fn new() -> Self {
        Self { pages: Vec::new() }
    }

    fn get(&self, entity: Entity) -> Option<usize> {
        let (page_idx, offset) = Self::page_index(entity);
        *self.pages.get(page_idx)?.as_ref()?.get(offset)?
    }

    fn set(&mut self, entity: Entity, value: Option<usize>) {
        let (page_idx, offset) = Self::page_index(entity);

        if page_idx >= self.pages.len() {
            self.pages.resize_with(page_idx + 1, || None);
        }

        if self.pages[page_idx].is_none() {
            self.pages[page_idx] = Some(Box::new([None; PAGE_SIZE]));
        }

        if let Some(page) = &mut self.pages[page_idx] {
            page[offset] = value;
        }
    }

    fn page_index(entity: Entity) -> (usize, usize) {
        let id = entity as usize;
        (id / PAGE_SIZE, id % PAGE_SIZE)
    }

    fn clear(&mut self) {
        self.pages.clear();
    }
}

pub struct SparseSet<T> {
    sparse: SparsePages,
    dense: Vec<T>,
    entities: Vec<Entity>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            sparse: SparsePages::new(),
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: SparsePages::new(),
            dense: Vec::with_capacity(capacity),
            entities: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, entity: Entity, value: T) {
        if let Some(dense_idx) = self.sparse.get(entity) {
            self.dense[dense_idx] = value;
        } else {
            let dense_idx = self.dense.len();
            self.sparse.set(entity, Some(dense_idx));
            self.dense.push(value);
            self.entities.push(entity);
        }
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        let dense_idx = self.sparse.get(entity)?;
        self.dense.get(dense_idx)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let dense_idx = self.sparse.get(entity)?;
        self.dense.get_mut(dense_idx)
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.sparse.get(entity).is_some()
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let dense_idx = self.sparse.get(entity)?;

        self.sparse.set(entity, None);

        let last_idx = self.dense.len() - 1;

        if dense_idx != last_idx {
            let last_entity = self.entities[last_idx];
            self.sparse.set(last_entity, Some(dense_idx));
            self.entities[dense_idx] = last_entity;
            self.dense.swap(dense_idx, last_idx);
        }

        self.entities.pop();
        self.dense.pop()
    }

    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.entities.clear();
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn iter(&self) -> SparseSetIter<'_, T> {
        SparseSetIter {
            entities: self.entities.iter(),
            dense: self.dense.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> SparseSetIterMut<'_, T> {
        SparseSetIterMut {
            entities: self.entities.iter(),
            dense: self.dense.iter_mut(),
        }
    }

    pub fn dense(&self) -> &[T] {
        &self.dense
    }

    pub fn dense_mut(&mut self) -> &mut [T] {
        &mut self.dense
    }
}

pub struct SparseSetIter<'a, T> {
    entities: std::slice::Iter<'a, Entity>,
    dense: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for SparseSetIter<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let entity = *self.entities.next()?;
        let value = self.dense.next()?;
        Some((entity, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.dense.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for SparseSetIter<'a, T> {}

pub struct SparseSetIterMut<'a, T> {
    entities: std::slice::Iter<'a, Entity>,
    dense: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for SparseSetIterMut<'a, T> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        let entity = *self.entities.next()?;
        let value = self.dense.next()?;
        Some((entity, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.dense.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for SparseSetIterMut<'a, T> {}
