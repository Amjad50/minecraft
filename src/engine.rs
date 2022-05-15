use std::{sync::Arc, time::Duration};

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    device::Queue,
    format::ClearValue,
    image::ImageAccess,
    sync::GpuFuture,
};
use winit::event::Event;

/// MC engine and renderer (for now)
pub(crate) struct Engine {
    queue: Arc<Queue>,

    r: f32,
    r_inc: f32,
    g: f32,
    g_inc: f32,
    b: f32,
    b_inc: f32,
}

impl Engine {
    pub fn new(queue: Arc<Queue>) -> Self {
        Self {
            queue,

            r: 1.0,
            r_inc: 0.006,
            g: 0.0,
            g_inc: 0.01,
            b: 0.0,
            b_inc: 0.015,
        }
    }

    pub fn handle_events(&mut self, _event: Event<()>) {}

    pub fn update(&mut self, delta: Duration) {
        self.r += self.r_inc * delta.as_secs_f32() * 100.;
        self.g += self.g_inc * delta.as_secs_f32() * 100.;
        self.b += self.b_inc * delta.as_secs_f32() * 100.;

        if self.r > 1.0 || self.r < 0.0 {
            self.r_inc = -self.r_inc;
        }
        if self.g > 1.0 || self.g < 0.0 {
            self.g_inc = -self.g_inc;
        }
        if self.b > 1.0 || self.b < 0.0 {
            self.b_inc = -self.b_inc;
        }
    }

    pub fn render<Fin>(&mut self, image: Arc<dyn ImageAccess>, future: Fin) -> Box<dyn GpuFuture>
    where
        Fin: GpuFuture + 'static,
    {
        let mut builder = AutoCommandBufferBuilder::primary(
            self.queue.device().clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .clear_color_image(image, ClearValue::Float([self.r, self.g, self.b, 1.0]))
            .unwrap();

        let command_buffer = builder.build().unwrap();

        future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .boxed()
    }
}
