use crate::app::App;

use crate::vulkanr::vulkan::*;

use super::instance::{PORTABILITY_MACOS_VERSION, VALIDATION_ENABLED, VALIDATION_LAYER};
use crate::ecs::MAX_FRAMES_IN_FLIGHT;

use vulkanalia::Device as VkDevice;

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use winit::window::Window;

impl App {
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

        // コンソール（色付き）とログファイルの両方に出力
        use log::{debug, error, trace, warn};
        if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            error!("({:?}) {}", type_, message);
            log_error!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            warn!("({:?}) {}", type_, message);
            log_warn!("({:?}) {}", type_, message);
        } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
            debug!("({:?}) {}", type_, message);
            log!("({:?}) {}", type_, message);
        } else {
            trace!("({:?}) {}", type_, message);
            log!("DEBUG ({:?}) {}", type_, message);
        }

        vk::FALSE
    }

    pub(super) unsafe fn create_instance_with_messenger(
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
            .collect::<HashSet<_>>();

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
            .user_callback(Some(Self::debug_callback));

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

    pub(super) unsafe fn create_sync_objects(
        device: &VkDevice,
    ) -> Result<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>)> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available = Vec::new();
        let mut render_finished = Vec::new();
        let mut in_flight = Vec::new();

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available.push(device.create_semaphore(&semaphore_info, None)?);
            render_finished.push(device.create_semaphore(&semaphore_info, None)?);
            in_flight.push(device.create_fence(&fence_info, None)?);
        }

        Ok((image_available, render_finished, in_flight))
    }
}
