/** This file gets included by the build.rs **/

// Aliases for various Vulkan types that libretro_vulkan.h uses.
// The original types are all prefixed with "Vk",
// but their Ash bindings drop this prefix in favor of the "vk" module
// (e.g. "VkDevice" becomes "vk::Device").

type retro_hw_render_context_negotiation_interface_type =
    crate::retro_hw_render_context_negotiation_interface_type::Type;
type retro_hw_render_interface_type = crate::retro_hw_render_interface_type::Type;

type PFN_vkGetInstanceProcAddr = Option<ash::vk::PFN_vkGetInstanceProcAddr>;
type PFN_vkGetDeviceProcAddr = Option<ash::vk::PFN_vkGetDeviceProcAddr>;
type VkApplicationInfo = ash::vk::ApplicationInfo;
type VkCommandBuffer = ash::vk::CommandBuffer;
type VkDevice = ash::vk::Device;
type VkImageLayout = ash::vk::ImageLayout;
type VkImageView = ash::vk::ImageView;
type VkImageViewCreateInfo = ash::vk::ImageViewCreateInfo;
type VkInstance = ash::vk::Instance;
type VkPhysicalDevice = ash::vk::PhysicalDevice;
type VkPhysicalDeviceFeatures = ash::vk::PhysicalDeviceFeatures;
type VkQueue = ash::vk::Queue;
type VkSemaphore = ash::vk::Semaphore;
type VkSurfaceKHR = ash::vk::SurfaceKHR;
