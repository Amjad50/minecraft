use std::{cell::Cell, collections::HashMap, rc::Rc};

use cgmath::{InnerSpace, Point2, Point3, Vector3};

use crate::object::{cube::Cube, InstancesMesh};

const Y_STRIDE: i32 = 16;
const Z_STRIDE: i32 = 16 * 256;

/// Helper function to convert an array index to a chunk position
const fn index_to_chunk_pos(i: usize) -> Point3<i32> {
    Point3::new(
        (i % 16) as i32,
        ((i / 16) % 256) as i32,
        (i / 16 / 256) as i32,
    )
}

/// Helper function to convert position inside a chunk to an array index
const fn chunk_pos_to_index(chunk_pos: Point3<i32>) -> usize {
    (chunk_pos.x + chunk_pos.y * Y_STRIDE + chunk_pos.z * Z_STRIDE) as usize
}

/// Helper function to convert point to the chunk that contains it
const fn chunk_id(pos: Point3<i32>) -> (i32, i32) {
    (pos.x.div_euclid(16) * 16, pos.z.div_euclid(16) * 16)
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
}

// --- Looking at section ---
#[derive(Debug)]
enum TraceChunkResult {
    /// A block was found at the position
    BlockFound(Point3<i32>, Vector3<i32>),
    /// We should move to the next chunk
    ChunkChange((i32, i32)),
    /// Radius exceeded without finding a block, abort search...
    ExceededRadius,
}

#[derive(Debug)]
pub struct CubeLookAt {
    pub cube: Point3<i32>,
    pub direction: Vector3<i32>,
}

#[derive(Debug)]
pub struct TraceResult {
    pub path: Vec<Point3<i32>>,
    pub result_cube: Option<CubeLookAt>,
}

/// A helper struct that allows tracing all blocks passing through a ray
/// from a position (possibly camera) and direction.
///
/// This goes through chunks as well
struct BlockRayTracer<'world> {
    world: &'world World,

    dt: Vector3<f32>,

    current_chunk: (i32, i32),
    chunk_inc_dir: (i32, i32),

    last_cube: Point3<i32>,
    current_cube: Point3<i32>,
    origin_cube_i32: Point3<i32>,
    cube_inc_dir: Vector3<i32>,
    t_next_cube: Vector3<f32>,

    max_radius_i32: i32,
    path: Vec<Point3<i32>>,
}

impl<'world> BlockRayTracer<'world> {
    pub fn new(
        world: &'world World,
        origin: &Point3<f32>,
        direction: &Vector3<f32>,
        max_radius: f32,
    ) -> Self {
        let direction = direction.normalize();

        let origin_cube_i32 = origin.map(|a| a.round() as i32);

        let origin_chunk = chunk_id(origin_cube_i32);

        let max_radius_i32 = max_radius.ceil() as i32;
        let current_chunk = origin_chunk;
        let current_cube = origin_cube_i32;

        let dt_dx = 1. / direction.x.abs();
        let dt_dy = 1. / direction.y.abs();
        let dt_dz = 1. / direction.z.abs();

        let (cube_inc_dir, t_next_cube) = {
            let t_next_x;
            let t_next_y;
            let t_next_z;
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
            (
                Vector3::new(inc_x, inc_y, inc_z),
                Vector3::new(t_next_x, t_next_y, t_next_z),
            )
        };

        let chunk_inc_dir = (cube_inc_dir.x * 16, cube_inc_dir.z * 16);

        Self {
            world,
            dt: Vector3::new(dt_dx, dt_dy, dt_dz),

            current_chunk,
            chunk_inc_dir,

            last_cube: current_cube,
            current_cube,
            origin_cube_i32,
            cube_inc_dir,
            t_next_cube,
            max_radius_i32,
            path: Vec::new(),
        }
    }

