use std::{fmt, marker::PhantomData};

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
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Instance {
    pub color: [f32; 4],
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
    pub scale: f32,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            color: [0.; 4],
            rotation: [0.; 3],
            translation: [0.; 3],
            scale: 1.,
        }
    }
}

impl_vertex!(Instance, color, rotation, translation, scale);

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
