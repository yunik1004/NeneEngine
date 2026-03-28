use crate::ecs::world::World;

// ── Component ─────────────────────────────────────────────────────────────────

/// Marker trait automatically implemented for every `Send + Sync + 'static` type.
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

// ── SparseSet ─────────────────────────────────────────────────────────────────

/// Sparse-set storage for a single component type.
///
/// - Random access: O(1) via `sparse` index
/// - Iteration: O(n) over packed `dense` arrays — no gaps
/// - Insert / remove: O(1) amortised (swap-remove)
pub struct SparseSet<T> {
    /// entity index → dense slot (`u32::MAX` = absent)
    sparse: Vec<u32>,
    /// packed component values
    dense: Vec<T>,
    /// dense slot → entity index (reverse map)
    dense_ids: Vec<u32>,
}

impl<T: Component> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            dense_ids: Vec::new(),
        }
    }

    pub fn insert(&mut self, idx: u32, value: T) {
        let i = idx as usize;
        if i >= self.sparse.len() {
            self.sparse.resize(i + 1, u32::MAX);
        }
        if self.sparse[i] != u32::MAX {
            self.dense[self.sparse[i] as usize] = value;
        } else {
            self.sparse[i] = self.dense.len() as u32;
            self.dense.push(value);
            self.dense_ids.push(idx);
        }
    }

    pub fn remove(&mut self, idx: u32) -> Option<T> {
        let i = idx as usize;
        if i >= self.sparse.len() || self.sparse[i] == u32::MAX {
            return None;
        }
        let slot = self.sparse[i] as usize;
        self.sparse[i] = u32::MAX;
        let value = self.dense.swap_remove(slot);
        self.dense_ids.swap_remove(slot);
        if slot < self.dense.len() {
            let moved = self.dense_ids[slot];
            self.sparse[moved as usize] = slot as u32;
        }
        Some(value)
    }

    pub fn get(&self, idx: u32) -> Option<&T> {
        let i = idx as usize;
        if i >= self.sparse.len() || self.sparse[i] == u32::MAX {
            return None;
        }
        Some(&self.dense[self.sparse[i] as usize])
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<&mut T> {
        let i = idx as usize;
        if i >= self.sparse.len() || self.sparse[i] == u32::MAX {
            return None;
        }
        Some(&mut self.dense[self.sparse[i] as usize])
    }

    pub fn contains(&self, idx: u32) -> bool {
        let i = idx as usize;
        i < self.sparse.len() && self.sparse[i] != u32::MAX
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.dense_ids.iter().copied().zip(self.dense.iter())
    }

    pub fn len(&self) -> usize {
        self.dense.len()
    }
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    pub fn clear(&mut self) {
        for &idx in &self.dense_ids {
            if (idx as usize) < self.sparse.len() {
                self.sparse[idx as usize] = u32::MAX;
            }
        }
        self.dense.clear();
        self.dense_ids.clear();
    }

    /// Split the dense arrays for simultaneous id and mutable value iteration.
    pub(crate) fn iter_parts_mut(&mut self) -> (&[u32], &mut [T]) {
        (&self.dense_ids, &mut self.dense)
    }

    pub(crate) fn dense_ids(&self) -> &[u32] {
        &self.dense_ids
    }
    pub(crate) fn dense_values(&self) -> &[T] {
        &self.dense
    }
}

impl<T: Component> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ── ErasedStorage ─────────────────────────────────────────────────────────────

pub(crate) trait ErasedStorage: std::any::Any + Send + Sync {
    fn remove_entity(&mut self, idx: u32);
    fn contains_entity(&self, idx: u32) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: Component> ErasedStorage for SparseSet<T> {
    fn remove_entity(&mut self, idx: u32) {
        self.remove(idx);
    }
    fn contains_entity(&self, idx: u32) -> bool {
        self.contains(idx)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub(crate) fn downcast_ref<T: Component>(s: &dyn ErasedStorage) -> Option<&SparseSet<T>> {
    s.as_any().downcast_ref()
}

pub(crate) fn downcast_mut<T: Component>(s: &mut dyn ErasedStorage) -> Option<&mut SparseSet<T>> {
    s.as_any_mut().downcast_mut()
}

// ── Bundle ────────────────────────────────────────────────────────────────────

/// A set of components that can be spawned together.
///
/// Implemented for tuples up to 12 elements. For a single component, use
/// [`World::spawn_one`] or a 1-tuple: `world.spawn((component,))`.
pub trait Bundle: 'static {
    #[doc(hidden)]
    fn insert_into(self, world: &mut World, idx: u32);
}

macro_rules! impl_bundle {
    ($($T:ident),+) => {
        impl<$($T: Component),+> Bundle for ($($T,)+) {
            #[allow(non_snake_case)]
            fn insert_into(self, world: &mut World, idx: u32) {
                let ($($T,)+) = self;
                $(world.insert_raw(idx, $T);)+
            }
        }
    };
}

impl_bundle!(A);
impl_bundle!(A, B);
impl_bundle!(A, B, C);
impl_bundle!(A, B, C, D);
impl_bundle!(A, B, C, D, E);
impl_bundle!(A, B, C, D, E, F);
impl_bundle!(A, B, C, D, E, F, G);
impl_bundle!(A, B, C, D, E, F, G, H);
impl_bundle!(A, B, C, D, E, F, G, H, I);
impl_bundle!(A, B, C, D, E, F, G, H, I, J);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K);
impl_bundle!(A, B, C, D, E, F, G, H, I, J, K, L);
