use std::{cell::Cell, collections::HashMap, rc::Rc};

use cgmath::{InnerSpace, Point2, Point3, Vector3};

use crate::object::{cube::Cube, InstancesMesh};

const Y_STRIDE: i32 = 16;
const Z_STRIDE: i32 = 16 * 256;

const fn index_to_chunk_pos(i: usize) -> Point3<i32> {
    Point3::new(
        (i % 16) as i32,
        ((i / 16) % 256) as i32,
        (i / 16 / 256) as i32,
    )
}

const fn chunk_pos_to_index(chunk_pos: Point3<i32>) -> usize {
    (chunk_pos.x + chunk_pos.y * Y_STRIDE + chunk_pos.z * Z_STRIDE) as usize
}

#[derive(Clone, Copy)]
pub(crate) struct ChunkCube {
    color: [f32; 4],
    rotation: [f32; 3],
}

pub(crate) struct Chunk {
    start: Point2<i32>,
    cubes: Box<[Option<ChunkCube>; 16 * 256 * 16]>,

    mesh: InstancesMesh<Cube>,
    dirty: bool,
    world_dirty_ref: Rc<Cell<bool>>,
}

impl Chunk {
    fn new(start: Point2<i32>, world_dirty_ref: Rc<Cell<bool>>) -> Self {
        world_dirty_ref.set(true);
        Self {
            cubes: Box::new([None; 16 * 256 * 16]),
            start,

            mesh: InstancesMesh::new().unwrap(),
            dirty: true,
            world_dirty_ref,
        }
    }

    fn in_relative_chunk_pos(&self, pos: Point3<i32>) -> Point3<i32> {
        pos - Vector3::new(self.start.x, 0, self.start.y)
    }

