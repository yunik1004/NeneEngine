use std::any::TypeId;
use std::collections::HashMap;

use crate::ecs::query::{QueryBuilder, QueryBuilderMut, RawIter};
use crate::ecs::storage::{
    Bundle, Component, ErasedStorage, SparseSet, downcast_mut, downcast_ref,
};

// ── Entity ────────────────────────────────────────────────────────────────────

/// A lightweight handle to an entity.
///
/// Contains an index and a generation counter so stale handles to despawned
/// (and subsequently re-spawned) entities are detected reliably.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
    pub(crate) id: u32,
    pub(crate) generation: u32,
}

// ── EntityMeta ────────────────────────────────────────────────────────────────

pub(crate) struct EntityMeta {
    pub generation: u32,
    pub alive: bool,
}

// ── World ─────────────────────────────────────────────────────────────────────

/// Container for all entities and their components.
///
/// # Example
/// ```
/// use nene::ecs::{World, Entity};
///
/// #[derive(Debug, PartialEq)]
/// struct Position { x: f32, y: f32 }
/// struct Velocity  { x: f32, y: f32 }
///
/// let mut world = World::new();
///
/// let e = world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
///
/// // Per-frame update
/// world.view_mut(|_e, pos: &mut Position, vel: &Velocity| {
///     pos.x += vel.x;
///     pos.y += vel.y;
/// });
///
/// assert_eq!(world.get::<Position>(e), Some(&Position { x: 1.0, y: 0.0 }));
/// ```
pub struct World {
    pub(crate) meta: Vec<EntityMeta>,
    free: Vec<u32>,
    count: u32,
    pub(crate) storages: HashMap<TypeId, Box<dyn ErasedStorage>>,
}

impl World {
    /// Create an empty world.
    pub fn new() -> Self {
        Self {
            meta: Vec::new(),
            free: Vec::new(),
            count: 0,
            storages: HashMap::new(),
        }
    }

    // ── Entity lifecycle ──────────────────────────────────────────────────────

