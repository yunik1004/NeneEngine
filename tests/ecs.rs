use nene::ecs::{Entity, World};

// ── Component types used across tests ────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq, Clone)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Debug, PartialEq, Clone)]
struct Health(f32);

struct Player;
struct Enemy;
struct InRange;

// ── Entity lifecycle ──────────────────────────────────────────────────────────

#[test]
fn spawn_is_alive() {
    let mut world = World::new();
    let e = world.spawn((Position { x: 0.0, y: 0.0 },));
    assert!(world.is_alive(e));
}

#[test]
fn despawn_marks_dead() {
    let mut world = World::new();
    let e = world.spawn((Position { x: 1.0, y: 2.0 },));
    assert!(world.despawn(e));
    assert!(!world.is_alive(e));
}

#[test]
fn despawn_stale_handle_false() {
    let mut world = World::new();
    let e = world.spawn((Position { x: 0.0, y: 0.0 },));
    world.despawn(e);
    // second despawn must return false — generation mismatch
    assert!(!world.despawn(e));
}

#[test]
fn id_reuse_generation_bump() {
    let mut world = World::new();
    let e1 = world.spawn((Health(100.0),));
    world.despawn(e1);
    let e2 = world.spawn((Health(50.0),));
    // same slot, different generation
    assert_ne!(e1, e2);
    assert!(!world.is_alive(e1));
    assert!(world.is_alive(e2));
}

#[test]
fn world_len_tracks_count() {
    let mut world = World::new();
    assert_eq!(world.len(), 0);
    let a = world.spawn((Position { x: 0.0, y: 0.0 },));
    let _b = world.spawn((Position { x: 1.0, y: 0.0 },));
    assert_eq!(world.len(), 2);
    world.despawn(a);
    assert_eq!(world.len(), 1);
}

// ── Component access ──────────────────────────────────────────────────────────

#[test]
fn insert_and_get() {
    let mut world = World::new();
    let e = world.spawn((Position { x: 0.0, y: 0.0 },));
    world.insert(e, Health(80.0));
    assert_eq!(world.get::<Health>(e), Some(&Health(80.0)));
}

#[test]
fn insert_replaces_existing() {
    let mut world = World::new();
    let e = world.spawn((Health(100.0),));
    world.insert(e, Health(50.0));
    assert_eq!(world.get::<Health>(e), Some(&Health(50.0)));
}

#[test]
fn get_missing_returns_none() {
    let mut world = World::new();
    let e = world.spawn((Position { x: 0.0, y: 0.0 },));
    assert_eq!(world.get::<Health>(e), None);
}

#[test]
fn get_dead_entity_returns_none() {
    let mut world = World::new();
    let e = world.spawn((Health(100.0),));
    world.despawn(e);
    assert_eq!(world.get::<Health>(e), None);
}

#[test]
fn get_mut_mutates() {
    let mut world = World::new();
    let e = world.spawn((Health(100.0),));
    world.get_mut::<Health>(e).unwrap().0 -= 30.0;
    assert_eq!(world.get::<Health>(e), Some(&Health(70.0)));
}

#[test]
fn remove_component() {
    let mut world = World::new();
    let e = world.spawn((Health(100.0), Position { x: 0.0, y: 0.0 }));
    let removed = world.remove::<Health>(e);
    assert_eq!(removed, Some(Health(100.0)));
    assert_eq!(world.get::<Health>(e), None);
    // other component still present
    assert!(world.get::<Position>(e).is_some());
}

#[test]
fn has_component() {
    let mut world = World::new();
    let e = world.spawn((Health(50.0),));
    assert!(world.has::<Health>(e));
    assert!(!world.has::<Position>(e));
}

// ── SparseSet remove correctness (the swap-remove bug guard) ──────────────────

