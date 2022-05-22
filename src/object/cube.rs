use cgmath::Point3;

use super::{Object, Vertex};

pub struct Cube {
    pub center: Point3<f32>,
    pub color: [f32; 4],
    pub rotation: [f32; 3],
}

impl Object for Cube {
    fn to_mesh(&self, mesh: &mut super::Mesh) {
        let translation = self.center.into();

        macro_rules! create_vertex {
            ($pos: expr) => {
                Vertex {
                    pos: $pos,
                    color: self.color,
                    rotation: self.rotation,
                    translation: translation,
                }
            };
        }

        // front
        let front_top_left = create_vertex!([-0.5, 0.5, -0.5]);
        let front_top_right = create_vertex!([0.5, 0.5, -0.5]);
        let front_bottom_left = create_vertex!([-0.5, -0.5, -0.5]);
        let front_bottom_right = create_vertex!([0.5, -0.5, -0.5]);

        // back
        let back_top_left = create_vertex!([-0.5, 0.5, 0.5]);
        let back_top_right = create_vertex!([0.5, 0.5, 0.5]);
        let back_bottom_left = create_vertex!([-0.5, -0.5, 0.5]);
        let back_bottom_right = create_vertex!([0.5, -0.5, 0.5]);

        // right
        let right_top_left = front_top_right;
        let right_top_right = back_top_right;
        let right_bottom_left = front_bottom_right;
        let right_bottom_right = back_bottom_right;

        // left
        let left_top_left = back_top_left;
        let left_top_right = front_top_left;
        let left_bottom_left = back_bottom_left;
        let left_bottom_right = front_bottom_left;

        // up
        let up_top_left = back_top_left;
        let up_top_right = back_top_right;
        let up_bottom_left = front_top_left;
        let up_bottom_right = front_top_right;

        // bottom
        let bottom_top_left = back_bottom_left;
        let bottom_top_right = back_bottom_right;
        let bottom_bottom_left = front_bottom_left;
        let bottom_bottom_right = front_bottom_right;

        let vertices = [
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
        let indices = [
            0, 1, 2, 1, 2, 3, // front
            4, 5, 6, 5, 6, 7, // back
            8, 9, 10, 9, 10, 11, // right
            12, 13, 14, 13, 14, 15, // left
            16, 17, 18, 17, 18, 19, // up
            20, 21, 22, 21, 22, 23, // bottom
        ];

        mesh.append_vertices(&vertices, &indices);
    }
}
