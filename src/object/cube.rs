use cgmath::Point3;

use super::{Instance, Mesh, Vertex};

pub struct Cube {
    pub center: Point3<f32>,
    pub color: [f32; 4],
}

impl Mesh for Cube {
    fn mesh() -> (Vec<Vertex>, Vec<u32>) {
        // creates a vertex with normal
        macro_rules! create_vertex {
            ($pos: expr, $normal: expr) => {
                Vertex {
                    pos: $pos,
                    normal: $normal,
                }
            };
            (copy $vec: expr, $normal: expr) => {
                Vertex {
                    pos: $vec.pos,
                    normal: $normal,
                }
            };
        }

        // front
        let normal = [0., 0., -1.];
        let front_top_left = create_vertex!([-0.5, 0.5, -0.5], normal);
        let front_top_right = create_vertex!([0.5, 0.5, -0.5], normal);
        let front_bottom_left = create_vertex!([-0.5, -0.5, -0.5], normal);
        let front_bottom_right = create_vertex!([0.5, -0.5, -0.5], normal);

        // back
        let normal = [0., 0., 1.];
        let back_top_left = create_vertex!([-0.5, 0.5, 0.5], normal);
        let back_top_right = create_vertex!([0.5, 0.5, 0.5], normal);
        let back_bottom_left = create_vertex!([-0.5, -0.5, 0.5], normal);
        let back_bottom_right = create_vertex!([0.5, -0.5, 0.5], normal);

        // right
        let normal = [1., 0., 0.];
        let right_top_left = create_vertex!(copy front_top_right, normal);
        let right_top_right = create_vertex!(copy back_top_right, normal);
        let right_bottom_left = create_vertex!(copy front_bottom_right, normal);
        let right_bottom_right = create_vertex!(copy back_bottom_right, normal);

        // left
        let normal = [-1., 0., 0.];
        let left_top_left = create_vertex!(copy back_top_left, normal);
        let left_top_right = create_vertex!(copy front_top_left, normal);
        let left_bottom_left = create_vertex!(copy back_bottom_left, normal);
        let left_bottom_right = create_vertex!(copy front_bottom_left, normal);

        // up
        let normal = [0., 1., 0.];
        let up_top_left = create_vertex!(copy back_top_left, normal);
        let up_top_right = create_vertex!(copy back_top_right, normal);
        let up_bottom_left = create_vertex!(copy front_top_left, normal);
        let up_bottom_right = create_vertex!(copy front_top_right, normal);

        // bottom
        let normal = [0., -1., 0.];
        let bottom_top_left = create_vertex!(copy back_bottom_left, normal);
        let bottom_top_right = create_vertex!(copy back_bottom_right, normal);
        let bottom_bottom_left = create_vertex!(copy front_bottom_left, normal);
        let bottom_bottom_right = create_vertex!(copy front_bottom_right, normal);

        let vertices = vec![
            // front
            front_top_left,
            front_top_right,
            front_bottom_left,
            front_bottom_right,
            // back
            back_top_left,
            back_top_right,
            back_bottom_left,
            back_bottom_right,
            // right
            right_top_left,
            right_top_right,
            right_bottom_left,
            right_bottom_right,
            // left
            left_top_left,
            left_top_right,
            left_bottom_left,
            left_bottom_right,
            // up
            up_top_left,
            up_top_right,
            up_bottom_left,
            up_bottom_right,
            // bottom
            bottom_top_left,
            bottom_top_right,
            bottom_bottom_left,
            bottom_bottom_right,
        ];

        // we have all distinct 24 vertices, just to make it easier later
        // to apply texture to specific face only
        let indices = vec![
            0, 1, 2, 1, 2, 3, // front
            4, 5, 6, 5, 6, 7, // back
            8, 9, 10, 9, 10, 11, // right
            12, 13, 14, 13, 14, 15, // left
            16, 17, 18, 17, 18, 19, // up
            20, 21, 22, 21, 22, 23, // bottom
        ];

        (vertices, indices)
    }

    fn to_instance(&self) -> Instance {
        Instance {
            translation: self.center.into(),
            color: self.color,
        }
    }
}
