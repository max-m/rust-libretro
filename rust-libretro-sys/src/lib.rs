#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/favicon.png"
)]

use core::fmt::Display;

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

// Include the auto-generated "raw" bindings
include!(concat!(env!("OUT_DIR"), "/bindings_libretro.rs"));

// Include the Vulkan module, if it has been enabled
#[cfg(feature = "vulkan")]
pub mod vulkan;

// Include the auto-generated namespaced `retro` module
include!(concat!(env!("OUT_DIR"), "/bindings_namespaced.rs"));
