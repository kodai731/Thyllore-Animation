use crate::core::device::{RRDevice, VALIDATION_LAYER};
use crate::types::{Fence, Semaphore};
use crate::vulkan::*;
use std::ffi::CStr;
use std::os::raw::c_void;
use thyllore_log_core::message_buffer::{push_message, MessageLevel, VALIDATION_MESSAGE_PREFIX};
use thyllore_log_core::validation_stats::{record_validation, ValidationSeverity};
use winit::window::Window;

pub const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
const VALIDATION_MODE: crate::core::device::ValidationMode = if cfg!(debug_assertions) {
    crate::core::device::ValidationMode::Enabled
} else {
    crate::core::device::ValidationMode::Disabled
};

pub const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[
    vk::KHR_SWAPCHAIN_EXTENSION.name,
    vk::KHR_BUFFER_DEVICE_ADDRESS_EXTENSION.name,
    vk::KHR_ACCELERATION_STRUCTURE_EXTENSION.name,
    vk::KHR_RAY_QUERY_EXTENSION.name,
    vk::KHR_RAY_TRACING_PIPELINE_EXTENSION.name,
    vk::KHR_DEFERRED_HOST_OPERATIONS_EXTENSION.name,
];

pub struct VulkanContext {
    pub entry: Entry,
    pub instance: Instance,
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub surface: vk::SurfaceKHR,
    pub rrdevice: RRDevice,
}

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = if data.message.is_null() {
        std::borrow::Cow::Borrowed("(no message)")
    } else {
        unsafe { CStr::from_ptr(data.message) }.to_string_lossy()
    };

    use log::{debug, error, trace, warn};
    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
        log_error!("({:?}) {}", type_, message);
        record_validation(ValidationSeverity::Error, &message);
        push_message(
            MessageLevel::Error,
            format!("{VALIDATION_MESSAGE_PREFIX} {message}"),
        );
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
        log_warn!("({:?}) {}", type_, message);
        record_validation(ValidationSeverity::Warning, &message);
        push_message(
            MessageLevel::Warning,
            format!("{VALIDATION_MESSAGE_PREFIX} {message}"),
        );
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}) {}", type_, message);
        log!("({:?}) {}", type_, message);
    } else {
        trace!("({:?}) {}", type_, message);
        log!("DEBUG ({:?}) {}", type_, message);
    }

    vk::FALSE
}

pub unsafe fn create_instance_with_messenger(
    window: &Window,
    entry: &Entry,
) -> Result<(Instance, vk::DebugUtilsMessengerEXT)> {
    let application_info = vk::ApplicationInfo::builder()
        .application_name(b"Vulkan Tutorial\0")
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(b"No Engine\0")
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 2, 0));

    let mut extensions = vk_window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        log!("Enabling extensions for macOS portability.");
        extensions.push(
            vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                .name
                .as_ptr(),
        );
        extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    let available_layers = entry
        .enumerate_instance_layer_properties()?
        .iter()
        .map(|l| l.layer_name)
        .collect::<std::collections::HashSet<_>>();

    if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
        return Err(anyhow!("Validation layer requested but not supported"));
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    let mut info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
        .user_callback(Some(debug_callback));

    if VALIDATION_ENABLED {
        info = info.push_next(&mut debug_info);
    }

    let instance = entry.create_instance(&info, None)?;

    let messenger = if VALIDATION_ENABLED {
        instance.create_debug_utils_messenger_ext(&debug_info, None)?
    } else {
        vk::DebugUtilsMessengerEXT::null()
    };

    Ok((instance, messenger))
}

pub unsafe fn create_sync_objects(
    rrdevice: &RRDevice,
    count: usize,
) -> Result<(Vec<Semaphore>, Vec<Semaphore>, Vec<Fence>)> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

    let mut image_available = Vec::new();
    let mut render_finished = Vec::new();
    let mut in_flight = Vec::new();

    for _ in 0..count {
        image_available.push(rrdevice.device.create_semaphore(&semaphore_info, None)?);
        render_finished.push(rrdevice.device.create_semaphore(&semaphore_info, None)?);
        in_flight.push(rrdevice.device.create_fence(&fence_info, None)?);
    }

    Ok((image_available, render_finished, in_flight))
}

pub unsafe fn create_vulkan_context(window: &Window) -> Result<VulkanContext> {
    let loader = LibloadingLoader::new(LIBRARY)?;
    let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
    let (instance, messenger) = create_instance_with_messenger(window, &entry)?;
    let surface = vk_window::create_surface(&instance, &window, &window)?;
    let rrdevice = RRDevice::new(
        &entry,
        &instance,
        &surface,
        VALIDATION_MODE,
        VALIDATION_LAYER,
        DEVICE_EXTENSIONS,
        PORTABILITY_MACOS_VERSION,
    )?;

    Ok(VulkanContext {
        entry,
        instance,
        messenger,
        surface,
        rrdevice,
    })
}
