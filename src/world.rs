use std::collections::HashMap;

use cgmath::{Point2, Point3, Vector3};

use crate::object::{
    cube::{Cube, CubeMesh},
    InstancesMesh,
};

#[derive(Clone, Copy)]
pub(crate) struct ChunkCube {
    color: [f32; 4],
    rotation: [f32; 3],
}

pub(crate) struct Chunk {
    start: Point2<i32>,
    cubes: Box<[Option<ChunkCube>; 16 * 256 * 16]>,
}

impl Chunk {
    fn new(start: Point2<i32>) -> Self {
        Self {
            cubes: Box::new([None; 16 * 256 * 16]),
            start,
        }
    }

    pub fn push_cube(&mut self, cube: Cube) {
        let position = cube.center.cast::<i32>().unwrap();
        let chunk_position = position - Vector3::new(self.start.x, 0, self.start.y);

        assert!(
            chunk_position.x >= 0
                && chunk_position.x < 16
                && chunk_position.y >= 0
                && chunk_position.y < 256
                && chunk_position.z >= 0
                && chunk_position.z < 16
        );
        let index = chunk_position.x + chunk_position.y * 16 + chunk_position.z * 16 * 256;

        assert!(!index.is_negative());

        self.cubes[index as usize] = Some(ChunkCube {
            color: cube.color,
            rotation: cube.rotation,
        });
    }

    pub fn add_to_mesh(&self, mesh: &mut CubeMesh) {
        for (i, cube) in self.cubes.iter().enumerate() {
            if let Some(cube) = cube {
                let chunk_pos = Point3::new(
                    (i % 16) as i32,
                    ((i / 16) % 256) as i32,
                    (i / 16 / 256) as i32,
                );
                let is_edge = chunk_pos.x == 0
                    || chunk_pos.x == 15
                    || chunk_pos.y == 0
                    || chunk_pos.y == 255
                    || chunk_pos.z == 0
                    || chunk_pos.z == 15;

                // if cubes on all sides are present, don't draw this one
                if !is_edge
                    && self.cubes[i - 1].is_some()
                    && self.cubes[i + 1].is_some()
                    && self.cubes[i - 16].is_some()
                    && self.cubes[i + 16].is_some()
                    && self.cubes[i - 256].is_some()
                    && self.cubes[i + 256].is_some()
                {
                    continue;
                }

                let pos = chunk_pos + Vector3::new(self.start.x, 0, self.start.y);
                mesh.add_cube(&Cube {
                    center: pos.cast().unwrap(),
                    color: cube.color,
                    rotation: cube.rotation,
                });
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct World {
    chunks: HashMap<(i32, i32), Chunk>,

    cube_mesh: CubeMesh,
    dirty: bool,
}

impl World {
    #[allow(dead_code)]
    pub fn push_cube(&mut self, block: Cube) {
        let chunk_id = (
            (block.center.x as i32) / 16 * 16,
            (block.center.z as i32) / 16 * 16,
        );
        self.chunks
            .entry(chunk_id)
            .or_insert(Chunk::new(chunk_id.into()))
            .push_cube(block);
        self.dirty = true;
    }

    pub fn create_chunk(&mut self, x: i32, y: u32, z: i32, color: [f32; 4]) {
        let start_x = (x / 16) * 16;
        let start_y = y;
        let start_z = (z / 16) * 16;

        let chunk_id = (start_x, start_z);
        let mut chunk = Chunk::new(chunk_id.into());

        for x in start_x..(start_x + 16) {
            for y in 0..start_y {
                for z in start_z..(start_z + 16) {
                    chunk.push_cube(Cube {
                        center: Point3::new(x, y as i32, z).cast().unwrap(),
                        color,
                        rotation: [0.0, 0.0, 0.0],
                    });
                }
            }
        }

        if self.chunks.insert(chunk_id, chunk).is_some() {
            eprintln!("WARN: Replacing chunk in {:?}", chunk_id);
        };

        self.dirty = true;
    }
}

impl World {
    pub(crate) fn mesh(&mut self) -> &InstancesMesh {
        if self.dirty {
            self.cube_mesh = CubeMesh::default();

            for chunk in self.chunks.values() {
                chunk.add_to_mesh(&mut self.cube_mesh);
            }
            self.dirty = false;
        }

        &self.cube_mesh.mesh()
    }
}