#[test]
fn remove_then_insert_other_entity() {
    // Exercises the exact bug path: remove entity in middle slot,
    // verify the entity that was swapped to fill the slot is still accessible.
    let mut world = World::new();
    let a = world.spawn((Health(10.0),));
    let b = world.spawn((Health(20.0),));
    let c = world.spawn((Health(30.0),));

    world.despawn(b); // removes Health(20.0) — b was dense slot 1, c moves to slot 1

    // c must still be reachable via the updated sparse entry
    assert_eq!(world.get::<Health>(c), Some(&Health(30.0)));

    // inserting a new component on c must NOT panic (previously triggered index OOB)
    world.insert(c, Position { x: 9.0, y: 0.0 });
    assert_eq!(world.get::<Position>(c), Some(&Position { x: 9.0, y: 0.0 }));

    // a is unaffected
    assert_eq!(world.get::<Health>(a), Some(&Health(10.0)));
}

#[test]
fn remove_last_entity_in_dense() {
    let mut world = World::new();
    let a = world.spawn((Health(1.0),));
    let b = world.spawn((Health(2.0),));
    world.despawn(b); // remove last dense slot — no swap needed
    assert_eq!(world.get::<Health>(a), Some(&Health(1.0)));
    assert_eq!(world.get::<Health>(b), None);
}

// ── Bundle spawn ─────────────────────────────────────────────────────────────

#[test]
fn spawn_multi_component_bundle() {
    let mut world = World::new();
    let e = world.spawn((
        Position { x: 1.0, y: 2.0 },
        Velocity { x: 3.0, y: 4.0 },
        Health(100.0),
    ));
    assert_eq!(world.get::<Position>(e), Some(&Position { x: 1.0, y: 2.0 }));
    assert_eq!(world.get::<Velocity>(e), Some(&Velocity { x: 3.0, y: 4.0 }));
    assert_eq!(world.get::<Health>(e), Some(&Health(100.0)));
}

#[test]
fn spawn_one() {
    let mut world = World::new();
    let e = world.spawn_one(Health(42.0));
    assert_eq!(world.get::<Health>(e), Some(&Health(42.0)));
}

// ── Query (immutable) ─────────────────────────────────────────────────────────

#[test]
fn query_all() {
    let mut world = World::new();
    world.spawn((Health(10.0),));
    world.spawn((Health(20.0),));
    world.spawn((Position { x: 0.0, y: 0.0 },)); // no Health
    let sum: f32 = world.query::<Health>().iter().map(|(_, h)| h.0).sum();
    assert_eq!(sum, 30.0);
}

#[test]
fn query_with_filter() {
    let mut world = World::new();
    world.spawn((Health(10.0), Enemy));
    world.spawn((Health(20.0), Player));
    world.spawn((Health(30.0), Enemy));

    let enemy_hp: f32 = world
        .query::<Health>()
        .with::<Enemy>()
        .iter()
        .map(|(_, h)| h.0)
        .sum();
    assert_eq!(enemy_hp, 40.0);
}

#[test]
fn query_without_filter() {
    let mut world = World::new();
    world.spawn((Health(10.0), InRange));
    world.spawn((Health(20.0),));
    world.spawn((Health(30.0), InRange));

    let out: f32 = world
        .query::<Health>()
        .without::<InRange>()
        .iter()
        .map(|(_, h)| h.0)
        .sum();
    assert_eq!(out, 20.0);
}

#[test]
fn query_with_and_without() {
    let mut world = World::new();
    world.spawn((Health(10.0), Enemy, InRange));
    world.spawn((Health(20.0), Enemy)); // Enemy, not InRange
    world.spawn((Health(30.0), Player, InRange));

    // Enemy AND NOT InRange
    let hp: f32 = world
        .query::<Health>()
        .with::<Enemy>()
        .without::<InRange>()
        .iter()
        .map(|(_, h)| h.0)
        .sum();
    assert_eq!(hp, 20.0);
}

#[test]
fn query_empty_when_no_storage() {
    let world = World::new();
    let count = world.query::<Health>().iter().count();
    assert_eq!(count, 0);
}

#[test]
fn query_skips_despawned() {
    let mut world = World::new();
    let a = world.spawn((Health(10.0),));
    world.spawn((Health(20.0),));
    world.despawn(a);
    let count = world.query::<Health>().iter().count();
    assert_eq!(count, 1);
}

// ── Query (mutable) ──────────────────────────────────────────────────────────

