use std::any::TypeId;
use std::marker::PhantomData;

use crate::ecs::storage::{Component, ErasedStorage, SparseSet, downcast_mut};
use crate::ecs::world::{Entity, EntityMeta, World};

// ── QueryBuilder (immutable) ──────────────────────────────────────────────────

/// Immutable query builder returned by [`World::query`].
///
/// Call [`with`](Self::with) / [`without`](Self::without) to add filters,
/// then iterate with a `for` loop or [`iter`](Self::iter).
///
/// ```
/// # use nene::ecs::World;
/// # struct Position { x: f32 }
/// # struct InRange;
/// # struct Dead;
/// let mut world = World::new();
/// world.spawn((Position { x: 1.0 }, InRange));
///
/// for (_e, pos) in world.query::<Position>().with::<InRange>().without::<Dead>() {
///     let _ = pos.x;
/// }
/// ```
pub struct QueryBuilder<'w, T: Component> {
    world: &'w World,
    with_types: Vec<TypeId>,
    without_types: Vec<TypeId>,
    _phantom: PhantomData<&'w T>,
}

impl<'w, T: Component> QueryBuilder<'w, T> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world,
            with_types: Vec::new(),
            without_types: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Restrict results to entities that also have component `F`.
    pub fn with<F: Component>(mut self) -> Self {
        self.with_types.push(TypeId::of::<F>());
        self
    }

    /// Restrict results to entities that do NOT have component `F`.
    pub fn without<F: Component>(mut self) -> Self {
        self.without_types.push(TypeId::of::<F>());
        self
    }

    /// Iterate over component values only, discarding the entity.
    ///
    /// Use this when you don't need to know which entity owns each component:
    /// ```
    /// # use nene::ecs::World;
    /// # struct Health(f32);
    /// # struct InRange;
    /// # let world = World::new();
    /// for hp in world.query::<Health>().with::<InRange>().values() {
    ///     let _ = hp.0;
    /// }
    /// ```
    pub fn values(self) -> impl Iterator<Item = &'w T> {
        self.iter().map(|(_, v)| v)
    }

    /// Materialise the query into a lazy iterator.
    pub fn iter(self) -> FilteredIter<'w, T> {
        let (ids, values) = match self.world.storage_ref::<T>() {
            Some(s) => (s.dense_ids(), s.dense_values()),
            None => return FilteredIter::empty(&self.world.meta),
        };

        // If any `with` type has no storage at all, result is always empty.
        let with_storages: Vec<&'w dyn ErasedStorage> = self
            .with_types
            .iter()
            .map(|tid| self.world.storages.get(tid).map(|s| s.as_ref()))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default();

        // `without` types that don't exist in storages can be skipped (trivially satisfied).
        let without_storages: Vec<&'w dyn ErasedStorage> = self
            .without_types
            .iter()
            .filter_map(|tid| self.world.storages.get(tid).map(|s| s.as_ref()))
            .collect();

        FilteredIter {
            pos: 0,
            ids,
            values,
            meta: &self.world.meta,
            with_storages,
            without_storages,
        }
    }
}

impl<'w, T: Component> IntoIterator for QueryBuilder<'w, T> {
    type Item = (Entity, &'w T);
    type IntoIter = FilteredIter<'w, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ── FilteredIter (immutable) ──────────────────────────────────────────────────

/// Lazy iterator produced by [`QueryBuilder::iter`].
pub struct FilteredIter<'w, T: Component> {
    pos: usize,
    ids: &'w [u32],
    values: &'w [T],
    meta: &'w [EntityMeta],
    with_storages: Vec<&'w dyn ErasedStorage>,
    without_storages: Vec<&'w dyn ErasedStorage>,
}

impl<'w, T: Component> FilteredIter<'w, T> {
    fn empty(meta: &'w [EntityMeta]) -> Self {
        Self {
            pos: 0,
            ids: &[],
            values: &[],
            meta,
            with_storages: Vec::new(),
            without_storages: Vec::new(),
        }
    }
}

impl<'w, T: Component> Iterator for FilteredIter<'w, T> {
    type Item = (Entity, &'w T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pos >= self.ids.len() {
                return None;
            }
            let id = self.ids[self.pos];
            let value = &self.values[self.pos];
            self.pos += 1;

            let m = &self.meta[id as usize];
            if !m.alive {
                continue;
            }
            if self.with_storages.iter().any(|s| !s.contains_entity(id)) {
                continue;
            }
            if self.without_storages.iter().any(|s| s.contains_entity(id)) {
                continue;
            }

            return Some((
                Entity {
                    id,
                    generation: m.generation,
                },
                value,
            ));
        }
    }
}

// ── QueryBuilderMut (mutable) ─────────────────────────────────────────────────

