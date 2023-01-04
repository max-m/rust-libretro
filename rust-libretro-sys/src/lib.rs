#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/favicon.png"
)]

use core::fmt::Display;
use rust_libretro_sys_proc::TryFromPrimitive;

#[cfg(feature = "vulkan")]
pub mod vulkan;

#[cfg(feature = "vulkan")]
use vulkan::*;

include!(concat!(env!("OUT_DIR"), "/bindings_libretro.rs"));

#[cfg(feature = "vulkan")]
include!(concat!(env!("OUT_DIR"), "/bindings_libretro_vulkan.rs"));

/// #define RETRO_DEVICE_SUBCLASS(base, id) (((id + 1) << RETRO_DEVICE_TYPE_SHIFT) | base)
#[macro_export]
macro_rules! RETRO_DEVICE_SUBCLASS {
    ($base:expr, $id:expr) => {
        (($id + 1) << RETRO_DEVICE_TYPE_SHIFT) | $base
    };
}

#[deprecated = "This uses relative positions. Use `RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X` instead."]
pub const RETRO_DEVICE_ID_LIGHTGUN_X: u32 = 0;
#[deprecated = "This uses relative positions. Use `RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y` instead."]
pub const RETRO_DEVICE_ID_LIGHTGUN_Y: u32 = 1;
#[deprecated = "Use `RETRO_DEVICE_ID_LIGHTGUN_AUX_A` instead."]
pub const RETRO_DEVICE_ID_LIGHTGUN_CURSOR: u32 = 3;
#[deprecated = "Use `RETRO_DEVICE_ID_LIGHTGUN_AUX_B` instead."]
pub const RETRO_DEVICE_ID_LIGHTGUN_TURBO: u32 = 4;
#[deprecated = "Use `RETRO_DEVICE_ID_LIGHTGUN_START` instead."]
pub const RETRO_DEVICE_ID_LIGHTGUN_PAUSE: u32 = 5;

/// Pass this to [`retro_video_refresh_t`] if rendering to hardware.
/// Passing NULL to [`retro_video_refresh_t`] is still a frame dupe as normal.
///
/// For some reason bindgen did not export this #define
pub const RETRO_HW_FRAME_BUFFER_VALID: *mut std::os::raw::c_void =
    -1_i32 as *mut std::os::raw::c_void;

#[derive(Debug, Default)]
pub struct InvalidEnumValue<T: Display>(T);

impl<T: Display> InvalidEnumValue<T> {
    pub fn new(value: T) -> Self {
        InvalidEnumValue(value)
    }
}

impl<T: Display> Display for InvalidEnumValue<T> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        writeln!(fmt, "Invalid enum value: {}", self.0)
    }
}
