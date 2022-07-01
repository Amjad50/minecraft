use cgmath::Point3;

use super::{Instance, Mesh, Vertex};

pub struct Square {
    pub center: Point3<f32>,
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

impl Mesh for Square {
    fn mesh() -> (Vec<Vertex>, Vec<u32>) {
        let top_left = [-0.5, -0.5, 0.];
        let top_right = [0.5, -0.5, 0.];
        let bottom_left = [-0.5, 0.5, 0.];
        let bottom_right = [0.5, 0.5, 0.];

        let normal = [0., 0., 1.];

        let vertices = vec![
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

        let indices = vec![0, 1, 2, 1, 2, 3];

        (vertices, indices)
    }

    fn to_instance(&self) -> Instance {
        Instance {
            translation: self.center.into(),
            color: self.color,
            rotation: self.rotation,
            ..Default::default()
        }
    }
}
