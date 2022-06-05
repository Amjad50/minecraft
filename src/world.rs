use cgmath::Point3;

use crate::object::{
    cube::{Cube, CubeMesh},
    InstancesMesh,
};

#[derive(Default)]
pub(crate) struct World {
    cubes: Vec<Cube>,

    cube_mesh: CubeMesh,
    dirty: bool,
}

impl World {
    pub fn push_cube(&mut self, block: Cube) {
        self.cubes.push(block);
        self.dirty = true;
    }

    pub fn create_chunk(&mut self, x: isize, y: usize, z: isize, color: [f32; 4]) {
        let start_x = (x / 16) * 16;
        let start_y = y;
        let start_z = (z / 16) * 16;

        for x in start_x..(start_x + 16) {
            for y in 0..start_y {
                for z in start_z..(start_z + 16) {
                    self.push_cube(Cube {
                        center: Point3::new(x, y as isize, z).cast().unwrap(),
                        color,
                        rotation: [0.0, 0.0, 0.0],
                    });
                }
            }
        }
    }
}

impl World {
    pub(crate) fn mesh(&mut self) -> &InstancesMesh {
        if self.dirty {
            self.cube_mesh = CubeMesh::default();

            for cube in &self.cubes {
                self.cube_mesh.add_cube(cube);
            }
        }

        &self.cube_mesh.mesh()
    }
}
