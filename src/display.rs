use std::sync::Arc;

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    swapchain::{AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

#[derive(Debug)]
pub(crate) enum FrameError {
    AcquireOutOfDate,
    MultipleBeginFrame,
    EmptyDisplay,
}

impl std::error::Error for FrameError {}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::AcquireOutOfDate => write!(
                f,
                "Acquire out of date error, try beginning the frame again"
            ),
            FrameError::MultipleBeginFrame => writeln!(
                f,
                "Tried to `begin_frame` multiple times before ending the previous frame"
            ),
            FrameError::EmptyDisplay => {
                write!(f, "The display is empty (maybe minimized in windows)")
            }
        }
    }
}

/// Houses all the setup and surface rendering for vulkan
pub(crate) struct Display {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<Window>>,
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

    current_image_num: usize,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Display {
    pub fn new(event_loop: &EventLoop<()>, swapchain_image_usage: ImageUsage) -> Self {
        let required_extensions = vulkano_win::required_extensions();

        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: required_extensions,
            ..Default::default()
        })
        .unwrap();

        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| {
                        q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false)
                    })
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            })
            .unwrap();

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: physical_device
                    .required_extensions()
                    .union(&device_extensions),
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                ..Default::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        // create swapchains
        let (swapchain, swapchain_images) = {
            let surface_capabilities = physical_device
                .surface_capabilities(&surface, Default::default())
                .unwrap();

            let image_format = Some(
                physical_device
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format,
                    image_extent: surface.window().inner_size().into(),
                    image_usage: swapchain_image_usage,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Self {
            device,
            queue,
            surface,
            swapchain,
            swapchain_images,

            current_image_num: 0,
            recreate_swapchain: false,
            previous_frame_end,
        }
    }

    pub fn resize(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    pub fn current_image(&self) -> Arc<SwapchainImage<Window>> {
        self.swapchain_images[self.current_image_num].clone()
    }

    pub fn swapchain_image_format(&self) -> Format {
        self.swapchain.image_format()
    }

    pub fn begin_frame(&mut self) -> Result<Box<dyn GpuFuture>, FrameError> {
        // Do not draw frame when screen dimensions are zero.
        // On Windows, this can occur from minimizing the application.
        if self.is_empty() {
            return Err(FrameError::EmptyDisplay);
        }

        let mut last_future = self
            .previous_frame_end
            .take()
            .ok_or(FrameError::MultipleBeginFrame)?;
        last_future.cleanup_finished();

        if self.recreate_swapchain {
            self.recreate_swapchains();
            self.recreate_swapchain = false;
        }

        let (image_num, suboptimal, acquire_future) =
            match vulkano::swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(FrameError::AcquireOutOfDate);
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        self.current_image_num = image_num;

        Ok(last_future.join(acquire_future).boxed())
    }

    pub fn end_frame<F>(&mut self, future: F)
    where
        F: GpuFuture + 'static,
    {
        let future = future
            .then_swapchain_present(
                self.queue.clone(),
                self.swapchain.clone(),
                self.current_image_num,
            )
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }
}

impl Display {
    fn is_empty(&self) -> bool {
        let dimensions = self.surface.window().inner_size();
        dimensions.width == 0 || dimensions.height == 0
    }

    fn recreate_swapchains(&mut self) {
        let dimensions = self.surface.window().inner_size();
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: dimensions.into(),
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = new_swapchain;
        self.swapchain_images = new_images;
    }
}
