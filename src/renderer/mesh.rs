//! [`GpuMesh`] and [`InstancedMesh`] — GPU geometry wrappers for [`Material`].

use super::material::InstanceData;
use super::{Context, IndexBuffer, InstanceBuffer, RenderPass, VertexBuffer};
use crate::mesh::{Mesh, Vertex};

/// GPU mesh for use with [`Material`].
///
/// Upload vertex and index data once; pass a reference to every
/// [`Material`] render call instead of managing buffers separately.
/// For dynamic geometry (updated every frame), use [`update`](GpuMesh::update).
pub struct GpuMesh {
    pub(super) vbuf: VertexBuffer,
    pub(super) ibuf: Option<IndexBuffer>,
    count: u32,
}

impl GpuMesh {
    /// Upload vertex and index data to the GPU.
    /// Pass an empty `indices` slice for a non-indexed (triangle-list) draw.
    pub fn new(ctx: &mut Context, vertices: &[Vertex], indices: &[u32]) -> Self {
        let ibuf = (!indices.is_empty()).then(|| ctx.create_index_buffer(indices));
        Self {
            vbuf: ctx.create_vertex_buffer(vertices),
            ibuf,
            count: vertices.len() as u32,
        }
    }

    /// Upload a [`Mesh`]'s vertex and index data to the GPU.
    pub fn from_mesh(ctx: &mut Context, mesh: &Mesh) -> Self {
        Self::new(ctx, &mesh.vertices, &mesh.indices)
    }

    /// Re-upload vertex data for dynamic meshes. Replaces the vertex buffer each call.
    pub fn update(&mut self, ctx: &mut Context, vertices: &[Vertex]) {
        self.vbuf = ctx.create_vertex_buffer(vertices);
        self.count = vertices.len() as u32;
    }

    /// Bind the vertex buffer and issue the draw call.
    ///
    /// Uses indexed draw when indices were provided at creation, otherwise
    /// issues a plain draw for non-indexed triangle lists.
    pub fn draw(&self, pass: &mut RenderPass) {
        pass.set_vertex_buffer(0, &self.vbuf);
        if let Some(ibuf) = &self.ibuf {
            pass.draw_indexed(ibuf);
        } else {
            pass.draw(0..self.count);
        }
    }
}

/// GPU mesh for instanced rendering with [`Material::render_instanced`].
///
/// Bundles the shared geometry and the per-instance buffer.
/// Call [`update`](InstancedMesh::update) every frame with the new instance list.
pub struct InstancedMesh {
    pub(super) vbuf: VertexBuffer,
    pub(super) ibuf: IndexBuffer,
    pub(super) inst_buf: InstanceBuffer,
    count: u32,
}

impl InstancedMesh {
    pub fn new(
        ctx: &mut Context,
        vertices: &[Vertex],
        indices: &[u32],
        instances: &[InstanceData],
    ) -> Self {
        Self {
            vbuf: ctx.create_vertex_buffer(vertices),
            ibuf: ctx.create_index_buffer(indices),
            inst_buf: ctx.create_instance_buffer(instances),
            count: instances.len() as u32,
        }
    }

    /// Re-upload the instance list for the current frame.
    pub fn update(&mut self, ctx: &mut Context, instances: &[InstanceData]) {
        ctx.update_instance_buffer(&self.inst_buf, instances);
        self.count = instances.len() as u32;
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    /// Bind geometry + instance buffer and issue the instanced draw call.
    pub fn draw(&self, pass: &mut RenderPass) {
        pass.set_vertex_buffer(0, &self.vbuf);
        pass.set_instance_buffer(1, &self.inst_buf);
        pass.draw_indexed_instanced(&self.ibuf, self.count);
    }
}
