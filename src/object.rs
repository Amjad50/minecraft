use bytemuck::{Pod, Zeroable};
use vulkano::impl_vertex;

pub trait Object {
    fn to_mesh(&self, mesh: &mut Mesh);
}

pub struct Square {
    pub center_pos: [f32; 3],
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

impl Object for Square {
    fn to_mesh(&self, mesh: &mut Mesh) {
        let [x, y, z] = self.center_pos;
        let top_left = [x - 0.5, y - 0.5, z];
        let top_right = [x + 0.5, y - 0.5, z];
        let bottom_left = [x - 0.5, y + 0.5, z];
        let bottom_right = [x + 0.5, y + 0.5, z];
        let vertices = [
            Vertex {
                center_pos: self.center_pos,
                pos: top_left,
                color: self.color,
                rotation: self.rotation,
            },
            Vertex {
                center_pos: self.center_pos,
                pos: top_right,
                color: self.color,
                rotation: self.rotation,
            },
            Vertex {
                center_pos: self.center_pos,
                pos: bottom_left,
                color: self.color,
                rotation: self.rotation,
            },
            Vertex {
                center_pos: self.center_pos,
                pos: bottom_right,
                color: self.color,
                rotation: self.rotation,
            },
        ];

        let indices = [0, 1, 2, 1, 2, 3];

        mesh.append_vertices(&vertices, &indices);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub center_pos: [f32; 3],
    pub pos: [f32; 3],
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

impl_vertex!(Vertex, center_pos, pos, color, rotation);

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
