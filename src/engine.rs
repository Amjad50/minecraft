use std::{sync::Arc, time::Duration};

use cgmath::{Deg, Point2, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    descriptor_set::{SingleLayoutDescSetPool, WriteDescriptorSet},
    device::Queue,
    format::{ClearValue, Format},
    image::{view::ImageView, AttachmentImage, ImageAccess},
    pipeline::{
        graphics::{
            depth_stencil::{CompareOp, DepthState, DepthStencilState},
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, PartialStateMode, Pipeline, PipelineBindPoint, StateMode,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};

use crate::{
    camera::Camera,
    object::{Instance, Vertex},
    world::World,
};

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/vertex.glsl",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/fragment.glsl"
    }
}

/// Minecraft engine and renderer (for now)
pub(crate) struct Engine {
    queue: Arc<Queue>,

    render_pass: Arc<RenderPass>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    uniform_buffer_pool: CpuBufferPool<vs::ty::UniformData>,
    descriptor_set_pool: SingleLayoutDescSetPool,

    depth_buffer: Arc<ImageView<AttachmentImage>>,

    // current mouse position for placing a block
    mouse_position: [f32; 2],
    holding_cursor: bool,
    // viewport saved size for placing a block
    viewport_size: [f32; 2],
    // collecting of blocks
    world: World,

    vertex_buffer_pool: CpuBufferPool<Vertex>,
    instance_buffer_pool: CpuBufferPool<Instance>,
    index_buffer_pool: CpuBufferPool<u32>,

    moving_direction: Vector3<f32>,

    camera: Camera,
}

impl Engine {
    pub fn new(queue: Arc<Queue>, image_format: Format) -> Self {
        // a render pass with color and reversed depth attachments (near is 1, far is 0)
        // which allows for high precision depth testing
        let render_pass = vulkano::single_pass_renderpass!(
            queue.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: image_format,
                    samples: 1,
                },
                depth:  {
                    load: Clear,
                    store: DontCare,
                    format: Format::D32_SFLOAT,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap();

        let vs = vs::load(queue.device().clone()).unwrap();
        let fs = fs::load(queue.device().clone()).unwrap();

        let graphics_pipeline = GraphicsPipeline::start()
            .vertex_input_state(
                BuffersDefinition::new()
                    .vertex::<Vertex>()
                    .instance::<Instance>(),
            )
            .input_assembly_state(InputAssemblyState {
                topology: PartialStateMode::Fixed(PrimitiveTopology::TriangleList),
                primitive_restart_enable: StateMode::Fixed(false),
            })
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState {
                depth: Some(DepthState {
                    enable_dynamic: false,
                    compare_op: StateMode::Fixed(CompareOp::Greater), // inverse operation
                    write_enable: StateMode::Fixed(true),
                }),
                ..Default::default()
            })
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(queue.device().clone())
            .unwrap();

        let uniform_buffer_pool =
            CpuBufferPool::new(queue.device().clone(), BufferUsage::uniform_buffer());
        let descriptor_set_pool = SingleLayoutDescSetPool::new(
            graphics_pipeline
                .layout()
                .set_layouts()
                .get(0)
                .unwrap()
                .clone(),
        );

        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(queue.device().clone(), [1, 1], Format::D32_SFLOAT).unwrap(),
        )
        .unwrap();

        let mut world = World::default();

        // create many chunks
        for i in 1..40 {
            world.create_chunk(i * 16, 60, i * 16, [1., 0., 0., 1.]);
            world.create_chunk(i * 16, 60, i * -16, [0., 1., 0., 1.]);
            world.create_chunk(i * -16, 60, i * 16, [0., 0., 1., 1.]);
            world.create_chunk(i * -16, 60, i * -16, [1., 0., 1., 1.]);
        }

        let vertex_buffer_pool =
            CpuBufferPool::new(queue.device().clone(), BufferUsage::vertex_buffer());
        let instance_buffer_pool =
            CpuBufferPool::new(queue.device().clone(), BufferUsage::vertex_buffer());
        let index_buffer_pool =
            CpuBufferPool::new(queue.device().clone(), BufferUsage::index_buffer());

