use std::{sync::Arc, time::Duration};

use cgmath::Deg;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
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
        GraphicsPipeline, PartialStateMode, Pipeline, StateMode,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};
use winit::event::{
    ElementState, Event, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta,
    VirtualKeyCode, WindowEvent,
};

use crate::{
    camera::Camera,
    object::{cube::Cube, Mesh, Square, Vertex},
};

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/vertex.glsl"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/fragment.glsl"
    }
}

#[derive(Default)]
struct World {
    cubes: Vec<Cube>,
    squares: Vec<Square>,
}

impl World {
    pub fn push_cube(&mut self, block: Cube) {
        self.cubes.push(block);
    }

    pub fn push_square(&mut self, block: Square) {
        self.squares.push(block);
    }
}

impl World {
    fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::with_capacity(self.cubes.len() * 6);

        for block in &self.cubes {
            mesh.append(block);
        }
        for block in &self.squares {
            mesh.append(block);
        }
        mesh
    }
}

/// Minecraft engine and renderer (for now)
pub(crate) struct Engine {
    queue: Arc<Queue>,

    render_pass: Arc<RenderPass>,
    graphics_pipeline: Arc<GraphicsPipeline>,

    depth_buffer: Arc<ImageView<AttachmentImage>>,

    // background coloring components
    r: f32,
    r_inc: f32,
    g: f32,
    g_inc: f32,
    b: f32,
    b_inc: f32,

    // current mouse position for placing a block
    mouse_position: [f32; 2],
    should_place_square: bool,
    holding_cursor: bool,
    // viewport saved size for placing a block
    viewport_size: [f32; 2],
    // collecting of blocks
    world: World,

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
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
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

        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(queue.device().clone(), [1, 1], Format::D32_SFLOAT).unwrap(),
        )
        .unwrap();

        Self {
            queue,
            render_pass,
            graphics_pipeline,

            depth_buffer,

            r: 1.0,
            r_inc: 0.006,
            g: 0.0,
            g_inc: 0.01,
            b: 0.0,
            b_inc: 0.015,

            mouse_position: [0., 0.],
            should_place_square: false,
            holding_cursor: false,
            viewport_size: [0., 0.],
            world: World::default(),
            camera: Camera::new(45., 0.0, 0.1, 100., [0., 0., 0.].into()),
        }
    }

    pub fn handle_events(&mut self, _event: Event<()>) {
        match _event {
            Event::WindowEvent {
                event: WindowEvent::ModifiersChanged(state),
                ..
            } => {
                self.should_place_square = state.intersects(ModifiersState::SHIFT);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.place_block(),
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
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => match keycode {
                VirtualKeyCode::W => self.camera.move_camera([0.0, 0.0, 0.2].into()),
                VirtualKeyCode::S => self.camera.move_camera([0.0, 0.0, -0.2].into()),
                VirtualKeyCode::A => self.camera.move_camera([-0.2, 0.0, 0.0].into()),
                VirtualKeyCode::D => self.camera.move_camera([0.2, 0.0, 0.0].into()),
                _ => {}
            },
            _ => {}
        }
    }

    pub fn update(&mut self, delta: Duration) {
        self.r += self.r_inc * delta.as_secs_f32() * 100.;
        self.g += self.g_inc * delta.as_secs_f32() * 100.;
        self.b += self.b_inc * delta.as_secs_f32() * 100.;

        if self.r > 1.0 {
            self.r_inc = -self.r_inc;
            self.r = 1.0;
        } else if self.r < 0.0 {
            self.r_inc = -self.r_inc;
            self.r = 0.0;
        }
        if self.g > 1.0 {
            self.g_inc = -self.g_inc;
            self.g = 1.0;
        } else if self.g < 0.0 {
            self.g_inc = -self.g_inc;
            self.g = 0.0;
        }
        if self.b > 1.0 {
            self.b_inc = -self.b_inc;
            self.b = 1.0;
        } else if self.b < 0.0 {
            self.b_inc = -self.b_inc;
            self.b = 0.0;
        }

        for block in &mut self.world.cubes {
            block.rotation[0] += 0.01 * 60. * delta.as_secs_f32();
            block.rotation[1] += 0.03 * 60. * delta.as_secs_f32();
            block.rotation[2] += 0.05 * 60. * delta.as_secs_f32();
        }
        for block in &mut self.world.squares {
            block.rotation[0] += 0.01 * 60. * delta.as_secs_f32();
            block.rotation[1] += 0.03 * 60. * delta.as_secs_f32();
            block.rotation[2] += 0.05 * 60. * delta.as_secs_f32();
        }
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
                    ClearValue::Float([self.r, self.g, self.b, 1.0]),
                    ClearValue::Depth(0.0),
                ],
            )
            .unwrap();

        let mesh = self.world.to_mesh();

        if !mesh.is_empty() {
            let index_buffer = CpuAccessibleBuffer::from_iter(
                self.queue.device().clone(),
                BufferUsage::all(),
                false,
                mesh.indices().iter().cloned(),
            )
            .unwrap();

            let vertex_buffer = CpuAccessibleBuffer::from_iter(
                self.queue.device().clone(),
                BufferUsage::all(),
                false,
                mesh.vertices().iter().cloned(),
            )
            .unwrap();

            self.camera
                .set_aspect(self.viewport_size[0] / self.viewport_size[1]);
            let push_constants = vs::ty::PushConstantData {
                perspective: self.camera.reversed_depth_perspective().into(),
                view: self.camera.view().into(),
            };

            builder
                .set_viewport(
                    0,
                    [Viewport {
                        origin: [0.0, 0.0],
                        dimensions: self.viewport_size,
                        depth_range: 0.0..1.0,
                    }],
                )
                .push_constants(self.graphics_pipeline.layout().clone(), 0, push_constants)
                .bind_index_buffer(index_buffer.clone())
                .bind_vertex_buffers(0, vec![vertex_buffer])
                .bind_pipeline_graphics(self.graphics_pipeline.clone())
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
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

impl Engine {
    /// Places block in the current mouse position
    fn place_block(&mut self) {
        // normalize position using viewport
        // this will just be able to normalize in the same direction
        // but the position will be very wrong, since we are using
        // perspective projection, we can retreive the correct position, but
        // its a bit of a hassle, since this will be removed anyway
        let pos = [
            (self.mouse_position[0] - self.viewport_size[0] / 2.) / self.viewport_size[0] * 4.,
            (self.mouse_position[1] - self.viewport_size[1] / 2.) / self.viewport_size[1] * -4.,
        ];

        // Pseudorandom number generator from the "Xorshift RNGs" paper by George Marsaglia.
        let mut random = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;

        // get z range from 5 to 70
        let z = (random % (70 - 5)) + 5;

        if self.should_place_square {
            let block = Square {
                center: [pos[0], pos[1], z as f32].into(),
                // use the current background color
                color: [self.r, self.g, self.b, 1.0],
                normal: [0.0, 0.0, -1.0].into(),
                rotation: [0.0, 0.0, 0.0],
            };
            self.world.push_square(block);
        } else {
            let block = Cube {
                center: [pos[0], pos[1], z as f32].into(),
                // use the current background color
                color: [self.r, self.g, self.b, 1.0],
                rotation: [0.0, 0.0, 0.0],
            };
            self.world.push_cube(block);
        }
    }
}
