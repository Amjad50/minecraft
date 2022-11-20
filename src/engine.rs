use std::{f32::consts::PI, sync::Arc, time::Duration};

use cgmath::{Deg, Matrix4, SquareMatrix, Vector3};
use vulkano::{
    buffer::{BufferUsage, CpuBufferPool, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, SubpassContents,
    },
    descriptor_set::{SingleLayoutDescSetPool, WriteDescriptorSet},
    device::Queue,
    format::{ClearValue, Format},
    image::{view::ImageView, AttachmentImage, ImageAccess},
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
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
    object::{cube::Cube, rotation_scale_matrix, Instance, InstancesMesh, Mesh, Vertex},
    world::{CubeLookAt, World},
};

#[allow(clippy::needless_question_mark)]
mod cubes_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/cubes.vert.glsl",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

#[allow(clippy::needless_question_mark)]
mod cubes_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/cubes.frag.glsl"
    }
}

#[allow(clippy::needless_question_mark)]
mod cubes_no_light_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/cubes_no_light.frag.glsl"
    }
}

#[allow(clippy::needless_question_mark)]
mod ui_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/shaders/ui.vert.glsl",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

#[allow(clippy::needless_question_mark)]
mod ui_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/shaders/ui.frag.glsl"
    }
}

/// Minecraft engine and renderer (for now)
pub(crate) struct Engine {
    queue: Arc<Queue>,

    render_pass: Arc<RenderPass>,
    cubes_graphics_pipeline: Arc<GraphicsPipeline>,
    cubes_line_graphics_pipeline: Arc<GraphicsPipeline>,
    ui_graphics_pipeline: Arc<GraphicsPipeline>,
    uniform_buffer_pool: CpuBufferPool<cubes_vs::ty::UniformData>,
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
    looking_at_cube: Option<CubeLookAt>,
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

        let vs_cubes = cubes_vs::load(queue.device().clone()).unwrap();
        let fs_cubes = cubes_fs::load(queue.device().clone()).unwrap();
        let fs_cubes_no_light = cubes_no_light_fs::load(queue.device().clone()).unwrap();

        let vs_ui = ui_vs::load(queue.device().clone()).unwrap();
        let fs_ui = ui_fs::load(queue.device().clone()).unwrap();

        let cubes_graphics_pipeline = GraphicsPipeline::start()
            .vertex_input_state(
                BuffersDefinition::new()
                    .vertex::<Vertex>()
                    .instance::<Instance>(),
            )
            .input_assembly_state(InputAssemblyState {
                topology: PartialStateMode::Fixed(PrimitiveTopology::TriangleList),
                primitive_restart_enable: StateMode::Fixed(false),
            })
            .vertex_shader(vs_cubes.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs_cubes.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState {
                depth: Some(DepthState {
                    enable_dynamic: false,
                    compare_op: StateMode::Fixed(CompareOp::Greater), // inverse operation
                    write_enable: StateMode::Fixed(true),
                }),
                ..Default::default()
            })
            .color_blend_state(ColorBlendState::new(1).blend_alpha())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(queue.device().clone())
            .unwrap();

        let cubes_line_graphics_pipeline = GraphicsPipeline::start()
            .vertex_input_state(
                BuffersDefinition::new()
                    .vertex::<Vertex>()
                    .instance::<Instance>(),
            )
            .input_assembly_state(InputAssemblyState {
                topology: PartialStateMode::Fixed(PrimitiveTopology::LineList),
                primitive_restart_enable: StateMode::Fixed(false),
            })
            .vertex_shader(vs_cubes.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs_cubes_no_light.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState {
                depth: Some(DepthState {
                    enable_dynamic: false,
                    compare_op: StateMode::Fixed(CompareOp::GreaterOrEqual), // inverse operation
                    write_enable: StateMode::Fixed(false),
                }),
                ..Default::default()
            })
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(queue.device().clone())
            .unwrap();

