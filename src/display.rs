use std::sync::Arc;

use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    image::{ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    swapchain::{Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

/// Houses all the setup and surface rendering for vulkan
pub(crate) struct Display {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Arc<Surface<Window>>,
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
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

        Self {
            device,
            queue,
            surface,
            swapchain,
            swapchain_images,
        }
    }

    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }

    pub fn swapchain(&self) -> Arc<Swapchain<Window>> {
        self.swapchain.clone()
    }

    pub fn swapchain_image(&self, num: usize) -> Option<Arc<SwapchainImage<Window>>> {
        self.swapchain_images.get(num).map(|img| img.clone())
    }
}

impl Display {
    pub fn is_empty(&self) -> bool {
        let dimensions = self.surface.window().inner_size();
        dimensions.width == 0 || dimensions.height == 0
    }

    pub fn recreate_swapchains(&mut self) {
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
