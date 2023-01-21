#![cfg(feature = "vulkan")]

pub use ash;

// `PFN_vkGetInstanceProcAddr` and `PFN_vkGetDeviceProcAddr` are function pointers
// using the "system" ABI, but `Debug` is only implemented for the "Rust" and "C" ABIs.
// Therefore we either use a Newtype wrapper around those types and implement `Debug` for them,
// or manually implement `Debug` for `retro_hw_render_interface_vulkan`.
impl std::fmt::Debug for retro_hw_render_interface_vulkan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("retro_hw_render_interface_vulkan")
            .field("interface_type", &self.interface_type)
            .field("interface_version", &self.interface_version)
            .field("handle", &self.handle)
            .field("instance", &self.instance)
            .field("gpu", &self.gpu)
            .field("device", &self.device)
            .field(
                "get_device_proc_addr",
                &self.get_device_proc_addr.map(|f| f as *const ()),
            )
            .field(
                "get_instance_proc_addr",
                &self.get_instance_proc_addr.map(|f| f as *const ()),
            )
            .field("queue", &self.queue)
            .field("queue_index", &self.queue_index)
            .field("set_image", &self.set_image)
            .field("get_sync_index", &self.get_sync_index)
            .field("get_sync_index_mask", &self.get_sync_index_mask)
            .field("set_command_buffers", &self.set_command_buffers)
            .field("wait_sync_index", &self.wait_sync_index)
            .field("lock_queue", &self.lock_queue)
            .field("unlock_queue", &self.unlock_queue)
            .field("set_signal_semaphore", &self.set_signal_semaphore)
            .finish()
    }
}

#[test]
/// This test makes sure that we implement Debug for the current version
/// of the interface struct.
fn retro_hw_render_interface_vulkan_debug_for_current_version() {
    assert_eq!(RETRO_HW_RENDER_INTERFACE_VULKAN_VERSION, 5);
}

include!(concat!(env!("OUT_DIR"), "/bindings_libretro_vulkan.rs"));
