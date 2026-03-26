use nene::physics::d3::{ColliderBuilder, RigidBodyBuilder, RigidBodyType, World};

#[test]
fn world_default_gravity() {
    let world = World::new();
    drop(world);
}

#[test]
fn world_custom_gravity() {
    let world = World::with_gravity([0.0, 0.0, 0.0]);
    drop(world);
}

#[test]
fn dynamic_body_falls_under_gravity() {
    let mut world = World::new();
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 10.0, 0.0));
    world.add_collider(ColliderBuilder::ball(0.5), handle);

    let y_before = world.position(handle).unwrap().y;
    for _ in 0..60 {
        world.step();
    }
    let y_after = world.position(handle).unwrap().y;

    assert!(y_after < y_before, "dynamic body should fall under gravity");
}

#[test]
fn fixed_body_does_not_move() {
    let mut world = World::new();
    let handle = world.add_body(RigidBodyBuilder::fixed().translation(0.0, 0.0, 0.0));
    for _ in 0..60 {
        world.step();
    }
    let pos = world.position(handle).unwrap();
    assert!(pos.x.abs() < 1e-6);
    assert!(pos.y.abs() < 1e-6);
    assert!(pos.z.abs() < 1e-6);
}

#[test]
fn zero_gravity_body_stays_put() {
    let mut world = World::with_gravity([0.0, 0.0, 0.0]);
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(3.0, 5.0, 7.0));
    for _ in 0..60 {
        world.step();
    }
    let pos = world.position(handle).unwrap();
    assert!((pos.x - 3.0).abs() < 1e-4);
    assert!((pos.y - 5.0).abs() < 1e-4);
    assert!((pos.z - 7.0).abs() < 1e-4);
}

#[test]
fn add_collider_to_body() {
    let mut world = World::new();
    let handle = world.add_body(RigidBodyBuilder::dynamic());
    let col_handle = world.add_collider(ColliderBuilder::ball(0.5), handle);
    world.remove_collider(col_handle);
}

#[test]
fn add_free_collider() {
    let mut world = World::new();
    let handle = world.add_free_collider(ColliderBuilder::cuboid(1.0, 1.0, 1.0));
    world.remove_collider(handle);
}

#[test]
fn remove_body_invalidates_handle() {
    let mut world = World::new();
    let handle = world.add_body(RigidBodyBuilder::dynamic());
    assert!(world.is_alive(handle));
    world.remove_body(handle);
    assert!(!world.is_alive(handle));
}

#[test]
fn body_type_is_correct() {
    let mut world = World::new();
    let dynamic = world.add_body(RigidBodyBuilder::dynamic());
    let fixed = world.add_body(RigidBodyBuilder::fixed());
    assert_eq!(world.body_type(dynamic), Some(RigidBodyType::Dynamic));
    assert_eq!(world.body_type(fixed), Some(RigidBodyType::Fixed));
}

#[test]
fn step_dt_advances_simulation() {
    let mut world = World::new();
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 100.0, 0.0));
    world.add_collider(ColliderBuilder::ball(0.5), handle);

    let y_before = world.position(handle).unwrap().y;
    world.step_dt(1.0 / 60.0);
    let y_after = world.position(handle).unwrap().y;

    assert!(y_after < y_before);
}
