mod display;

use display::Display;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    format::ClearValue,
    image::ImageUsage,
    swapchain::AcquireError,
    sync::{self, FlushError, GpuFuture},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    let event_loop = EventLoop::new();
    let mut display = Display::new(
        &event_loop,
        ImageUsage {
            color_attachment: true,
            transfer_destination: true,
            ..ImageUsage::none()
        },
    );

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(display.device()).boxed());
    let mut r = 1.0;
    let mut r_inc = 0.006;
    let mut g = 0.0;
    let mut g_inc = 0.01;
    let mut b = 0.0;
    let mut b_inc = 0.015;
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
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                r += r_inc;
                g += g_inc;
                b += b_inc;

                if r > 1.0 || r < 0.0 {
                    r_inc = -r_inc;
                }
                if g > 1.0 || g < 0.0 {
                    g_inc = -g_inc;
                }
                if b > 1.0 || b < 0.0 {
                    b_inc = -b_inc;
                }

                // Do not draw frame when screen dimensions are zero.
                // On Windows, this can occur from minimizing the application.
                if display.is_empty() {
                    return;
                }

                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    display.recreate_swapchains();
                    recreate_swapchain = false;
                }

                let (image_num, suboptimal, acquire_future) =
                    match vulkano::swapchain::acquire_next_image(display.swapchain(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                let mut builder = AutoCommandBufferBuilder::primary(
                    display.device(),
                    display.queue().family(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();

                builder
                    .clear_color_image(
                        display.swapchain_image(image_num).unwrap(),
                        ClearValue::Float([r, g, b, 1.0]),
                    )
                    .unwrap();

                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(display.queue(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(display.queue(), display.swapchain(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(display.device()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(display.device()).boxed());
                    }
                }
            }
            _ => (),
        }
    });
}
