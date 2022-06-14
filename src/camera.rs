use std::f32::consts::PI;

use cgmath::{Angle, InnerSpace, Matrix3, Matrix4, Point3, Rad, SquareMatrix, Vector3};

const MIN_PITCH: Rad<f32> = Rad(-89.0 * PI / 180.0);
const MAX_PITCH: Rad<f32> = Rad(89.0 * PI / 180.0);

const MIN_FOV: Rad<f32> = Rad(1.0 * PI / 180.0);
const MAX_FOV: Rad<f32> = Rad(179.0 * PI / 180.0);

fn clamp_rad(rad: Rad<f32>, min: Rad<f32>, max: Rad<f32>) -> Rad<f32> {
    Rad(rad.0.clamp(min.0, max.0))
}

pub(crate) struct Camera {
    position: Point3<f32>,

    yaw: Rad<f32>,
    pitch: Rad<f32>,

    camera_front: Vector3<f32>,
    movement_axes: Matrix3<f32>,

    fov: Rad<f32>,
    aspect: f32,
    near: f32,
    far: f32,

    perspective: Matrix4<f32>,
    view: Matrix4<f32>,

    perspective_dirty: bool,
    view_dirty: bool,
}

impl Camera {
    pub fn new<F: Into<Rad<f32>>>(
        fov: F,
        aspect: f32,
        near: f32,
        far: f32,
        position: Point3<f32>,
    ) -> Camera {
        Camera {
            position,

            yaw: Rad(0.),
            pitch: Rad(0.),

            camera_front: Vector3::unit_z(),
            movement_axes: Matrix3::identity(),

            fov: clamp_rad(fov.into(), MIN_FOV, MAX_FOV),
            aspect,
            near,
            far,

            perspective: Matrix4::identity(),
            view: Matrix4::identity(),

            perspective_dirty: true,
            view_dirty: true,
        }
    }

    pub fn position(&self) -> &Point3<f32> {
        &self.position
    }

    pub fn direction(&self) -> &Vector3<f32> {
        &self.camera_front
    }

    pub fn reversed_depth_perspective(&mut self) -> cgmath::Matrix4<f32> {
        if self.perspective_dirty {
            // compute the focal length (1 / tan(fov / 2))
            let focal_length = (self.fov / 2.0).cot();

            // projection matrix, this uses reversed depth (near is 1, far is 0)
            // this matrix is transposed to work for the shader
            self.perspective = [
                [focal_length / self.aspect, 0.0, 0.0, 0.0],
                [0.0, -focal_length, 0.0, 0.0],
                [0.0, 0.0, self.near / (self.far - self.near), 1.0],
                [
                    0.0,
                    0.0,
                    (self.far * self.near) / (self.far - self.near),
                    0.0,
                ],
            ]
            .into();

            self.perspective_dirty = false;
        }

        self.perspective
    }

    pub fn view(&mut self) -> cgmath::Matrix4<f32> {
        if self.view_dirty {
            self.view = Matrix4::look_to_lh(self.position, self.camera_front, Vector3::unit_y());
            self.view_dirty = false;
        }
        self.view
    }
}

impl Camera {
    pub fn rotate_camera<P: Into<Rad<f32>>, Y: Into<Rad<f32>>>(&mut self, pitch: P, yaw: Y) {
        let yaw = yaw.into();
        let pitch = pitch.into();

        // TODO: need to subtract for some reason, would be better to
        //       stick with the euler rotation direction
        self.yaw -= yaw;
        self.pitch = clamp_rad(self.pitch + pitch, MIN_PITCH, MAX_PITCH);

        let mut front = Vector3::new(
            -self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        )
        .normalize();
        self.camera_front = front;
        // don't move up and down based on direction
        front.y = 0.;
        front = front.normalize();

        let up = Vector3::unit_y();
        let right = up.cross(front).normalize();
        self.movement_axes = Matrix3::from_cols(right, up, front);

        self.view_dirty = true;
    }

    pub fn move_camera(&mut self, direction: Vector3<f32>) {
        self.position += self.movement_axes * direction;
        self.view_dirty = true;
    }

    #[allow(dead_code)]
    pub fn set_position(&mut self, position: Point3<f32>) {
        self.position = position;
        self.view_dirty = true;
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        if self.aspect != aspect {
            self.aspect = aspect;
            self.perspective_dirty = true;
        }
    }

    pub fn zoom<F: Into<Rad<f32>>>(&mut self, delta: F) {
        let fov = clamp_rad(self.fov + delta.into(), MIN_FOV, MAX_FOV);

        if self.fov != fov {
            self.fov = fov;
            self.perspective_dirty = true;
        }
    }
}