    fn move_to_next_cube(&mut self) -> Option<TraceChunkResult> {
        const fn chunk_change(dir: i32, val: i32) -> bool {
            (dir == -1 && val.rem_euclid(16) == 15) || (dir == 1 && val.rem_euclid(16) == 0)
        }

        self.last_cube = self.current_cube;

        if self.t_next_cube.x < self.t_next_cube.y {
            if self.t_next_cube.x < self.t_next_cube.z {
                self.current_cube.x += self.cube_inc_dir.x;
                self.t_next_cube.x += self.dt.x;
                if chunk_change(self.cube_inc_dir.x, self.current_cube.x) {
                    return Some(TraceChunkResult::ChunkChange((
                        self.current_chunk.0 + self.chunk_inc_dir.0,
                        self.current_chunk.1,
                    )));
                }
            } else {
                self.current_cube.z += self.cube_inc_dir.z;
                self.t_next_cube.z += self.dt.z;
                if chunk_change(self.cube_inc_dir.z, self.current_cube.z) {
                    return Some(TraceChunkResult::ChunkChange((
                        self.current_chunk.0,
                        self.current_chunk.1 + self.chunk_inc_dir.1,
                    )));
                }
            }
        } else if self.t_next_cube.y < self.t_next_cube.z {
            self.current_cube.y += self.cube_inc_dir.y;
            self.t_next_cube.y += self.dt.y;
        } else {
            self.current_cube.z += self.cube_inc_dir.z;
            self.t_next_cube.z += self.dt.z;
            if chunk_change(self.cube_inc_dir.z, self.current_cube.z) {
                return Some(TraceChunkResult::ChunkChange((
                    self.current_chunk.0,
                    self.current_chunk.1 + self.chunk_inc_dir.1,
                )));
            }
        }

        let distance = (self.current_cube - self.origin_cube_i32).magnitude2(); // squared distance

        if distance > self.max_radius_i32 * self.max_radius_i32 {
            Some(TraceChunkResult::ExceededRadius)
        } else {
            None
        }
    }

    // Reference: https://playtechs.blogspot.com/2007/03/raytracing-on-grid.html
    fn trace_chunk(&mut self, chunk: &Chunk) -> TraceChunkResult {
        loop {
            self.path.push(self.current_cube);

            // This will almost always be some, unless we are outside the `y`
            // range (0-255), then we should just follow the trace until we
            // get back on range.
            if let Some(chunk_pos) = chunk.in_chunk_pos(self.current_cube) {
                let index = chunk_pos_to_index(chunk_pos);
                if chunk.cubes[index].is_some() {
                    return TraceChunkResult::BlockFound(
                        self.current_cube,
                        self.last_cube - self.current_cube,
                    );
                }
            }

            if let Some(r) = self.move_to_next_cube() {
                return r;
            }
        }
    }

    fn trace_no_chunk(&mut self) -> TraceChunkResult {
        // TODO: maybe we can optimize this since we don't need
        //       to loop over all cubes
        loop {
            self.path.push(self.current_cube);

            if let Some(r) = self.move_to_next_cube() {
                return r;
            }
        }
    }

    pub fn run(mut self) -> TraceResult {
        let result = loop {
            let result = if let Some(chunk) = self.world.chunks.get(&self.current_chunk) {
                self.trace_chunk(chunk)
            } else {
                self.trace_no_chunk()
            };

            match result {
                TraceChunkResult::BlockFound(cube, direction) => {
                    break Some(CubeLookAt { cube, direction })
                }
                TraceChunkResult::ChunkChange(next_chunk) => {
                    self.current_chunk = next_chunk;
                }
                TraceChunkResult::ExceededRadius => break None,
            }
        };

        TraceResult {
            path: self.path,
            result_cube: result,
        }
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
        let chunk_id = chunk_id(block.center.cast().unwrap());
        self.chunks
            .entry(chunk_id)
            .or_insert_with(|| Chunk::new(chunk_id.into(), self.dirty.clone()))
            .push_cube(block);
    }

    #[allow(dead_code)]
    pub fn remove_cube(&mut self, pos: Point3<i32>) {
        assert!(pos.y >= 0);
        let chunk_id = chunk_id(pos.cast().unwrap());
        let chunk = self
            .chunks
            .entry(chunk_id)
            .or_insert_with(|| Chunk::new(chunk_id.into(), self.dirty.clone()));

        chunk.remove_cube(pos);
    }

    pub fn create_chunk(&mut self, x: i32, y: u32, z: i32, color: [f32; 4]) {
        let chunk_id = chunk_id(Point3::new(x, 0, z));
        let start_x = chunk_id.0;
        let start_y = y;
        let start_z = chunk_id.1;

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

        let chunk_containing_pos = chunk_id(Point3::new(pos.x, 0, pos.y));

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
        let chunk_containing_pos = chunk_id(Point3::new(pos.x, 0, pos.y));
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
    ) -> TraceResult {
        let tracer = BlockRayTracer::new(self, origin, direction, max_radius);

        tracer.run()
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
