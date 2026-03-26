use nene::{
    math::{Quat, Vec3},
    scene::{Node, Scene, Transform},
};

// ── basic structure ───────────────────────────────────────────────────────────

#[test]
fn scene_starts_empty() {
    let scene = Scene::new();
    assert!(scene.is_empty());
    assert!(scene.roots().is_empty());
}

#[test]
fn add_root_node_increments_len() {
    let mut scene = Scene::new();
    scene.add_node(Node::new());
    assert_eq!(scene.len(), 1);
}

#[test]
fn add_child_increments_len() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::new());
    scene.add_child(root, Node::new());
    assert_eq!(scene.len(), 2);
}

#[test]
fn node_named() {
    let mut scene = Scene::new();
    let id = scene.add_node(Node::named("sun"));
    assert_eq!(scene.get(id).name.as_deref(), Some("sun"));
}

#[test]
fn add_child_sets_parent() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::new());
    let child = scene.add_child(root, Node::new());
    assert_eq!(scene.get(child).parent(), Some(root));
}

#[test]
fn root_nodes_list() {
    let mut scene = Scene::new();
    let a = scene.add_node(Node::new());
    let b = scene.add_node(Node::new());
    assert!(scene.roots().contains(&a));
    assert!(scene.roots().contains(&b));
}

#[test]
fn child_not_in_roots() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::new());
    let child = scene.add_child(root, Node::new());
    assert!(!scene.roots().contains(&child));
}

// ── world-transform propagation ───────────────────────────────────────────────

#[test]
fn world_transform_default_is_identity() {
    let mut scene = Scene::new();
    let id = scene.add_node(Node::new());
    scene.update();
    assert_eq!(scene.get(id).world_transform(), nene::math::Mat4::IDENTITY);
}

#[test]
fn world_transform_root_translation() {
    let mut scene = Scene::new();
    let id = scene.add_node(
        Node::new().with_transform(Transform::from_position(Vec3::new(1.0, 2.0, 3.0))),
    );
    scene.update();
    let col = scene.get(id).world_transform().w_axis;
    assert!((col.x - 1.0).abs() < 1e-5);
    assert!((col.y - 2.0).abs() < 1e-5);
    assert!((col.z - 3.0).abs() < 1e-5);
}

#[test]
fn world_transform_inherits_parent_translation() {
    let mut scene = Scene::new();
    let parent = scene.add_node(
        Node::new().with_transform(Transform::from_position(Vec3::new(5.0, 0.0, 0.0))),
    );
    let child = scene.add_child(
        parent,
        Node::new().with_transform(Transform::from_position(Vec3::new(1.0, 0.0, 0.0))),
    );
    scene.update();
    let col = scene.get(child).world_transform().w_axis;
    assert!((col.x - 6.0).abs() < 1e-5);
}

#[test]
fn world_transform_three_levels() {
    let mut scene = Scene::new();
    let root = scene.add_node(
        Node::new().with_transform(Transform::from_position(Vec3::new(1.0, 0.0, 0.0))),
    );
    let mid = scene.add_child(
        root,
        Node::new().with_transform(Transform::from_position(Vec3::new(2.0, 0.0, 0.0))),
    );
    let leaf = scene.add_child(
        mid,
        Node::new().with_transform(Transform::from_position(Vec3::new(3.0, 0.0, 0.0))),
    );
    scene.update();
    let x = scene.get(leaf).world_transform().w_axis.x;
    assert!((x - 6.0).abs() < 1e-5);
}

#[test]
fn world_transform_rotation_applied() {
    let mut scene = Scene::new();
    let id = scene.add_node(
        Node::new().with_transform(Transform::from_rotation(Quat::from_rotation_y(
            std::f32::consts::FRAC_PI_2,
        ))),
    );
    scene.update();
    // In a right-handed system a +90° Y rotation maps X → -Z
    let x_axis = scene.get(id).world_transform().x_axis;
    assert!(x_axis.x.abs() < 1e-5);
    assert!((x_axis.z + 1.0).abs() < 1e-5);
}

#[test]
fn update_called_twice_is_stable() {
    let mut scene = Scene::new();
    let id = scene.add_node(
        Node::new().with_transform(Transform::from_position(Vec3::new(3.0, 0.0, 0.0))),
    );
    scene.update();
    scene.update();
    let x = scene.get(id).world_transform().w_axis.x;
    assert!((x - 3.0).abs() < 1e-5);
}

// ── removal ───────────────────────────────────────────────────────────────────

#[test]
fn remove_root_node() {
    let mut scene = Scene::new();
    let id = scene.add_node(Node::new());
    scene.remove_node(id);
    assert!(scene.is_empty());
    assert!(scene.roots().is_empty());
}

#[test]
fn remove_node_also_removes_children() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::new());
    scene.add_child(root, Node::new());
    scene.add_child(root, Node::new());
    assert_eq!(scene.len(), 3);
    scene.remove_node(root);
    assert!(scene.is_empty());
}

#[test]
fn remove_child_keeps_parent() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::new());
    let child = scene.add_child(root, Node::new());
    scene.remove_node(child);
    assert_eq!(scene.len(), 1);
    assert!(scene.get(root).children().is_empty());
}

// ── walk ──────────────────────────────────────────────────────────────────────

#[test]
fn walk_visits_all_nodes() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::named("root"));
    scene.add_child(root, Node::named("child_a"));
    scene.add_child(root, Node::named("child_b"));

    let mut visited: Vec<String> = Vec::new();
    scene.walk(|_, node| {
        if let Some(n) = &node.name {
            visited.push(n.clone());
        }
    });
    assert_eq!(visited.len(), 3);
}

#[test]
fn walk_preorder_parent_before_child() {
    let mut scene = Scene::new();
    let root = scene.add_node(Node::named("root"));
    scene.add_child(root, Node::named("child"));

    let mut order: Vec<String> = Vec::new();
    scene.walk(|_, node| {
        if let Some(n) = &node.name {
            order.push(n.clone());
        }
    });
    assert_eq!(order[0], "root");
    assert_eq!(order[1], "child");
}
