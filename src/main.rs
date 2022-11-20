mod camera;
mod display;
mod engine;
mod object;
mod world;

use std::time::Instant;

use display::Display;
use engine::Engine;
use tracing_subscriber::prelude::*;
use vulkano::image::ImageUsage;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    let event_loop = EventLoop::new();
    let mut display = Display::new(
        &event_loop,
        ImageUsage {
            color_attachment: true,
            transfer_destination: true,
            ..ImageUsage::none()
        },
    );
    let mut engine = Engine::new(display.queue(), display.swapchain_image_format());

    let mut t = Instant::now();
    event_loop.run(move |event, _, control_flow: &mut ControlFlow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                display.resize();
            }
            Event::RedrawEventsCleared => {
                let future = display.begin_frame();

                match future {
                    Ok(future) => {
                        let current_image = display.current_image();

                        let future = engine.render(current_image, future);

                        display.end_frame(future);
                    }
                    Err(e) => {
                        eprintln!("Error on begin frame: {e}");
                        return;
                    }
                }
            }
            _ => (),
        }

        engine.handle_events(event);
        engine.update(t.elapsed());
        t = Instant::now();
    });
}
