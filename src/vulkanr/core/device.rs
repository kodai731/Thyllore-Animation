use super::swapchain::*;
use crate::log;
use crate::vulkanr::vulkan::*;
use std::collections::HashSet;
use std::ops::Deref;
use thiserror::Error;
use vulkanalia::Device as VulkanDevice;

#[derive(Clone, Debug)]
pub struct Device(VulkanDevice);

impl Device {
    pub fn new(device: VulkanDevice) -> Self {
        Device(device)
    }
    pub fn null() -> Self {
        unsafe { Device(std::mem::zeroed()) }
    }
}

impl Default for Device {
    fn default() -> Self {
        Device::null()
    }
}

impl Deref for Device {
    type Target = VulkanDevice;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct RRDevice {
    pub device: Device,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub msaa_samples: vk::SampleCountFlags,
    pub min_uniform_buffer_offset_alignment: u64,
}

impl RRDevice {
    pub unsafe fn default() -> RRDevice {
        Self {
            device: std::mem::zeroed(),
            physical_device: vk::PhysicalDevice::default(),
            graphics_queue: vk::Queue::default(),
            present_queue: vk::Queue::default(),
            msaa_samples: vk::SampleCountFlags::default(),
            min_uniform_buffer_offset_alignment: 256,
        }
    }
    pub unsafe fn new(
        entry: &Entry,
        instance: &Instance,
        surface: &vk::SurfaceKHR,
        validation_enabled: bool,
        validation_layer: vk::ExtensionName,
        device_extensions: &[vk::ExtensionName],
        portability_macro_version: Version,
    ) -> Result<RRDevice> {
        let (physical_device, sample_count) =
            pick_physical_device(instance, surface, device_extensions)?;
        let (device, graphics_queue, present_queue) = create_logical_device(
            entry,
            instance,
            surface,
            validation_enabled,
            validation_layer,
            device_extensions,
            portability_macro_version,
            &physical_device,
        )?;

        let properties = instance.get_physical_device_properties(physical_device);
        let min_ubo_alignment = properties.limits.min_uniform_buffer_offset_alignment;

        println!("created logical device");
        Ok(Self {
            device: device,
            physical_device: physical_device,
            graphics_queue: graphics_queue,
            present_queue: present_queue,
            msaa_samples: sample_count,
            min_uniform_buffer_offset_alignment: min_ubo_alignment,
        })
    }
}

unsafe fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    surface: &vk::SurfaceKHR,
    validation_enabled: bool,
    validation_layer: vk::ExtensionName,
    device_extensions: &[vk::ExtensionName],
    portability_macro_version: Version,
    physical_device: &vk::PhysicalDevice,
) -> Result<(Device, vk::Queue, vk::Queue)> {
    let indices = QueueFamilyIndices::get(instance, surface, physical_device)?;
    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.present);
    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    let layers = if validation_enabled {
        vec![validation_layer.as_ptr()]
    } else {
        vec![]
    };

    let mut extensions = device_extensions
        .iter()
        .map(|n| n.as_ptr())
        .collect::<Vec<_>>();
    if cfg!(target_os = "macos") && entry.version()? >= portability_macro_version {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    let features = vk::PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .sample_rate_shading(true)
        .fill_mode_non_solid(true)
        .independent_blend(true);

    let mut acceleration_structure_features =
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true);

    let mut ray_query_features = vk::PhysicalDeviceRayQueryFeaturesKHR::builder().ray_query(true);

    let mut vulkan_12_features =
        vk::PhysicalDeviceVulkan12Features::builder().buffer_device_address(true);

    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features)
        .push_next(&mut vulkan_12_features)
        .push_next(&mut acceleration_structure_features)
        .push_next(&mut ray_query_features);

    let vulkan_device = instance.create_device(*physical_device, &info, None)?;
    let device = Device::new(vulkan_device);

    let graphics_queue = device.get_device_queue(indices.graphics, 0);
    let present_queue = device.get_device_queue(indices.present, 0);
    Ok((device, graphics_queue, present_queue))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}
// TODO: system for trasfer bit, see https://kylemayes.github.io/vulkanalia/vertex/staging_buffer.html