        let ui_graphics_pipeline = GraphicsPipeline::start()
            .vertex_input_state(
                BuffersDefinition::new()
                    .vertex::<Vertex>()
                    .instance::<Instance>(),
            )
            .input_assembly_state(InputAssemblyState {
                topology: PartialStateMode::Fixed(PrimitiveTopology::LineList),
                primitive_restart_enable: StateMode::Fixed(false),
            })
            .vertex_shader(vs_ui.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs_ui.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(queue.device().clone())
            .unwrap();

        let uniform_buffer_pool =
            CpuBufferPool::new(queue.device().clone(), BufferUsage::uniform_buffer());
        let descriptor_set_pool = SingleLayoutDescSetPool::new(
            cubes_graphics_pipeline
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
        let x_size = 5;
        let y_size = 5;
        for x in 0..x_size {
            for y in 0..y_size {
                world.create_chunk(
                    x * 16,
                    60,
                    y * 16,
                    [
                        x as f32 / x_size as f32,
                        y as f32 / y_size as f32,
                        (x + y) as f32 / (x_size + y_size) as f32,
                        1.,
                    ],
                );
            }
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
            cubes_graphics_pipeline,
            cubes_line_graphics_pipeline,
            ui_graphics_pipeline,
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
            camera: Camera::new(Deg(45.), 0.0, 0.1, 100., [0., 125., -25.].into()),
            looking_at_cube: None,
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
                event:
                    WindowEvent::MouseInput {
                        button,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match button {
                MouseButton::Left => {
                    self.remove_looking_at();
                }
                MouseButton::Middle => {
                    self.place_at_looking_at();
                }
                _ => unreachable!(),
            },
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                // unfortunately, we can't get the position inside a button
                // click event, so we have to keep track of it.
                let mouse_position: [f32; 2] = position.into();
                let angles = [
                    // movement in x direction in display moves the camera
                    // around the y axis (yaw)
                    mouse_position[0] - self.mouse_position[0],
                    // movement in y direction in display moves the camera
                    // around the x axis (pitch)
                    //
                    // because the `y` axis is inverted, we have to negate
                    // the angle here
                    -(mouse_position[1] - self.mouse_position[1]),
                ];
                self.mouse_position = mouse_position;

                if self.holding_cursor {
                    self.camera
                        .rotate_camera(Deg(angles[1] * 0.10), Deg(angles[0] * 0.1));
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
                self.camera.zoom(Deg(y as f32 * 1.));
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

    #[tracing::instrument(skip_all)]
    pub fn update(&mut self, delta: Duration) {
        self.camera
            .move_camera(self.moving_direction * delta.as_secs_f32() * 50.);

        const LOOK_RADIUS: f32 = 100.;

        let result = self.world.cube_looking_at(
            self.camera.position(),
            self.camera.direction(),
            LOOK_RADIUS,
        );
        self.looking_at_cube = result.result_cube;
    }

    #[tracing::instrument(skip_all)]
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

        self.camera
            .set_aspect(self.viewport_size[0] / self.viewport_size[1]);

        let uniform_subbuffer = self
            .uniform_buffer_pool
            .next(cubes_vs::ty::UniformData {
                perspective: self.camera.reversed_depth_perspective().into(),
                view: self.camera.view().into(),
                rotation_scale: Matrix4::identity().into(),
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
                self.cubes_graphics_pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .bind_pipeline_graphics(self.cubes_graphics_pipeline.clone());

        // create them once
        let empty_cube_mesh = InstancesMesh::<Cube>::new().unwrap();
        let index_buffer = self
            .index_buffer_pool
            .chunk(empty_cube_mesh.indices().iter().cloned())
            .unwrap();
        let vertex_buffer = self
            .vertex_buffer_pool
            .chunk(empty_cube_mesh.vertices().iter().cloned())
            .unwrap();

        let mut render_mesh = |mesh: &InstancesMesh<Cube>| {
            let instance_buffer = self
                .instance_buffer_pool
                .chunk(mesh.instances().iter().cloned())
                .unwrap();

            builder
                .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
                .bind_index_buffer(index_buffer.clone())
                .draw_indexed(
                    index_buffer.len() as u32,
                    instance_buffer.len() as u32,
                    0,
                    0,
                    0,
                )
                .unwrap();
        };

        for chunk in self.world.all_chunks_mut() {
            let span = tracing::info_span!("render mesh {}", "{:?}", chunk.start());
            let _enter = span.enter();
            let mesh = chunk.mesh();
            render_mesh(mesh);
        }

        self.render_looking_at(&mut builder);
        self.render_ui(img_size, &mut builder);

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .boxed()
    }

    #[tracing::instrument(skip_all)]
    fn render_looking_at(
        &mut self,

        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        if let Some(CubeLookAt { cube, .. }) = self.looking_at_cube {
            let cube_vertices = Cube::mesh().0;
            let indices = [
                0, 1, // front t
                1, 3, // front r
                0, 2, // front l
                2, 3, // front b
                //
                4, 5, // back t
                5, 7, // back r
                4, 6, // back l
                6, 7, // back b
                //
                1, 5, // right t
                3, 7, // right b
                //
                0, 4, // left t
                2, 6, // left b
            ];
            let instances = [Instance {
                color: [1., 1., 1., 1.],
                translation: cube.cast::<f32>().unwrap().into(),
            }];
            let vertex_buffer = self.vertex_buffer_pool.chunk(cube_vertices).unwrap();
            let instance_buffer = self.instance_buffer_pool.chunk(instances).unwrap();
            let index_buffer = self.index_buffer_pool.chunk(indices).unwrap();

            let uniform_subbuffer = self
                .uniform_buffer_pool
                .next(cubes_vs::ty::UniformData {
                    // scale a bit outward so that it doesn't collide with the block
                    // itself and draw glitched cube (because of depth collision)
                    rotation_scale: rotation_scale_matrix([0., 0., 0.], 1.012).into(),
                    perspective: self.camera.reversed_depth_perspective().into(),
                    view: self.camera.view().into(),
                })
                .unwrap();
            let descriptor_set = self
                .descriptor_set_pool
                .next([WriteDescriptorSet::buffer(0, uniform_subbuffer)])
                .unwrap();

            builder
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.cubes_line_graphics_pipeline.layout().clone(),
                    0,
                    descriptor_set,
                )
                .bind_vertex_buffers(0, (vertex_buffer, instance_buffer.clone()))
                .bind_pipeline_graphics(self.cubes_line_graphics_pipeline.clone())
                .bind_index_buffer(index_buffer.clone())
                .draw_indexed(
                    index_buffer.len() as u32,
                    instance_buffer.len() as u32,
                    0,
                    0,
                    0,
                )
                .unwrap();
        }
    }

    #[tracing::instrument(skip_all)]
    fn render_ui(
        &mut self,
        img_size: [u32; 2],
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        // create a line for the cross cross of 20 pixels in size
        let vertices = [
            Vertex {
                pos: [0., 10., 0.],
                normal: [0., 0., 0.],
            },
            Vertex {
                pos: [0., -10., 0.],
                normal: [0., 0., 0.],
            },
        ];

        let vertex_buffer = self.vertex_buffer_pool.chunk(vertices).unwrap();

        let instances = [Instance {
            color: [1., 1., 1., 1.],
            translation: [img_size[0] as f32 / 2., img_size[1] as f32 / 2., 0.],
        }];
        let instance_buffer = self.instance_buffer_pool.chunk(instances).unwrap();

        builder.bind_pipeline_graphics(self.ui_graphics_pipeline.clone());

        // draw the line two times
        for r in [0., PI / 2.] {
            builder
                .bind_vertex_buffers(0, (vertex_buffer.clone(), instance_buffer.clone()))
                .push_constants(
                    self.ui_graphics_pipeline.layout().clone(),
                    0,
                    ui_vs::ty::PushConstants {
                        display_size: img_size,
                        rotation_scale: rotation_scale_matrix([0., 0., r], 1.).into(),
                        _dummy0: [0; 8],
                    },
                )
                .draw(
                    vertex_buffer.len() as u32,
                    instance_buffer.len() as u32,
                    0,
                    0,
                )
                .unwrap();
        }
    }
}

impl Engine {
    /// place a random block at the current looking block
    fn place_at_looking_at(&mut self) {
        if let Some(cube) = &self.looking_at_cube {
            // we use the direction to know where the ray is coming from
            let new_cube = cube.cube + cube.direction;

            self.world.push_cube(Cube {
                center: new_cube.cast().unwrap(),
                color: [1., 0.5, 1.0, 1.],
            })
        }
    }

    fn remove_looking_at(&mut self) {
        if let Some(cube) = &self.looking_at_cube {
            self.world.remove_cube(cube.cube);
        }
    }
}