#[test]
fn query_mut_modifies() {
    let mut world = World::new();
    world.spawn((Health(100.0), Enemy));
    world.spawn((Health(100.0), Enemy));
    world.spawn((Health(100.0), Player));

    for (_, hp) in world.query_mut::<Health>().with::<Enemy>() {
        hp.0 -= 10.0;
    }

    let enemy_hp: f32 = world
        .query::<Health>()
        .with::<Enemy>()
        .iter()
        .map(|(_, h)| h.0)
        .sum();
    let player_hp: f32 = world
        .query::<Health>()
        .with::<Player>()
        .iter()
        .map(|(_, h)| h.0)
        .sum();
    assert_eq!(enemy_hp, 180.0);
    assert_eq!(player_hp, 100.0);
}

#[test]
fn query_mut_without_filter() {
    let mut world = World::new();
    world.spawn((Health(50.0), InRange));
    world.spawn((Health(50.0),));

    for (_, hp) in world.query_mut::<Health>().without::<InRange>() {
        hp.0 = 0.0;
    }

    let still_alive: usize = world
        .query::<Health>()
        .iter()
        .filter(|(_, h)| h.0 > 0.0)
        .count();
    assert_eq!(still_alive, 1);
}

// ── IntoIterator (for-loop syntax) ────────────────────────────────────────────

#[test]
fn into_iter_query() {
    let mut world = World::new();
    world.spawn((Health(5.0),));
    let count = world.query::<Health>().into_iter().count();
    assert_eq!(count, 1);
}

#[test]
fn into_iter_query_mut() {
    let mut world = World::new();
    world.spawn((Health(10.0),));
    for (_, hp) in world.query_mut::<Health>() {
        hp.0 *= 2.0;
    }
    let val = world.query::<Health>().iter().next().map(|(_, h)| h.0);
    assert_eq!(val, Some(20.0));
}

// ── view / view_mut ──────────────────────────────────────────────────────────

#[test]
fn view_two_components() {
    let mut world = World::new();
    world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 2.0, y: 0.0 }));
    world.spawn((Position { x: 3.0, y: 0.0 },)); // no Velocity, skipped

    let mut count = 0;
    world.view(|_, pos: &Position, vel: &Velocity| {
        assert_eq!(pos.x + vel.x, 3.0);
        count += 1;
    });
    assert_eq!(count, 1);
}

#[test]
fn view_mut_two_components() {
    let mut world = World::new();
    world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 2.0 }));

    world.view_mut(|_, pos: &mut Position, vel: &Velocity| {
        pos.x += vel.x;
        pos.y += vel.y;
    });

    let pos = world
        .query::<Position>()
        .iter()
        .next()
        .map(|(_, p)| (p.x, p.y));
    assert_eq!(pos, Some((1.0, 2.0)));
}

// ── Dynamic component add/remove (marker tagging) ────────────────────────────

#[test]
fn tag_and_untag_marker() {
    let mut world = World::new();
    let e = world.spawn((Health(100.0), Enemy));

    world.insert(e, InRange);
    assert!(world.has::<InRange>(e));

    let count_in = world.query::<Health>().with::<InRange>().iter().count();
    assert_eq!(count_in, 1);

    world.remove::<InRange>(e);
    assert!(!world.has::<InRange>(e));

    let count_in = world.query::<Health>().with::<InRange>().iter().count();
    assert_eq!(count_in, 0);
}

#[test]
fn tag_many_despawn_some_retag() {
    let mut world = World::new();
    let entities: Vec<Entity> = (0..20)
        .map(|i| world.spawn((Health(i as f32), Enemy)))
        .collect();

    // tag even-indexed
    for &e in entities.iter().step_by(2) {
        world.insert(e, InRange);
    }
    // despawn odd-indexed
    for &e in entities.iter().skip(1).step_by(2) {
        world.despawn(e);
    }

    // all surviving tagged entities still visible
    let tagged = world.query::<Health>().with::<InRange>().iter().count();
    assert_eq!(tagged, 10);

    // untag all
    let to_untag: Vec<Entity> = world
        .query::<Health>()
        .with::<InRange>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    for e in to_untag {
        world.remove::<InRange>(e);
    }

    let tagged_after = world.query::<Health>().with::<InRange>().iter().count();
    assert_eq!(tagged_after, 0);
}