impl QueueFamilyIndices {
    pub unsafe fn get(
        instance: &Instance,
        surface: &vk::SurfaceKHR,
        physical_device: &vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties = instance.get_physical_device_queue_family_properties(*physical_device);
        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;
        for (index, properties) in properties.iter().enumerate() {
            if instance.get_physical_device_surface_support_khr(
                *physical_device,
                index as u32,
                *surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            Err(anyhow!(SuitabilityError("Missing required queue families")))
        }
    }
}

#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);
unsafe fn pick_physical_device(
    instance: &Instance,
    surface: &vk::SurfaceKHR,
    device_extensions: &[vk::ExtensionName],
) -> Result<(vk::PhysicalDevice, vk::SampleCountFlags)> {
    for physical_device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(physical_device);

        if let Err(error) =
            check_physical_device(instance, surface, &physical_device, device_extensions)
        {
            log::warn!(
                "Skipping Physical Device (`{}`): {}",
                properties.device_name,
                error
            );
        } else {
            log::info!("Selected Physical Device (`{}`).", properties.device_name);
            let sample_count = get_max_msaa_samples(instance, physical_device);
            return Ok((physical_device, sample_count));
        }
    }

    Err(anyhow!("Failed to find suitable physical device"))
}

unsafe fn check_physical_device(
    instance: &Instance,
    surface: &vk::SurfaceKHR,
    physical_device: &vk::PhysicalDevice,
    device_extensions: &[vk::ExtensionName],
) -> Result<()> {
    let properties = instance.get_physical_device_properties(*physical_device);
    log!("Checking device: {}", properties.device_name);
    log!("Device type: {:?}", properties.device_type);

    if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
        log!("Device rejected: Only discrete GPUs are supported");
        return Err(anyhow!(SuitabilityError(
            "Only discrete GPUs are supported"
        )));
    }
    log!("Device type check passed");

    let features = instance.get_physical_device_features(*physical_device);
    if features.sampler_anisotropy != vk::TRUE {
        log!("Device rejected: No sampler anisotropy");
        return Err(anyhow!(SuitabilityError("No sampler anisotropy")));
    }
    log!("Sampler anisotropy check passed");

    if features.geometry_shader != vk::TRUE {
        log!("Device rejected: Missing geometry shader system");
        return Err(anyhow!(SuitabilityError(
            "Missing geometry shader supported"
        )));
    }
    log!("Geometry shader check passed");

    match QueueFamilyIndices::get(instance, surface, physical_device) {
        Ok(_) => log!("Queue family check passed"),
        Err(e) => {
            log!("Device rejected: Queue family check failed - {}", e);
            return Err(e);
        }
    }

    match check_physical_device_extensions(instance, physical_device, device_extensions) {
        Ok(_) => log!("Device extensions check passed"),
        Err(e) => {
            log!("Device rejected: Extensions check failed - {}", e);
            return Err(e);
        }
    }

    let support = SwapchainSupport::get(instance, surface, physical_device)?;
    if support.formats.is_empty() || support.present_modes.is_empty() {
        log!("Device rejected: Insufficient swapchain system");
        return Err(anyhow!(SuitabilityError("Insufficient swapchain system")));
    }
    log!("Swapchain system check passed");

    log!("All device checks passed!");
    Ok(())
}

unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: &vk::PhysicalDevice,
    device_extensions: &[vk::ExtensionName],
) -> Result<()> {
    let extensions = instance
        .enumerate_device_extension_properties(*physical_device, None)?
        .iter()
        .map(|e| e.extension_name)
        .collect::<HashSet<_>>();

    log!("Required extensions: {}", device_extensions.len());
    for ext in device_extensions {
        let supported = extensions.contains(ext);
        log!(
            "  {} - {}",
            ext,
            if supported {
                "SUPPORTED"
            } else {
                "NOT SUPPORTED"
            }
        );
    }

    let missing_extensions: Vec<_> = device_extensions
        .iter()
        .filter(|e| !extensions.contains(e))
        .collect();

    if missing_extensions.is_empty() {
        Ok(())
    } else {
        log!("Missing {} extensions:", missing_extensions.len());
        for ext in &missing_extensions {
            log!("  - {}", ext);
        }
        Err(anyhow!(SuitabilityError("Device Extensions Not Supported")))
    }
}

unsafe fn get_max_msaa_samples(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> vk::SampleCountFlags {
    let properties = instance.get_physical_device_properties(physical_device);
    let counts = properties.limits.framebuffer_color_sample_counts
        & properties.limits.framebuffer_depth_sample_counts;
    [
        vk::SampleCountFlags::_64,
        vk::SampleCountFlags::_32,
        vk::SampleCountFlags::_16,
        vk::SampleCountFlags::_8,
        vk::SampleCountFlags::_4,
        vk::SampleCountFlags::_2,
    ]
    .iter()
    .cloned()
    .find(|c| counts.contains(*c))
    .unwrap_or(vk::SampleCountFlags::_1)
}
