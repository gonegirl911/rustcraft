use super::Renderer;
use bytemuck::Pod;
use std::{cmp::Reverse, marker::PhantomData, mem};
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub struct Mesh<V> {
    vertex_buffer: wgpu::Buffer,
    phantom: PhantomData<V>,
}

impl<V: Vertex> Mesh<V> {
    pub fn from_data(Renderer { device, .. }: &Renderer, vertices: &[V]) -> Self {
        Self {
            vertex_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            phantom: PhantomData,
        }
    }

    fn uninit_mut(Renderer { device, .. }: &Renderer, len: usize) -> Self {
        Self {
            vertex_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (len * mem::size_of::<V>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            phantom: PhantomData,
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.len(), 0..1);
    }

    fn write(&self, Renderer { queue, .. }: &Renderer, vertices: &[V]) {
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
    }

    fn len(&self) -> u32 {
        (self.vertex_buffer.size() / mem::size_of::<V>() as u64) as u32
    }
}

pub struct TransparentMesh<C, V> {
    vertices: Vec<(C, [V; 3])>,
    mesh: Mesh<V>,
}

impl<C, V: Vertex> TransparentMesh<C, V> {
    pub fn from_data<F>(renderer: &Renderer, vertices: &[V], mut coords: F) -> Self
    where
        F: FnMut([V; 3]) -> C,
    {
        Self {
            mesh: Mesh::uninit_mut(renderer, vertices.len()),
            vertices: vertices
                .chunks_exact(3)
                .map(|v| {
                    let v = v.try_into().unwrap_or_else(|_| unreachable!());
                    (coords(v), v)
                })
                .collect(),
        }
    }

    pub fn draw<'a, D, F>(
        &'a mut self,
        renderer: &Renderer,
        render_pass: &mut wgpu::RenderPass<'a>,
        mut dist: F,
    ) where
        D: Ord,
        F: FnMut(&C) -> D,
    {
        self.vertices.sort_by_key(|(c, _)| Reverse(dist(c)));
        self.mesh.write(
            renderer,
            &self
                .vertices
                .iter()
                .flat_map(|(_, v)| v)
                .copied()
                .collect::<Vec<_>>(),
        );
        self.mesh.draw(render_pass);
    }
}

pub struct IndexedMesh<V, I> {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    phantom: PhantomData<(V, I)>,
}

impl<V: Vertex, I: Index> IndexedMesh<V, I> {
    pub fn from_data(Renderer { device, .. }: &Renderer, vertices: &[V], indices: &[I]) -> Self {
        Self {
            vertex_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            index_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            phantom: PhantomData,
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), I::FORMAT);
        render_pass.draw_indexed(0..self.len(), 0, 0..1);
    }

    fn len(&self) -> u32 {
        (self.index_buffer.size() / mem::size_of::<I>() as u64) as u32
    }
}

pub trait Vertex: Pod {
    const ATTRIBS: &'static [wgpu::VertexAttribute];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

pub trait Index: Pod {
    const FORMAT: wgpu::IndexFormat;
}

impl Index for u16 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}

impl Index for u32 {
    const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}