/// Mutable query builder returned by [`World::query_mut`].
///
/// Call [`with`](Self::with) / [`without`](Self::without) to add filters,
/// then iterate with a `for` loop or [`iter_mut`](Self::iter_mut).
///
/// ```
/// # use nene::ecs::World;
/// # struct Position { x: f32, y: f32 }
/// # struct Velocity  { x: f32, y: f32 }
/// # struct Dead;
/// let mut world = World::new();
/// world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
///
/// for (_e, pos) in world.query_mut::<Position>().without::<Dead>() {
///     pos.x += 1.0;
/// }
/// ```
pub struct QueryBuilderMut<'w, T: Component> {
    world: &'w mut World,
    with_types: Vec<TypeId>,
    without_types: Vec<TypeId>,
    _phantom: PhantomData<&'w mut T>,
}

impl<'w, T: Component> QueryBuilderMut<'w, T> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        Self {
            world,
            with_types: Vec::new(),
            without_types: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Restrict results to entities that also have component `F`.
    pub fn with<F: Component>(mut self) -> Self {
        self.with_types.push(TypeId::of::<F>());
        self
    }

    /// Restrict results to entities that do NOT have component `F`.
    pub fn without<F: Component>(mut self) -> Self {
        self.without_types.push(TypeId::of::<F>());
        self
    }

    /// Iterate over mutable component values only, discarding the entity.
    ///
    /// Use this when you need to mutate components but don't need the entity:
    /// ```
    /// # use nene::ecs::World;
    /// # struct Health(f32);
    /// # struct InRange;
    /// # let mut world = World::new();
    /// for hp in world.query_mut::<Health>().with::<InRange>().values_mut() {
    ///     hp.0 -= 10.0;
    /// }
    /// ```
    pub fn values_mut(self) -> impl Iterator<Item = &'w mut T> {
        self.iter_mut().map(|(_, v)| v)
    }

    /// Materialise the query into an eager iterator of mutable references.
    pub fn iter_mut(self) -> std::vec::IntoIter<(Entity, &'w mut T)> {
        let mut results: Vec<(Entity, &'w mut T)> = Vec::new();
        let type_t = TypeId::of::<T>();

        // Temporarily remove T's storage so we can borrow the rest for filters.
        let Some(mut storage_t) = self.world.storages.remove(&type_t) else {
            return results.into_iter();
        };
        let set_t = downcast_mut::<T>(storage_t.as_mut()).unwrap();
        let (ids, values) = set_t.iter_parts_mut();

        // Bail early if any `with` type has no storage (no entity can match).
        let with_ok = self
            .with_types
            .iter()
            .all(|tid| self.world.storages.contains_key(tid));
        if with_ok {
            let with_storages: Vec<&dyn ErasedStorage> = self
                .with_types
                .iter()
                .map(|tid| self.world.storages[tid].as_ref())
                .collect();
            let without_storages: Vec<&dyn ErasedStorage> = self
                .without_types
                .iter()
                .filter_map(|tid| self.world.storages.get(tid).map(|s| s.as_ref()))
                .collect();

            for i in 0..ids.len() {
                let id = ids[i];
                let m = &self.world.meta[id as usize];
                if !m.alive {
                    continue;
                }
                if with_storages.iter().any(|s| !s.contains_entity(id)) {
                    continue;
                }
                if without_storages.iter().any(|s| s.contains_entity(id)) {
                    continue;
                }
                // SAFETY: pos advances monotonically — each slot yielded at most once.
                let val = unsafe { &mut *(&mut values[i] as *mut T) };
                results.push((
                    Entity {
                        id,
                        generation: m.generation,
                    },
                    val,
                ));
            }
        }

        // Put T's storage back before returning.
        self.world.storages.insert(type_t, storage_t);
        results.into_iter()
    }
}

impl<'w, T: Component> IntoIterator for QueryBuilderMut<'w, T> {
    type Item = (Entity, &'w mut T);
    type IntoIter = std::vec::IntoIter<(Entity, &'w mut T)>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// ── Legacy iterators (used internally by view / view_mut) ─────────────────────

pub(crate) struct RawIter<'w, T: Component> {
    ids: std::slice::Iter<'w, u32>,
    values: std::slice::Iter<'w, T>,
    meta: &'w [EntityMeta],
}

impl<'w, T: Component> RawIter<'w, T> {
    pub(crate) fn new(set: &'w SparseSet<T>, meta: &'w [EntityMeta]) -> Self {
        Self {
            ids: set.dense_ids().iter(),
            values: set.dense_values().iter(),
            meta,
        }
    }
}

impl<'w, T: Component> Iterator for RawIter<'w, T> {
    type Item = (Entity, &'w T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let &id = self.ids.next()?;
            let value = self.values.next()?;
            let m = &self.meta[id as usize];
            if m.alive {
                return Some((
                    Entity {
                        id,
                        generation: m.generation,
                    },
                    value,
                ));
            }
        }
    }
}
