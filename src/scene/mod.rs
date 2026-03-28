use crate::ecs::Entity;
use crate::math::{Mat4, Quat, Vec3};

/// Local-space transform: position, rotation, uniform scale.
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::default()
        }
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::default()
        }
    }

    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// A stable handle to a node inside a [`Scene`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(usize);

/// A node in the scene hierarchy.
pub struct Node {
    pub name: Option<String>,
    pub transform: Transform,
    /// Optional ECS entity associated with this node.
    ///
    /// Set this to link a scene node to its ECS data. [`Scene::walk_entities`]
    /// then lets you iterate only nodes that have an entity attached.
    pub entity: Option<Entity>,
    world_transform: Mat4,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

impl Node {
    pub fn new() -> Self {
        Self {
            name: None,
            transform: Transform::default(),
            entity: None,
            world_transform: Mat4::IDENTITY,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::new()
        }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Attach an ECS entity to this node.
    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.entity = Some(entity);
        self
    }

    /// World-space transform, valid after the last [`Scene::update`] call.
    pub fn world_transform(&self) -> Mat4 {
        self.world_transform
    }

    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn children(&self) -> &[NodeId] {
        &self.children
    }
}

/// A scene graph: a forest of [`Node`]s connected in parent–child relationships.
///
/// Call [`Scene::update`] once per frame to propagate world transforms from
/// roots down to all descendants.
pub struct Scene {
    nodes: Vec<Option<Node>>,
    roots: Vec<NodeId>,
    free: Vec<usize>,
    count: usize,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            free: Vec::new(),
            count: 0,
        }
    }

    fn alloc(&mut self, node: Node) -> NodeId {
        self.count += 1;
        if let Some(slot) = self.free.pop() {
            self.nodes[slot] = Some(node);
            NodeId(slot)
        } else {
            let id = NodeId(self.nodes.len());
            self.nodes.push(Some(node));
            id
        }
    }

    /// Add a root-level node and return its id.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.alloc(node);
        self.roots.push(id);
        id
    }

    /// Add `node` as a child of `parent` and return its id.
    ///
    /// Returns `None` if `parent` is not a live node.
    pub fn add_child(&mut self, parent: NodeId, mut node: Node) -> Option<NodeId> {
        self.nodes.get(parent.0).and_then(Option::as_ref)?;
        node.parent = Some(parent);
        let id = self.alloc(node);
        if let Some(Some(p)) = self.nodes.get_mut(parent.0) {
            p.children.push(id);
        }
        Some(id)
    }

    /// Remove `id` and all its descendants from the scene.
    pub fn remove_node(&mut self, id: NodeId) {
        let mut to_remove = vec![id];
        let mut i = 0;
        while i < to_remove.len() {
            let cur = to_remove[i];
            if let Some(Some(n)) = self.nodes.get(cur.0) {
                to_remove.extend_from_slice(&n.children);
            }
            i += 1;
        }

        if let Some(Some(n)) = self.nodes.get(id.0)
            && let Some(pid) = n.parent
            && let Some(Some(p)) = self.nodes.get_mut(pid.0)
        {
            p.children.retain(|&c| c != id);
        }

        self.roots.retain(|&r| r != id);

        self.count -= to_remove.len();
        for rid in to_remove {
            self.nodes[rid.0] = None;
            self.free.push(rid.0);
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)?.as_ref()
    }

    /// Remember to call [`Scene::update`] afterwards to refresh world transforms.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.0)?.as_mut()
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Recompute world transforms for every node, top-down.
    /// Call this once per frame after modifying any transforms.
    pub fn update(&mut self) {
        for i in 0..self.roots.len() {
            let root = self.roots[i];
            self.update_subtree(root, Mat4::IDENTITY);
        }
    }

    fn update_subtree(&mut self, id: NodeId, parent_world: Mat4) {
        let (world, children) = if let Some(Some(node)) = self.nodes.get(id.0) {
            (
                parent_world * node.transform.to_mat4(),
                node.children.clone(),
            )
        } else {
            return;
        };
        if let Some(Some(node)) = self.nodes.get_mut(id.0) {
            node.world_transform = world;
        }
        for child in children {
            self.update_subtree(child, world);
        }
    }

    /// Visit every live node in pre-order (parent before its children).
    pub fn walk<F: FnMut(NodeId, &Node)>(&self, mut f: F) {
        for &root in &self.roots {
            self.walk_subtree(root, &mut f);
        }
    }

    /// Visit every node that has an ECS [`Entity`] attached, in pre-order.
    ///
    /// Skips nodes whose [`Node::entity`] field is `None`. Use this to
    /// synchronise world transforms into ECS components each frame.
    ///
    /// # Example
    /// ```no_run
    /// # use nene::scene::{Scene, Node};
    /// # use nene::ecs::World;
    /// # struct Transform3d(nene::math::Mat4);
    /// # let mut scene = Scene::new();
    /// # let mut world = World::new();
    /// // After scene.update():
    /// scene.walk_entities(|_id, node, entity| {
    ///     if let Some(t) = world.get_mut::<Transform3d>(entity) {
    ///         t.0 = node.world_transform();
    ///     }
    /// });
    /// ```
    pub fn walk_entities<F: FnMut(NodeId, &Node, Entity)>(&self, mut f: F) {
        self.walk(|id, node| {
            if let Some(entity) = node.entity {
                f(id, node, entity);
            }
        });
    }

    fn walk_subtree<F: FnMut(NodeId, &Node)>(&self, id: NodeId, f: &mut F) {
        if let Some(Some(node)) = self.nodes.get(id.0) {
            f(id, node);
            for &child in &node.children {
                self.walk_subtree(child, f);
            }
        }
    }
}
