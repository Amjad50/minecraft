use std::{cell::Cell, collections::HashMap, rc::Rc};

use cgmath::{InnerSpace, Point2, Point3, Vector3};

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

    cube_mesh: CubeMesh,
    dirty: bool,
    world_dirty_ref: Rc<Cell<bool>>,
}

impl Chunk {
    fn new(start: Point2<i32>, world_dirty_ref: Rc<Cell<bool>>) -> Self {
        world_dirty_ref.set(true);
        Self {
            cubes: Box::new([None; 16 * 256 * 16]),
            start,

            cube_mesh: CubeMesh::default(),
            dirty: true,
            world_dirty_ref,
        }
    }

    #[allow(dead_code)]
    pub fn start(&self) -> &Point2<i32> {
        &self.start
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

        self.cubes[index as usize] = Some(ChunkCube {
            color: cube.color,
            rotation: cube.rotation,
        });

        self.dirty = true;
        self.world_dirty_ref.set(true);
    }

    pub fn remove_cube(&mut self, pos: Point3<i32>) {
        let chunk_position = pos - Vector3::new(self.start.x, 0, self.start.y);

        assert!(
            chunk_position.x >= 0
                && chunk_position.x < 16
                && chunk_position.y >= 0
                && chunk_position.y < 256
                && chunk_position.z >= 0
                && chunk_position.z < 16
        );

        let index = chunk_position.x + chunk_position.y * 16 + chunk_position.z * 16 * 256;

        self.cubes[index as usize] = None;
        self.dirty = true;
        self.world_dirty_ref.set(true);
    }

    fn add_to_mesh(&mut self, mesh: &mut CubeMesh) {
        if self.dirty {
            self.cube_mesh = CubeMesh::default();
            self.dirty = false;

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
                    self.cube_mesh.add_cube(&Cube {
                        center: pos.cast().unwrap(),
                        color: cube.color,
                        rotation: cube.rotation,
                    });
                }
            }
        }

        mesh.add_mesh(&self.cube_mesh);
    }

    #[allow(dead_code)]
    pub fn cubes(&self) -> impl Iterator<Item = Point3<i32>> + '_ {
        self.cubes.iter().enumerate().filter_map(|(i, cube)| {
            if cube.is_some() {
                let chunk_pos = Point3::new(
                    (i % 16) as i32,
                    ((i / 16) % 256) as i32,
                    (i / 16 / 256) as i32,
                );
                let pos = chunk_pos + Vector3::new(self.start.x, 0, self.start.y);
                Some(pos)
            } else {
                None
            }
        })
    }

    pub fn cubes_around(
        &self,
        pos: Point3<i32>,
        radius: f32,
    ) -> impl Iterator<Item = Point3<i32>> + '_ {
        let mut cubes = Vec::new();

        let chunk_pos = pos - Vector3::new(self.start.x, 0, self.start.y);

        // get the size of the cube around pos with radius
        let area_cube_radius = radius.ceil() as i32;
        let min_x = (chunk_pos.x - area_cube_radius).max(0);
        let max_x = (chunk_pos.x + area_cube_radius).min(15);
        let min_y = (chunk_pos.y - area_cube_radius).max(0);
        let max_y = (chunk_pos.y + area_cube_radius).min(255);
        let min_z = (chunk_pos.z - area_cube_radius).max(0);
        let max_z = (chunk_pos.z + area_cube_radius).min(15);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    let index = x + y * 16 + z * 16 * 256;
                    if self.cubes[index as usize].is_some() {
                        // is inside radius
                        let cube_pos =
                            Point3::new(x, y, z) + Vector3::new(self.start.x, 0, self.start.y);
                        if (pos - cube_pos).cast::<f32>().unwrap().magnitude() <= radius {
                            cubes.push(
                                Point3::new(x, y, z) + Vector3::new(self.start.x, 0, self.start.y),
                            );
                        }
                    }
                }
            }
        }

        cubes.into_iter()
    }
}

#[derive(Default)]
pub(crate) struct World {
    chunks: HashMap<(i32, i32), Chunk>,

    cube_mesh: CubeMesh,
    dirty: Rc<Cell<bool>>,
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
            .or_insert_with(|| Chunk::new(chunk_id.into(), self.dirty.clone()))
            .push_cube(block);
    }

    #[allow(dead_code)]
    pub fn remove_cube(&mut self, pos: Point3<i32>) {
        assert!(pos.y >= 0);
        let chunk_id = ((pos.x as i32) / 16 * 16, (pos.z as i32) / 16 * 16);
        let chunk = self
            .chunks
            .entry(chunk_id)
            .or_insert_with(|| Chunk::new(chunk_id.into(), self.dirty.clone()));

        chunk.remove_cube(pos);
    }

    pub fn create_chunk(&mut self, x: i32, y: u32, z: i32, color: [f32; 4]) {
        let start_x = (x / 16) * 16;
        let start_y = y;
        let start_z = (z / 16) * 16;

        let chunk_id = (start_x, start_z);
        let mut chunk = Chunk::new(chunk_id.into(), self.dirty.clone());

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
        self.dirty.set(true);
    }

    #[allow(dead_code)]
    pub fn chunks(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks.values()
    }

    #[allow(dead_code)]
    pub fn chunks_around(&self, pos: Point2<i32>, radius: f32) -> impl Iterator<Item = &Chunk> {
        let mut chunks = Vec::new();

        let chunk_containing_pos = (pos.x / 16 * 16, pos.y / 16 * 16);

        let radius_chunks = (radius / 16.).ceil() as i32;

        for x in -radius_chunks..=radius_chunks {
            for y in -radius_chunks..=radius_chunks {
                let chunk_id = (
                    chunk_containing_pos.0 + x * 16,
                    chunk_containing_pos.1 + y * 16,
                );
                if let Some(chunk) = self.chunks.get(&chunk_id) {
                    chunks.push(chunk);
                }
            }
        }

        chunks.into_iter()
    }

    /// Since we can't create a mut iterator easily because of lifetimes errors,
    /// we used callback function to mutate chunks if needed.
    pub fn chunks_around_mut_callback(
        &mut self,
        pos: Point2<i32>,
        radius: f32,
        f: impl Fn(&mut Chunk),
    ) {
        let chunk_containing_pos = (pos.x / 16 * 16, pos.y / 16 * 16);
        let radius_chunks = (radius / 16.).ceil() as i32;

        for x in -radius_chunks..=radius_chunks {
            for y in -radius_chunks..=radius_chunks {
                let chunk_id = (
                    chunk_containing_pos.0 + x * 16,
                    chunk_containing_pos.1 + y * 16,
                );
                if let Some(chunk) = self.chunks.get_mut(&chunk_id) {
                    f(chunk);
                }
            }
        }
    }
}

impl World {
    pub(crate) fn mesh(&mut self) -> &InstancesMesh {
        if self.dirty.get() {
            self.cube_mesh = CubeMesh::default();

            for chunk in self.chunks.values_mut() {
                chunk.add_to_mesh(&mut self.cube_mesh);
            }
            self.dirty.set(false);
        }

        self.cube_mesh.mesh()
    }
}
