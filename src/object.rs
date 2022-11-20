use std::{fmt, marker::PhantomData};

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Rad};
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
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    instances: Vec<Instance>,

    phantom: PhantomData<M>,
}

impl<M: Mesh> InstancesMesh<M> {
    pub fn new() -> Result<Self, InstancesMeshError> {
        let mesh = M::mesh();

        if mesh.0.is_empty() {
            return Err(InstancesMeshError::EmptyVertices);
        }
        if mesh.1.is_empty() {
            return Err(InstancesMeshError::EmptyIndices);
        }

        Ok(Self {
            vertices: mesh.0,
            indices: mesh.1,
            instances: Vec::new(),
            phantom: PhantomData,
        })
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

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() || self.instances.is_empty()
    }

    pub fn append_instance(&mut self, instance: &M) {
        self.instances.push(instance.to_instance());
    }

    #[allow(dead_code)]
    pub fn extend_mesh(&mut self, mesh: &Self) {
        self.instances.extend_from_slice(&mesh.instances);
    }
}
