use bytemuck::{Pod, Zeroable};
use cgmath::{Point3, Vector3};
use vulkano::impl_vertex;

pub mod cube;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 3],

    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}

impl_vertex!(Vertex, pos, color, normal, rotation, translation);

#[derive(Default)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(n),
            indices: Vec::with_capacity(n),
        }
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn append_vertices(&mut self, vertices: &[Vertex], indices: &[u32]) {
        let current_vertices_len = self.vertices.len() as u32;
        self.indices
            .extend(indices.iter().map(|i| i + current_vertices_len));
        self.vertices.extend(vertices);
    }

    pub fn append(&mut self, obj: &dyn Object) {
        obj.to_mesh(self);
    }
}

pub trait Object {
    fn to_mesh(&self, mesh: &mut Mesh);
}

pub struct Square {
    pub normal: Vector3<f32>,
    pub center: Point3<f32>,
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

impl Object for Square {
    fn to_mesh(&self, mesh: &mut Mesh) {
        let translation = self.center.into();
        let rotation = self.rotation;
        let normal = self.normal.into();

        let top_left = [-0.5, -0.5, 0.];
        let top_right = [0.5, -0.5, 0.];
        let bottom_left = [-0.5, 0.5, 0.];
        let bottom_right = [0.5, 0.5, 0.];
        let vertices = [
            Vertex {
                pos: top_left,
                color: self.color,
                normal,
                rotation,
                translation,
            },
            Vertex {
                pos: top_right,
                color: self.color,
                normal,
                rotation,
                translation,
            },
            Vertex {
                pos: bottom_left,
                color: self.color,
                normal,
                rotation,
                translation,
            },
            Vertex {
                pos: bottom_right,
                color: self.color,
                normal,
                rotation,
                translation,
            },
        ];

        let indices = [0, 1, 2, 1, 2, 3];

        mesh.append_vertices(&vertices, &indices);
    }
}
