use bytemuck::{Pod, Zeroable};
use vulkano::impl_vertex;

pub mod cube;
#[allow(dead_code)]
pub mod square;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
}

impl_vertex!(Vertex, pos, normal);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Instance {
    pub color: [f32; 4],
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}
impl_vertex!(Instance, color, rotation, translation);

#[derive(Default)]
pub struct InstancesMesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    instances: Vec<Instance>,
}

impl InstancesMesh {
    pub fn with_vertices(vertices: &[Vertex], indices: &[u32]) -> Self {
        Self {
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            instances: Vec::new(),
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() || self.instances.is_empty()
    }

    pub fn append_instance(&mut self, instance: Instance) {
        self.instances.push(instance);
    }
}
