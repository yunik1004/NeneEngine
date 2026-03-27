use nene::physics::d2::{ColliderBuilder, RigidBodyBuilder, RigidBodyType, World};

#[test]
fn world_default_gravity() {
    let world = World::new();
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
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 10.0));
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
    let handle = world.add_body(RigidBodyBuilder::fixed().translation(0.0, 0.0));
    for _ in 0..60 {
        world.step();
    }
    let pos = world.position(handle).unwrap();
    assert!(pos.x.abs() < 1e-6);
    assert!(pos.y.abs() < 1e-6);
}

#[test]
fn zero_gravity_body_stays_put() {
    let mut world = World::with_gravity([0.0, 0.0]);
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(3.0, 5.0));
    for _ in 0..60 {
        world.step();
    }
    let pos = world.position(handle).unwrap();
    assert!((pos.x - 3.0).abs() < 1e-4);
    assert!((pos.y - 5.0).abs() < 1e-4);
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
    let handle = world.add_free_collider(ColliderBuilder::cuboid(1.0, 1.0));
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
    let handle = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 100.0));
    world.add_collider(ColliderBuilder::ball(0.5), handle);

    let y_before = world.position(handle).unwrap().y;
    world.step_dt(1.0 / 60.0);
    let y_after = world.position(handle).unwrap().y;

    assert!(y_after < y_before);
}

// ── Raycasting ────────────────────────────────────────────────────────────────

/// Helper: world with a 2×2 box centred at origin.
fn world_with_box() -> (World, nene::physics::d2::ColliderHandle) {
    let mut world = World::with_gravity([0.0, 0.0]);
    let body = world.add_body(RigidBodyBuilder::fixed());
    let col = world.add_collider(ColliderBuilder::cuboid(1.0, 1.0), body);
    world.step(); // update BVH
    (world, col)
}

#[test]
fn cast_ray_hits_box() {
    let (world, col) = world_with_box();
    // Shoot downward from above the box.
    let hit = world.cast_ray(
        nene::math::Vec2::new(0.0, 5.0),
        nene::math::Vec2::new(0.0, -1.0),
        20.0,
        true,
    );
    assert!(hit.is_some(), "ray should hit the box");
    let hit = hit.unwrap();
    assert_eq!(hit.collider, col);
    assert!(hit.toi > 0.0 && hit.toi < 20.0);
}

#[test]
fn cast_ray_misses_when_off_to_side() {
    let (world, _) = world_with_box();
    let hit = world.cast_ray(
        nene::math::Vec2::new(5.0, 5.0),  // off to the side
        nene::math::Vec2::new(0.0, -1.0), // straight down, won't intersect
        20.0,
        true,
    );
    assert!(hit.is_none(), "ray should miss");
}

#[test]
fn cast_ray_normal_points_up_for_top_surface() {
    let (world, _) = world_with_box();
    let hit = world
        .cast_ray(
            nene::math::Vec2::new(0.0, 5.0),
            nene::math::Vec2::new(0.0, -1.0),
            20.0,
            true,
        )
        .unwrap();
    // Normal should point upward (away from box top surface).
    assert!(
        hit.normal.y > 0.5,
        "normal should point upward, got {:?}",
        hit.normal
    );
}

#[test]
fn cast_ray_body_handle_present_for_attached_collider() {
    let (world, _) = world_with_box();
    let hit = world
        .cast_ray(
            nene::math::Vec2::new(0.0, 5.0),
            nene::math::Vec2::new(0.0, -1.0),
            20.0,
            true,
        )
        .unwrap();
    assert!(hit.body.is_some(), "hit collider should have a parent body");
}

#[test]
fn cast_ray_all_returns_hit() {
    let (world, _) = world_with_box();
    let hits = world.cast_ray_all(
        nene::math::Vec2::new(0.0, 5.0),
        nene::math::Vec2::new(0.0, -1.0),
        20.0,
        true,
    );
    assert!(
        !hits.is_empty(),
        "cast_ray_all should return at least one hit"
    );
}

#[test]
fn intersect_point_inside_box() {
    let (world, col) = world_with_box();
    let hits = world.intersect_point(nene::math::Vec2::new(0.0, 0.0));
    assert!(
        hits.contains(&col),
        "point at origin should be inside the box"
    );
}

#[test]
fn intersect_point_outside_box() {
    let (world, _) = world_with_box();
    let hits = world.intersect_point(nene::math::Vec2::new(10.0, 10.0));
    assert!(
        hits.is_empty(),
        "point far away should not intersect anything"
    );
}
