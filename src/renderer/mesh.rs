//! [`GpuMesh`] — GPU geometry wrapper for [`Material`](crate::renderer::Material).

use super::material::InstanceData;
use super::{Context, IndexBuffer, InstanceBuffer, RenderPass, VertexBuffer};
use crate::mesh::{Mesh, Vertex};

/// GPU mesh for use with [`Material`](crate::renderer::Material).
///
/// Upload vertex and index data once; pass a reference to every
/// [`Material`](crate::renderer::Material) render call instead of managing buffers separately.
/// For dynamic geometry (updated every frame), use [`update`](GpuMesh::update).
/// For instanced rendering, use [`with_instances`](GpuMesh::with_instances) and
/// [`update_instances`](GpuMesh::update_instances).
pub struct GpuMesh {
    pub(super) vbuf: VertexBuffer,
    pub(super) ibuf: Option<IndexBuffer>,
    pub(super) inst_buf: Option<InstanceBuffer>,
    vertex_count: u32,
    inst_count: u32,
}

impl GpuMesh {
    /// Upload vertex and index data to the GPU.
    /// Pass an empty `indices` slice for a non-indexed (triangle-list) draw.
    pub fn new(ctx: &mut Context, vertices: &[Vertex], indices: &[u32]) -> Self {
        let ibuf = (!indices.is_empty()).then(|| ctx.create_index_buffer(indices));
        Self {
            vbuf: ctx.create_vertex_buffer(vertices),
            ibuf,
            inst_buf: None,
            vertex_count: vertices.len() as u32,
            inst_count: 0,
        }
    }

    /// Upload a [`Mesh`]'s vertex and index data to the GPU.
    pub fn from_mesh(ctx: &mut Context, mesh: &Mesh) -> Self {
        Self::new(ctx, &mesh.vertices, &mesh.indices)
    }

    /// Upload vertex, index, and instance data for instanced rendering.
    pub fn with_instances(
        ctx: &mut Context,
        vertices: &[Vertex],
        indices: &[u32],
        instances: &[InstanceData],
    ) -> Self {
        let mut mesh = Self::new(ctx, vertices, indices);
        mesh.inst_buf = Some(ctx.create_instance_buffer(instances));
        mesh.inst_count = instances.len() as u32;
        mesh
    }

    /// Re-upload vertex data for dynamic meshes.
    pub fn update(&mut self, ctx: &mut Context, vertices: &[Vertex]) {
        self.vbuf = ctx.create_vertex_buffer(vertices);
        self.vertex_count = vertices.len() as u32;
    }

    /// Re-upload the instance list for the current frame.
    pub fn update_instances(&mut self, ctx: &mut Context, instances: &[InstanceData]) {
        if let Some(buf) = &self.inst_buf {
            ctx.update_instance_buffer(buf, instances);
        } else {
            self.inst_buf = Some(ctx.create_instance_buffer(instances));
        }
        self.inst_count = instances.len() as u32;
    }

    pub fn instance_count(&self) -> u32 {
        self.inst_count
    }

    pub(crate) fn draw(&self, pass: &mut RenderPass) {
        pass.set_vertex_buffer(0, &self.vbuf);
        if let Some(ibuf) = &self.ibuf {
            pass.draw_indexed(ibuf);
        } else {
            pass.draw(0..self.vertex_count);
        }
    }

    pub(crate) fn draw_instanced(&self, pass: &mut RenderPass) {
        if let (Some(ibuf), Some(inst_buf)) = (&self.ibuf, &self.inst_buf) {
            pass.set_vertex_buffer(0, &self.vbuf);
            pass.set_instance_buffer(1, inst_buf);
            pass.draw_indexed_instanced(ibuf, self.inst_count);
        }
    }
}
