use std::{fmt, marker::PhantomData, sync::Arc};

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Rad};
use vulkano::{buffer::BufferUsage, device::Queue, impl_vertex};

use crate::buffers::MirroredBuffer;

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
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Instance {
    pub color: [f32; 4],
    pub translation: [f32; 3],
}

impl_vertex!(Instance, color, translation);

pub fn rotation_scale_matrix(rotation: [f32; 3], scale: f32) -> Matrix4<f32> {
    Matrix4::from(cgmath::Euler::new(
        Rad(rotation[0]),
        Rad(rotation[1]),
        Rad(rotation[2]),
    )) * Matrix4::from_scale(scale)
}

#[derive(Debug)]
pub enum InstancesMeshError {
    EmptyVertices,
    EmptyIndices,
}

impl std::error::Error for InstancesMeshError {}

impl fmt::Display for InstancesMeshError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InstancesMeshError::EmptyVertices => write!(f, "Cannot use mesh with no vertices"),
            InstancesMeshError::EmptyIndices => write!(f, "Cannot use mesh with no indices"),
        }
    }
}

pub trait Mesh {
    fn mesh() -> (Vec<Vertex>, Vec<u32>);
    fn to_instance(&self) -> Instance;
}

pub struct InstancesMesh<M: Mesh> {
    instances: Vec<Instance>,

    vertex_buffer: MirroredBuffer<Vertex>,
    index_buffer: MirroredBuffer<u32>,
    instance_buffer: MirroredBuffer<Instance>,

    phantom: PhantomData<M>,
}

impl<M: Mesh> InstancesMesh<M> {
    pub fn new(queue: &Arc<Queue>) -> Result<Self, InstancesMeshError> {
        let mesh = M::mesh();

        if mesh.0.is_empty() {
            return Err(InstancesMeshError::EmptyVertices);
        }
        if mesh.1.is_empty() {
            return Err(InstancesMeshError::EmptyIndices);
        }
        let vertices = mesh.0;
        let indices = mesh.1;

        let vertex_buffer = MirroredBuffer::from_iter(
            queue,
            2,
            BufferUsage::vertex_buffer(),
            vertices.iter().cloned(),
        );

        let index_buffer = MirroredBuffer::from_iter(
            queue,
            2,
            BufferUsage::index_buffer(),
            indices.iter().cloned(),
        );

        let instance_buffer = MirroredBuffer::from_iter(queue, 2, BufferUsage::vertex_buffer(), []);

        Ok(Self {
            instances: Vec::new(),
            vertex_buffer,
            index_buffer,
            instance_buffer,
            phantom: PhantomData,
        })
    }

    #[allow(dead_code)]
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn clear_instances(&mut self) {
        self.instances.clear();
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn append_instance(&mut self, instance: &M) {
        self.instances.push(instance.to_instance());
    }

    pub fn rebuild_instance_buffer(&mut self) {
        self.instance_buffer
            .update_data(self.instances.iter().cloned());
    }

    pub fn extend_mesh(&mut self, mesh: &Self) {
        self.instances.extend_from_slice(&mesh.instances);
    }

    pub fn vertex_buffer(&self) -> &MirroredBuffer<Vertex> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &MirroredBuffer<u32> {
        &self.index_buffer
    }

    pub fn instance_buffer(&self) -> &MirroredBuffer<Instance> {
        &self.instance_buffer
    }
}
