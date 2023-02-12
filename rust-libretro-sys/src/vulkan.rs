#![cfg(feature = "vulkan")]

//! Aliases for various Vulkan types that libretro_vulkan.h uses.
//! The original types are all prefixed with "Vk",
//! but their Ash bindings drop this prefix in favor of the "vk" module
//! (e.g. "VkDevice" becomes "vk::Device").

pub use ash;
use ash::vk::*;

pub type PFN_vkGetInstanceProcAddr = Option<ash::vk::PFN_vkGetInstanceProcAddr>;
pub type PFN_vkGetDeviceProcAddr = Option<ash::vk::PFN_vkGetDeviceProcAddr>;
pub type VkApplicationInfo = ApplicationInfo;
pub type VkCommandBuffer = CommandBuffer;
pub type VkDevice = Device;
pub type VkDeviceCreateInfo = DeviceCreateInfo;
pub type VkImageLayout = ImageLayout;
pub type VkImageView = ImageView;
pub type VkImageViewCreateInfo = ImageViewCreateInfo;
pub type VkInstance = Instance;
pub type VkInstanceCreateInfo = InstanceCreateInfo;
pub type VkPhysicalDevice = PhysicalDevice;
pub type VkPhysicalDeviceFeatures = PhysicalDeviceFeatures;
pub type VkQueue = Queue;
pub type VkSemaphore = Semaphore;
pub type VkSurfaceKHR = SurfaceKHR;