    /// Spawn a new entity with the given component bundle (tuple).
    ///
    /// For a single component, use [`spawn_one`](Self::spawn_one) or a 1-tuple:
    /// `world.spawn((MyComponent,))`.
    ///
    /// ```
    /// # use nene::ecs::World;
    /// # struct Pos { x: f32 }
    /// # struct Vel { x: f32 }
    /// let mut world = World::new();
    /// let e = world.spawn((Pos { x: 0.0 }, Vel { x: 1.0 }));
    /// assert!(world.is_alive(e));
    /// ```
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let id = if let Some(id) = self.free.pop() {
            self.meta[id as usize].alive = true;
            id
        } else {
            let id = self.meta.len() as u32;
            self.meta.push(EntityMeta {
                generation: 0,
                alive: true,
            });
            id
        };
        self.count += 1;
        let entity = Entity {
            id,
            generation: self.meta[id as usize].generation,
        };
        bundle.insert_into(self, id);
        entity
    }

    /// Spawn a new entity with a single component.
    ///
    /// ```
    /// # use nene::ecs::World;
    /// # struct Health(f32);
    /// let mut world = World::new();
    /// let e = world.spawn_one(Health(100.0));
    /// assert!(world.is_alive(e));
    /// ```
    pub fn spawn_one<T: Component>(&mut self, component: T) -> Entity {
        self.spawn((component,))
    }

    /// Destroy an entity and remove all its components.
    ///
    /// Returns `true` if the entity was alive. Stale handles become invalid.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let idx = entity.id;
        self.meta[idx as usize].alive = false;
        self.meta[idx as usize].generation = self.meta[idx as usize].generation.wrapping_add(1);
        self.count -= 1;
        self.free.push(idx);
        for storage in self.storages.values_mut() {
            storage.remove_entity(idx);
        }
        true
    }

    /// Returns `true` if `entity` was created by this world and has not been despawned.
    pub fn is_alive(&self, entity: Entity) -> bool {
        let i = entity.id as usize;
        i < self.meta.len() && self.meta[i].alive && self.meta[i].generation == entity.generation
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// `true` if no entities exist.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    // ── Component access ──────────────────────────────────────────────────────

    /// Add or replace a component on `entity`.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        if self.is_alive(entity) {
            self.insert_raw(entity.id, value);
        }
    }

    /// Remove a component from `entity`, returning the value if present.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        downcast_mut::<T>(self.storages.get_mut(&TypeId::of::<T>())?.as_mut())?.remove(entity.id)
    }

    /// Borrow a component immutably.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        downcast_ref::<T>(self.storages.get(&TypeId::of::<T>())?.as_ref())?.get(entity.id)
    }

    /// Borrow a component mutably.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        downcast_mut::<T>(self.storages.get_mut(&TypeId::of::<T>())?.as_mut())?.get_mut(entity.id)
    }

    /// Returns `true` if `entity` has component `T`.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.storages
            .get(&TypeId::of::<T>())
            .and_then(|s| downcast_ref::<T>(s.as_ref()))
            .map(|s| s.contains(entity.id))
            .unwrap_or(false)
    }

    // ── Iteration ─────────────────────────────────────────────────────────────

    /// Query entities that have component `T`, with optional filters.
    ///
    /// Returns a [`QueryBuilder`] that supports `.with::<F>()` / `.without::<F>()`
    /// chaining before iteration.
    ///
    /// ```
    /// # use nene::ecs::World;
    /// # struct Pos { x: f32 }
    /// # struct Active;
    /// let mut world = World::new();
    /// world.spawn((Pos { x: 1.0 }, Active));
    /// world.spawn_one(Pos { x: 2.0 }); // no Active
    ///
    /// // All entities with Pos
    /// let total: f32 = world.query::<Pos>().iter().map(|(_, p)| p.x).sum();
    /// assert_eq!(total, 3.0);
    ///
    /// // Only entities with Pos AND Active
    /// let active: f32 = world.query::<Pos>().with::<Active>().iter().map(|(_, p)| p.x).sum();
    /// assert_eq!(active, 1.0);
    /// ```
    pub fn query<T: Component>(&self) -> QueryBuilder<'_, T> {
        QueryBuilder::new(self)
    }

    /// Mutably query entities that have component `T`, with optional filters.
    ///
    /// Returns a [`QueryBuilderMut`] that supports `.with::<F>()` / `.without::<F>()`
    /// chaining before iteration.
    ///
    /// ```
    /// # use nene::ecs::World;
    /// # struct Pos { x: f32, y: f32 }
    /// # struct Vel { x: f32, y: f32 }
    /// # struct Dead;
    /// let mut world = World::new();
    /// world.spawn((Pos { x: 0.0, y: 0.0 }, Vel { x: 1.0, y: 0.0 }));
    ///
    /// for (_, pos) in world.query_mut::<Pos>().without::<Dead>() {
    ///     pos.x += 1.0;
    /// }
    /// ```
    pub fn query_mut<T: Component>(&mut self) -> QueryBuilderMut<'_, T> {
        QueryBuilderMut::new(self)
    }

    /// Visit every entity that has both components `A` and `B`.
    pub fn view<A: Component, B: Component, F>(&self, mut f: F)
    where
        F: FnMut(Entity, &A, &B),
    {
        let Some(set_a) = self.storage_ref::<A>() else {
            return;
        };
        let Some(set_b) = self.storage_ref::<B>() else {
            return;
        };
        for (entity, val_a) in RawIter::new(set_a, &self.meta) {
            if let Some(val_b) = set_b.get(entity.id) {
                f(entity, val_a, val_b);
            }
        }
    }

    /// Visit every entity that has both `A` and `B`, with `A` mutable.
    ///
    /// # Panics
    /// Panics if `A` and `B` are the same type.
    pub fn view_mut<A: Component, B: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(Entity, &mut A, &B),
    {
        let type_a = TypeId::of::<A>();
        let type_b = TypeId::of::<B>();
        assert_ne!(
            type_a, type_b,
            "view_mut: A and B must be different component types"
        );

        // Temporarily remove storage A so we can borrow storages for B simultaneously.
        let Some(mut storage_a) = self.storages.remove(&type_a) else {
            return;
        };
        let set_a = downcast_mut::<A>(storage_a.as_mut())
            .expect("ECS invariant violated: type ID mismatch in view_mut");
        let (ids, values) = set_a.iter_parts_mut();

        if let Some(storage_b) = self.storages.get(&type_b)
            && let Some(set_b) = downcast_ref::<B>(storage_b.as_ref())
        {
            for i in 0..ids.len() {
                let id = ids[i];
                if !self.meta[id as usize].alive {
                    continue;
                }
                if let Some(val_b) = set_b.get(id) {
                    let entity = Entity {
                        id,
                        generation: self.meta[id as usize].generation,
                    };
                    f(entity, &mut values[i], val_b);
                }
            }
        }

        self.storages.insert(type_a, storage_a);
    }

    /// Visit every entity that has components `A`, `B`, and `C`, with `A` mutable.
    ///
    /// # Panics
    /// Panics if any two of `A`, `B`, `C` are the same type.
    pub fn view_mut3<A: Component, B: Component, C: Component, F>(&mut self, mut f: F)
    where
        F: FnMut(Entity, &mut A, &B, &C),
    {
        let type_a = TypeId::of::<A>();
        let type_b = TypeId::of::<B>();
        let type_c = TypeId::of::<C>();
        assert_ne!(type_a, type_b, "view_mut3: A and B must be different");
        assert_ne!(type_a, type_c, "view_mut3: A and C must be different");
        assert_ne!(type_b, type_c, "view_mut3: B and C must be different");

        let Some(mut storage_a) = self.storages.remove(&type_a) else {
            return;
        };
        let set_a = downcast_mut::<A>(storage_a.as_mut())
            .expect("ECS invariant violated: type ID mismatch in view_mut3");
        let (ids, values) = set_a.iter_parts_mut();

        let sb = self.storages.get(&type_b);
        let sc = self.storages.get(&type_c);
        if let (Some(sb), Some(sc)) = (sb, sc)
            && let (Some(set_b), Some(set_c)) = (
                downcast_ref::<B>(sb.as_ref()),
                downcast_ref::<C>(sc.as_ref()),
            )
        {
            for i in 0..ids.len() {
                let id = ids[i];
                if !self.meta[id as usize].alive {
                    continue;
                }
                if let (Some(vb), Some(vc)) = (set_b.get(id), set_c.get(id)) {
                    let entity = Entity {
                        id,
                        generation: self.meta[id as usize].generation,
                    };
                    f(entity, &mut values[i], vb, vc);
                }
            }
        }

        self.storages.insert(type_a, storage_a);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    pub(crate) fn insert_raw<T: Component>(&mut self, idx: u32, value: T) {
        let storage = self
            .storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SparseSet::<T>::new()));
        downcast_mut::<T>(storage.as_mut())
            .expect("ECS invariant violated: type ID mismatch in insert_raw")
            .insert(idx, value);
    }

    pub(crate) fn storage_ref<T: Component>(&self) -> Option<&SparseSet<T>> {
        downcast_ref::<T>(self.storages.get(&TypeId::of::<T>())?.as_ref())
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
