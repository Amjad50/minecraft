use cgmath::Point3;

use super::{Instance, InstancesMesh, Vertex};

pub struct Square {
    pub center: Point3<f32>,
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

fn create_square_vertices() -> ([Vertex; 4], [u32; 6]) {
    let top_left = [-0.5, -0.5, 0.];
    let top_right = [0.5, -0.5, 0.];
    let bottom_left = [-0.5, 0.5, 0.];
    let bottom_right = [0.5, 0.5, 0.];

    let normal = [0., 0., 1.];

    let vertices = [
        Vertex {
            pos: top_left,
            normal,
        },
        Vertex {
            pos: top_right,
            normal,
        },
        Vertex {
            pos: bottom_left,
            normal,
        },
        Vertex {
            pos: bottom_right,
            normal,
        },
    ];

    let indices = [0, 1, 2, 1, 2, 3];

    (vertices, indices)
}

pub struct SquareMesh {
    mesh: InstancesMesh,
}

impl Default for SquareMesh {
    fn default() -> Self {
        let (vertices, indices) = create_square_vertices();
        let mesh = InstancesMesh::with_vertices(&vertices, &indices);

        Self { mesh }
    }
}

impl SquareMesh {
    pub fn mesh(&self) -> &InstancesMesh {
        &self.mesh
    }

    pub fn add_square(&mut self, cube: &Square) {
        self.mesh.append_instance(Instance {
            translation: cube.center.into(),
            color: cube.color,
            rotation: cube.rotation,
        });
    }
}