        Self {
            queue,
            render_pass,
            graphics_pipeline,
            uniform_buffer_pool,
            descriptor_set_pool,

            depth_buffer,

            mouse_position: [0., 0.],
            holding_cursor: false,
            viewport_size: [0., 0.],
            world,
            vertex_buffer_pool,
            instance_buffer_pool,
            index_buffer_pool,
            moving_direction: Vector3::new(0., 0., 0.),
            camera: Camera::new(45., 0.0, 0.1, 100., [0., 125., -25.].into()),
        }
    }

    pub fn handle_events(&mut self, event: Event<()>) {
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        button: MouseButton::Right,
                        state,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => {
                    self.holding_cursor = true;
                }
                ElementState::Released => {
                    self.holding_cursor = false;
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                // unfortunately, we can't get the position inside a button
                // click event, so we have to keep track of it.
                let mouse_position: [f32; 2] = position.into();
                let diff = [
                    mouse_position[0] - self.mouse_position[0],
                    mouse_position[1] - self.mouse_position[1],
                ];
                self.mouse_position = mouse_position;

                if self.holding_cursor {
                    self.camera
                        .rotate_camera(Deg(diff[0] * 0.1).into(), Deg(diff[1] * 0.10).into());
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_, y),
                        ..
                    },
                ..
            } => {
                self.camera.zoom(y as f32 * 1.);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if pressed {
                    match keycode {
                        VirtualKeyCode::W => self.moving_direction.z = 1.,
                        VirtualKeyCode::S => self.moving_direction.z = -1.,
                        VirtualKeyCode::D => self.moving_direction.x = 1.,
                        VirtualKeyCode::A => self.moving_direction.x = -1.,
                        VirtualKeyCode::Space => self.moving_direction.y = 1.,
                        VirtualKeyCode::LShift => self.moving_direction.y = -1.,
                        _ => {}
                    }
                } else {
                    match keycode {
                        VirtualKeyCode::W | VirtualKeyCode::S => self.moving_direction.z = 0.,
                        VirtualKeyCode::D | VirtualKeyCode::A => self.moving_direction.x = 0.,
                        VirtualKeyCode::Space | VirtualKeyCode::LShift => {
                            self.moving_direction.y = 0.
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.camera
            .move_camera(self.moving_direction * delta.as_secs_f32() * 50.);

        const RADIUS: f32 = 10.;
        self.world.chunks_around_mut_callback(
            Point2::new(
                self.camera.position().x as i32,
                self.camera.position().z as i32,
            ),
            RADIUS,
            |chunk| {
                for cube in chunk
                    .cubes_around(self.camera.position().cast::<i32>().unwrap(), RADIUS)
                    .collect::<Vec<_>>()
                {
                    chunk.remove_cube(cube);
                }
            },
        );
    }

    pub fn render<Fin>(&mut self, image: Arc<dyn ImageAccess>, future: Fin) -> Box<dyn GpuFuture>
    where
        Fin: GpuFuture + 'static,
    {
        let img_size = image.dimensions().width_height();
        // save for later
        self.viewport_size = [img_size[0] as f32, img_size[1] as f32];

        // only resize when needed
        if self.depth_buffer.image().dimensions() != image.dimensions() {
            self.depth_buffer = ImageView::new_default(
                AttachmentImage::transient(
                    self.queue.device().clone(),
                    img_size,
                    Format::D32_SFLOAT,
                )
                .unwrap(),
            )
            .unwrap();
        }

        let image_view = ImageView::new_default(image).unwrap();

        let framebuffer = Framebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![image_view, self.depth_buffer.clone()],
                ..Default::default()
            },
        )
        .unwrap();

        let mut builder = AutoCommandBufferBuilder::primary(
            self.queue.device().clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        builder
            .begin_render_pass(
                framebuffer,
                SubpassContents::Inline,
                vec![
                    // blue sky color
                    ClearValue::Float([0., 0.7, 1., 1.0]),
                    ClearValue::Depth(0.0),
                ],
            )
            .unwrap();

        let mesh = self.world.mesh();

        if !mesh.is_empty() {
            let index_buffer = self
                .index_buffer_pool
                .chunk(mesh.indices().iter().cloned())
                .unwrap();

            let vertex_buffer = self
                .vertex_buffer_pool
                .chunk(mesh.vertices().iter().cloned())
                .unwrap();

            let instance_buffer = self
                .instance_buffer_pool
                .chunk(mesh.instances().iter().cloned())
                .unwrap();

            self.camera
                .set_aspect(self.viewport_size[0] / self.viewport_size[1]);

            let uniform_subbuffer = self
                .uniform_buffer_pool
                .next(vs::ty::UniformData {
                    perspective: self.camera.reversed_depth_perspective().into(),
                    view: self.camera.view().into(),
                })
                .unwrap();
            let descriptor_set = self
                .descriptor_set_pool
                .next([WriteDescriptorSet::buffer(0, uniform_subbuffer)])
                .unwrap();

            builder
                .set_viewport(
                    0,
                    [Viewport {
                        origin: [0.0, 0.0],
                        dimensions: self.viewport_size,
                        depth_range: 0.0..1.0,
                    }],
                )
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.graphics_pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .bind_index_buffer(index_buffer.clone())
                .bind_vertex_buffers(0, (vertex_buffer, instance_buffer.clone()))
                .bind_pipeline_graphics(self.graphics_pipeline.clone())
                .draw_indexed(
                    index_buffer.len() as u32,
                    instance_buffer.len() as u32,
                    0,
                    0,
                    0,
                )
                .unwrap();
        }

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .boxed()
    }
}