    fn in_chunk_pos(&self, pos: Point3<i32>) -> Option<Point3<i32>> {
        let chunk_pos = self.in_relative_chunk_pos(pos);

        if chunk_pos.x >= 0
            && chunk_pos.x < 16
            && chunk_pos.y >= 0
            && chunk_pos.y < 256
            && chunk_pos.z >= 0
            && chunk_pos.z < 16
        {
            Some(chunk_pos)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn start(&self) -> &Point2<i32> {
        &self.start
    }

    pub fn push_cube(&mut self, cube: Cube) {
        let position = cube.center.cast::<i32>().unwrap();
        // must be inside the chunk
        let chunk_position = self.in_chunk_pos(position).unwrap();

        let index = chunk_pos_to_index(chunk_position);

        self.cubes[index] = Some(ChunkCube {
            color: cube.color,
            rotation: cube.rotation,
        });

        self.dirty = true;
        self.world_dirty_ref.set(true);
    }

    pub fn remove_cube(&mut self, pos: Point3<i32>) {
        // must be inside the chunk
        let chunk_position = self.in_chunk_pos(pos).unwrap();

        let index = chunk_pos_to_index(chunk_position);

        self.cubes[index] = None;
        self.dirty = true;
        self.world_dirty_ref.set(true);
    }

    fn add_to_mesh(&mut self, mesh: &mut InstancesMesh<Cube>) {
        if self.dirty {
            self.mesh = InstancesMesh::new().unwrap();
            self.dirty = false;

            for (i, cube) in self.cubes.iter().enumerate() {
                if let Some(cube) = cube {
                    let chunk_pos = index_to_chunk_pos(i);

                    let is_edge = chunk_pos.x == 0
                        || chunk_pos.x == 15
                        || chunk_pos.y == 0
                        || chunk_pos.y == 255
                        || chunk_pos.z == 0
                        || chunk_pos.z == 15;

                    // if cubes on all sides are present, don't draw this one
                    if is_edge
                        || self.cubes[i - 1].is_none()
                        || self.cubes[i + 1].is_none()
                        || self.cubes[i - Y_STRIDE as usize].is_none()
                        || self.cubes[i + Y_STRIDE as usize].is_none()
                        || self.cubes[i - Z_STRIDE as usize].is_none()
                        || self.cubes[i + Z_STRIDE as usize].is_none()
                    {
                        let pos = chunk_pos + Vector3::new(self.start.x, 0, self.start.y);
                        self.mesh.append_instance(&Cube {
                            center: pos.cast().unwrap(),
                            color: cube.color,
                            rotation: cube.rotation,
                        });
                    }
                }
            }
        }

        mesh.extend_mesh(&self.mesh);
    }

    #[allow(dead_code)]
    pub fn cubes(&self) -> impl Iterator<Item = Point3<i32>> + '_ {
        self.cubes.iter().enumerate().filter_map(|(i, cube)| {
            if cube.is_some() {
                let chunk_pos = index_to_chunk_pos(i);
                let pos = chunk_pos + Vector3::new(self.start.x, 0, self.start.y);
                Some(pos)
            } else {
                None
            }
        })
    }

    /// Returns cubes around the given position with the given radius
    #[allow(dead_code)]
    pub fn cubes_around(
        &self,
        pos: Point3<i32>,
        radius: f32,
    ) -> impl Iterator<Item = Point3<i32>> + '_ {
        let mut cubes = Vec::new();

        let chunk_pos = self.in_relative_chunk_pos(pos);

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
                    let index = chunk_pos_to_index(Point3::new(x, y, z));
                    if self.cubes[index].is_some() {
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

    pub fn cube_looking_at(
        &self,
        origin: &Point3<f32>,
        direction: &Vector3<f32>,
        max_radius: f32,
    ) -> Option<Point3<i32>> {
        let chunk_pos = self.in_relative_chunk_pos(origin.cast::<i32>().unwrap());
        let direction = direction.normalize();

        let inside = chunk_pos.x >= 0
            && chunk_pos.x < 16
            && chunk_pos.y >= 0
            && chunk_pos.y < 256
            && chunk_pos.z >= 0
            && chunk_pos.z < 16;

        if inside {
            return self.cube_looking_at_inside(&origin, &direction, max_radius);
        }

        None
    }

    // Reference: https://playtechs.blogspot.com/2007/03/raytracing-on-grid.html
    /// `origin` must be inside the chunk, and that is guaranteed by the caller
    fn cube_looking_at_inside(
        &self,
        origin: &Point3<f32>,
        direction: &Vector3<f32>,
        max_radius: f32,
    ) -> Option<Point3<i32>> {
        let origin_i32 = origin.map(|a| a.round() as i32);
        let max_radius_i32 = max_radius.ceil() as i32;
        let mut current = origin_i32;

        let dt_dx = 1. / direction.x.abs();
        let dt_dy = 1. / direction.y.abs();
        let dt_dz = 1. / direction.z.abs();

        let mut t_next_x;
        let mut t_next_y;
        let mut t_next_z;
        let inc_x = if direction.x == 0. {
            t_next_x = std::f32::INFINITY;
            0
        } else if direction.x < 0. {
            t_next_x = (origin.x + 0.5 - origin.x.round()) * dt_dx;
            -1
        } else {
            t_next_x = (origin.x.round() + 0.5 - origin.x) * dt_dx;
            1
        };
        let inc_y = if direction.y == 0. {
            t_next_y = std::f32::INFINITY;
            0
        } else if direction.y < 0. {
            t_next_y = (origin.y + 0.5 - origin.y.round()) * dt_dy;
            -1
        } else {
            t_next_y = (origin.y.round() + 0.5 - origin.y) * dt_dy;
            1
        };
        let inc_z = if direction.z == 0. {
            t_next_z = std::f32::INFINITY;
            0
        } else if direction.z < 0. {
            t_next_z = (origin.z + 0.5 - origin.z.round()) * dt_dz;
            -1
        } else {
            t_next_z = (origin.z.round() + 0.5 - origin.z) * dt_dz;
            1
        };

        let inc = Vector3::new(inc_x, inc_y, inc_z);

        loop {
            if let Some(chunk_pos) = self.in_chunk_pos(current) {
                let index = chunk_pos_to_index(chunk_pos);
                if self.cubes[index].is_some() {
                    return Some(current);
                }
            } else {
                break;
            }

            if t_next_x < t_next_y {
                if t_next_x < t_next_z {
                    current.x += inc.x;
                    t_next_x += dt_dx;
                } else {
                    current.z += inc.z;
                    t_next_z += dt_dz;
                }
            } else {
                if t_next_y < t_next_z {
                    current.y += inc.y;
                    t_next_y += dt_dy;
                } else {
                    current.z += inc.z;
                    t_next_z += dt_dz;
                }
            }

            let distance = (current - origin_i32).magnitude2(); // squared distance
            if distance > max_radius_i32 * max_radius_i32 {
                return None;
            }
        }

        None
    }
}

pub(crate) struct World {
    chunks: HashMap<(i32, i32), Chunk>,

    mesh: InstancesMesh<Cube>,
    dirty: Rc<Cell<bool>>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            mesh: InstancesMesh::new().unwrap(),
            dirty: Rc::new(Cell::new(false)),
        }
    }
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
    #[allow(dead_code)]
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

    pub fn cube_looking_at(
        &self,
        origin: &Point3<f32>,
        direction: &Vector3<f32>,
        max_radius: f32,
    ) -> Option<Point3<i32>> {
        let chunk_containing_origin = (
            (origin.x.round() as i32) / 16 * 16,
            (origin.z.round() as i32) / 16 * 16,
        );

        // check current chunk
        if let Some(chunk) = self.chunks.get(&chunk_containing_origin) {
            if let Some(cube) = chunk.cube_looking_at(origin, direction, max_radius) {
                return Some(cube);
            }
        }

        None
    }
}

impl World {
    pub(crate) fn mesh(&mut self) -> &InstancesMesh<Cube> {
        if self.dirty.get() {
            self.mesh = InstancesMesh::new().unwrap();

            for chunk in self.chunks.values_mut() {
                chunk.add_to_mesh(&mut self.mesh);
            }
            self.dirty.set(false);
        }

        &self.mesh
    }
}
