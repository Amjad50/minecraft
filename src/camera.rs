use std::f32::consts::PI;

use cgmath::{Angle, Deg, InnerSpace, Matrix3, Matrix4, Point3, Rad, SquareMatrix, Vector3};

const LOW_PITCH: Rad<f32> = Rad(-89.0 * PI / 180.0);
const HIGH_PITCH: Rad<f32> = Rad(89.0 * PI / 180.0);

pub(crate) struct Camera {
    position: Point3<f32>,

    yaw: Rad<f32>,
    pitch: Rad<f32>,

    camera_axes: Matrix3<f32>,

    fov: f32,
    aspect: f32,
    near: f32,
    far: f32,

    perspective: Matrix4<f32>,
    view: Matrix4<f32>,

    perspective_dirty: bool,
    view_dirty: bool,
}

impl Camera {
    pub fn new(fov: f32, aspect: f32, near: f32, far: f32, position: Point3<f32>) -> Camera {
        Camera {
            position,

            yaw: Rad(0.),
            pitch: Rad(0.),

            camera_axes: Matrix3::identity(),

            fov: fov.clamp(1.0, 179.0),
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

    pub fn reversed_depth_perspective(&mut self) -> cgmath::Matrix4<f32> {
        if self.perspective_dirty {
            // convert to radians
            let fov_rad: Rad<f32> = Deg(self.fov).into();
            // compute the focal length (1 / tan(fov / 2))
            let focal_length = (fov_rad / 2.0).cot();

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
            self.view = Matrix4::look_to_lh(self.position, self.camera_axes.z, self.camera_axes.y);
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
        self.pitch += pitch;

        if self.pitch > HIGH_PITCH {
            self.pitch = HIGH_PITCH;
        } else if self.pitch < LOW_PITCH {
            self.pitch = LOW_PITCH;
        }

        let front = Vector3::new(
            -self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        )
        .normalize();
        let up = Vector3::unit_y();
        let right = up.cross(front).normalize();
        self.camera_axes = Matrix3::from_cols(right, up, front);

        self.view_dirty = true;
    }

    pub fn move_camera(&mut self, direction: Vector3<f32>) {
        self.position += self.camera_axes * direction;
        self.view_dirty = true;
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        if self.aspect != aspect {
            self.aspect = aspect;
            self.perspective_dirty = true;
        }
    }

    pub fn zoom(&mut self, delta: f32) {
        let fov = (self.fov + delta).clamp(1.0, 179.0);

        if self.fov != fov {
            self.fov = fov;
            self.perspective_dirty = true;
        }
    }
}
