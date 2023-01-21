/** This file gets included by the build.rs **/

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
