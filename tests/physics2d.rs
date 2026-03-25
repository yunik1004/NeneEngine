use nene::physics::d2::{ColliderBuilder, RigidBodyBuilder, RigidBodyType, World};

#[test]
fn world_default_gravity() {
    let world = World::new();
    // just verify it constructs without panic
    drop(world);
}

#[test]
fn world_custom_gravity() {
    let world = World::with_gravity([0.0, 0.0]);
    drop(world);
}

#[test]
fn dynamic_body_falls_under_gravity() {
    let mut world = World::new();
    let body = RigidBodyBuilder::dynamic().translation(0.0, 10.0).build();
    let handle = world.add_body(body);
    world.add_collider(ColliderBuilder::ball(0.5).build(), handle);

    let y_before = world.body(handle).unwrap().translation().y;

    for _ in 0..60 {
        world.step();
    }

    let y_after = world.body(handle).unwrap().translation().y;
    assert!(y_after < y_before, "dynamic body should fall under gravity");
}

#[test]
fn fixed_body_does_not_move() {
    let mut world = World::new();
    let body = RigidBodyBuilder::fixed().translation(0.0, 0.0).build();
    let handle = world.add_body(body);

    for _ in 0..60 {
        world.step();
    }

    let pos = world.body(handle).unwrap().translation();
    assert!((pos.x).abs() < 1e-6);
    assert!((pos.y).abs() < 1e-6);
}

#[test]
fn zero_gravity_body_stays_put() {
    let mut world = World::with_gravity([0.0, 0.0]);
    let body = RigidBodyBuilder::dynamic().translation(3.0, 5.0).build();
    let handle = world.add_body(body);

    for _ in 0..60 {
        world.step();
    }

    let pos = world.body(handle).unwrap().translation();
    assert!((pos.x - 3.0).abs() < 1e-4);
    assert!((pos.y - 5.0).abs() < 1e-4);
}

#[test]
fn add_collider_to_body() {
    let mut world = World::new();
    let body = RigidBodyBuilder::dynamic().build();
    let handle = world.add_body(body);
    let col = ColliderBuilder::ball(0.5).build();
    let col_handle = world.add_collider(col, handle);
    // collider handle should be valid (no panic)
    world.remove_collider(col_handle);
}

#[test]
fn add_free_collider() {
    let mut world = World::new();
    let col = ColliderBuilder::cuboid(1.0, 1.0).build();
    let handle = world.add_free_collider(col);
    world.remove_collider(handle);
}

#[test]
fn remove_body_invalidates_handle() {
    let mut world = World::new();
    let body = RigidBodyBuilder::dynamic().build();
    let handle = world.add_body(body);
    assert!(world.body(handle).is_some());
    world.remove_body(handle);
    assert!(world.body(handle).is_none());
}

#[test]
fn body_type_is_correct() {
    let mut world = World::new();
    let dynamic = world.add_body(RigidBodyBuilder::dynamic().build());
    let fixed = world.add_body(RigidBodyBuilder::fixed().build());

    assert_eq!(
        world.body(dynamic).unwrap().body_type(),
        RigidBodyType::Dynamic
    );
    assert_eq!(world.body(fixed).unwrap().body_type(), RigidBodyType::Fixed);
}

#[test]
fn step_dt_advances_simulation() {
    let mut world = World::new();
    let body = RigidBodyBuilder::dynamic().translation(0.0, 100.0).build();
    let handle = world.add_body(body);
    world.add_collider(ColliderBuilder::ball(0.5).build(), handle);

    let y_before = world.body(handle).unwrap().translation().y;
    world.step_dt(1.0 / 60.0);
    let y_after = world.body(handle).unwrap().translation().y;

    assert!(y_after < y_before);
}
