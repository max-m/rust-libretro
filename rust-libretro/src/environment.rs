//! Unsafe functions related to the libretro environment callback.
//! For safe versions have a look at the [`contexts`] module and
//! the context types you get in your core callbacks.

use crate::{
    contexts::*,
    error::{EnvironmentCallError, StringError},
    get_path_from_pointer, get_str_from_pointer, proc,
    sys::*,
    types::*,
};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CString},
    path::Path,
};

/// Gets a value from an environment callback.
///
/// The first value of the return type is the queried data,
/// the second value is the return value of the callback itself.
#[inline(always)]
pub unsafe fn get<T: Default>(
    callback: retro_environment_t,
    id: u32,
) -> Result<T, EnvironmentCallError> {
    get_mut(callback, id, Default::default())
}

/// Similar to [`get`] but uses zeroed memory instead of the [`Default`] trait.
#[inline(always)]
pub unsafe fn get_unchecked<T>(
    callback: retro_environment_t,
    id: u32,
) -> Result<T, EnvironmentCallError> {
    let data = std::mem::MaybeUninit::zeroed().assume_init();

    get_mut(callback, id, data)
}

/// Passes a value to the environment callback and returns the modified value.
///
/// The second value is the return value of the callback itself.
pub unsafe fn get_mut<T>(
    callback: retro_environment_t,
    id: u32,
    mut data: T,
) -> Result<T, EnvironmentCallError> {
    let callback = callback.ok_or(EnvironmentCallError::NullPointer("retro_environment_t"))?;

    match (callback)(id, (&mut data as *mut _) as *mut c_void) {
        true => Ok(data),
        false => Err(EnvironmentCallError::Failure),
    }
}

/// Helper function to query a string pointer and convert it into a [`Path`].
pub unsafe fn get_path<'a>(
    callback: retro_environment_t,
    id: u32,
) -> Result<&'a Path, EnvironmentCallError> {
    let ptr: *mut c_void = std::ptr::null_mut();
    let ptr = get_mut(callback, id, ptr)?;

    get_path_from_pointer(ptr as *const c_char).map_err(Into::into)
}

/// Helper function to query a nullable string pointer and convert it into [`Option<Path>`].
pub unsafe fn get_optional_path<'a>(
    callback: retro_environment_t,
    id: u32,
) -> Result<Option<&'a Path>, EnvironmentCallError> {
    let ptr: *mut c_void = std::ptr::null_mut();
    let ptr = get_mut(callback, id, ptr)?;
    if ptr.is_null() {
        return Ok(None);
    }

    get_path_from_pointer(ptr as *const c_char)
        .map(Some)
        .map_err(Into::into)
}

/// Passes a value to the environment callback.
///
/// Returns [`None`] if the environment callback hasn’t been set
/// and the return status of the callback otherwise.
#[inline(always)]
pub unsafe fn set<T: std::fmt::Debug>(
    callback: retro_environment_t,
    id: u32,
    value: T,
) -> Result<(), EnvironmentCallError> {
    set_ptr(callback, id, &value as *const _)
}

/// Passes a value (by a raw const pointer) to the environment callback.
///
/// Returns [`None`] if the environment callback hasn’t been set
/// and the return status of the callback otherwise.
pub unsafe fn set_ptr<T>(
    callback: retro_environment_t,
    id: u32,
    ptr: *const T,
) -> Result<(), EnvironmentCallError> {
    let callback = callback.ok_or(EnvironmentCallError::NullPointer("retro_environment_t"))?;

    match (callback)(id, ptr as *mut c_void) {
        true => Ok(()),
        false => Err(EnvironmentCallError::Failure),
    }
}

#[macro_export]
macro_rules! validate_bitflags {
    ($flags:ident, $ty:ty, $bits:expr) => {{
        let unchecked = unsafe { $flags::from_bits_unchecked($bits) };

        #[cfg(feature = "strict-bitflags")]
        {
            let truncated = $flags::from_bits_truncate($bits);

            if truncated.bits() == $bits {
                Ok(unchecked)
            } else {
                let known_bits = $flags::all().bits();
                let diff = unchecked.bits() & !known_bits;
                let unknown = format!("{diff:0width$b}", width = <$ty>::BITS as usize);
                let known = format!("{known_bits:0width$b}", width = <$ty>::BITS as usize);

                Err($crate::error::EnvironmentCallError::UnknownBits(
                    known, unknown,
                ))
            }
        }

        #[cfg(not(feature = "strict-bitflags"))]
        {
            Ok::<_, $crate::error::EnvironmentCallError>(unchecked)
        }
    }};
}

/* ========================================================================== *\
 *                    Environment callback implementations                    *
\* ========================================================================== */

/// Sets screen rotation of graphics.
#[proc::context(GenericContext)]
pub unsafe fn set_rotation(
    callback: retro_environment_t,
    rotation: Rotation,
) -> Result<(), EnvironmentCallError> {
    // const unsigned *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_ROTATION,
        rotation.get_env_value(),
    )
}

/// Boolean value whether or not the implementation should use overscan,
/// or crop away overscan.
#[deprecated(
    note = "This function is considered deprecated in favor of using core options to manage overscan in a more nuanced, core-specific way"
)]
#[proc::context(GenericContext)]
pub unsafe fn get_overscan(callback: retro_environment_t) -> Result<bool, EnvironmentCallError> {
    // bool *
    get(callback, RETRO_ENVIRONMENT_GET_OVERSCAN)
}

/// Boolean value whether or not frontend supports frame duping,
/// passing NULL to video frame callback.
#[proc::context(GenericContext)]
pub unsafe fn can_dupe(callback: retro_environment_t) -> Result<bool, EnvironmentCallError> {
    // bool *
    get(callback, RETRO_ENVIRONMENT_GET_CAN_DUPE)
}

/// Sets a message to be displayed in implementation-specific manner
/// for a certain amount of 'frames'.
/// Should not be used for trivial messages, which should simply be
/// logged via [`RETRO_ENVIRONMENT_GET_LOG_INTERFACE`] (or as a
/// fallback, stderr).
#[proc::context(GenericContext)]
pub unsafe fn set_message(
    callback: retro_environment_t,
    message: &str,
    frames: u32,
) -> Result<(), EnvironmentCallError> {
    // RetroArch copies the string interally,
    // so we don’t need to keep it around for longer than this call
    let msg = CString::new(message).map_err(StringError::from)?;

    // const struct retro_message *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_MESSAGE,
        retro_message {
            msg: msg.as_ptr(),
            frames,
        },
    )
}

/// Requests the frontend to shutdown.
/// Should only be used if game has a specific
/// way to shutdown the game from a menu item or similar.
#[proc::context(GenericContext)]
pub unsafe fn shutdown(callback: retro_environment_t) {
    // N/A (NULL)
    let _ = set_ptr(
        callback,
        RETRO_ENVIRONMENT_SHUTDOWN,
        std::ptr::null() as *const c_void,
    );
}

/// Gives a hint to the frontend how demanding this implementation
/// is on a system. E.g. reporting a level of 2 means
/// this implementation should run decently on all frontends
/// of level 2 and up.
///
/// It can be used by the frontend to potentially warn
/// about too demanding implementations.
///
/// The levels are "floating".
///
/// This function can be called on a per-game basis,
/// as certain games an implementation can play might be
/// particularly demanding.
#[proc::context(LoadGameContext)]
pub unsafe fn set_performance_level(
    callback: retro_environment_t,
    level: u8,
) -> Result<(), EnvironmentCallError> {
    // const unsigned *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL,
        level as u32,
    )
}

/// Returns the "system" directory of the frontend.
/// This directory can be used to store system specific
/// content such as BIOSes, configuration data, etc.
/// The returned value can be `NULL`.
/// If so, no such directory is defined,
/// and it's up to the implementation to find a suitable directory.
///
/// **NOTE**: Some cores used this folder also for "save" data such as
/// memory cards, etc, for lack of a better place to put it.
/// This is now discouraged, and if possible, cores should try to
/// use the new [`get_save_directory()`].
#[proc::context(GenericContext)]
pub unsafe fn get_system_directory<'a>(
    callback: retro_environment_t,
) -> Result<Option<&'a Path>, EnvironmentCallError> {
    // const char **
    get_optional_path(callback, RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY)
}

/// Sets the internal pixel format used by the implementation.
/// The default pixel format is [`retro_pixel_format::RETRO_PIXEL_FORMAT_0RGB1555`].
/// This pixel format however, is deprecated (see enum [`retro_pixel_format`]).
/// If the call returns `false`, the frontend does not support this pixel
/// format.
#[proc::context(LoadGameContext)]
#[proc::context(GetAvInfoContext)]
pub unsafe fn set_pixel_format<F: Into<retro_pixel_format>>(
    callback: retro_environment_t,
    format: F,
) -> Result<(), EnvironmentCallError> {
    // const enum retro_pixel_format *
    set(callback, RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, format.into())
}

/// Sets an array of retro_input_descriptors.
/// It is up to the frontend to present this in a usable way.
/// The array is terminated by retro_input_descriptor::description
/// being set to `NULL`.
/// This function can be called at any time, but it is recommended
/// to call it as early as possible.
#[proc::context(GenericContext)]
pub unsafe fn set_input_descriptors(
    callback: retro_environment_t,
    descriptors: &[retro_input_descriptor],
) -> Result<(), EnvironmentCallError> {
    // const struct retro_input_descriptor *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
        descriptors.as_ptr(),
    )
}

/// Sets a callback function used to notify core about keyboard events.
#[proc::context(GenericContext)]
pub unsafe fn set_keyboard_callback(
    callback: retro_environment_t,
    data: retro_keyboard_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_keyboard_callback *
    set(callback, RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK, data)
}

/// Sets an interface which frontend can use to eject and insert
/// disk images.
/// This is used for games which consist of multiple images and
/// must be manually swapped out by the user (e.g. PSX).
#[proc::context(GenericContext)]
pub unsafe fn set_disk_control_interface(
    callback: retro_environment_t,
    data: retro_disk_control_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_disk_control_callback *
    set(callback, RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE, data)
}

/// Sets an interface to let a libretro core render with
/// hardware acceleration.
///
/// If successful, libretro cores will be able to render to a
/// frontend-provided framebuffer.
/// The size of this framebuffer will be at least as large as
/// max_width/max_height provided in [`Core::on_get_av_info`].
/// If HW rendering is used, call either
/// [`RunContext::draw_hardware_frame`] or [`RunContext::dupe_frame`].
#[proc::context(LoadGameContext)]
pub unsafe fn set_hw_render(
    callback: retro_environment_t,
    data: retro_hw_render_callback,
) -> Result<retro_hw_render_callback, EnvironmentCallError> {
    // struct retro_hw_render_callback *
    get_mut(callback, RETRO_ENVIRONMENT_SET_HW_RENDER, data)
}

/// Allows the core to test whether [`RETRO_ENVIRONMENT_GET_VARIABLE`]
/// is supported by the frontend.
#[proc::context(GenericContext)]
pub unsafe fn supports_get_variable(callback: retro_environment_t) -> bool {
    // const struct retro_variable *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_GET_VARIABLE,
        std::ptr::null() as *const c_void,
    )
    .map(|_| true)
    .unwrap_or(false)
}

/// Interface to acquire user-defined information from environment
/// that cannot feasibly be supported in a multi-system way.
///
/// See also [`get_variable()`] and [`get_all_variables()`].
///
/// The `key` is set it should be set to a key which has already been set by
/// [`set_variables`] or [`set_core_options`].
///
/// If `key` is None, this call obtains the complete environment string
/// if more complex parsing is necessary.
/// The environment string is formatted as key-value pairs
/// delimited by semicolons as so:
/// `key1=value1;key2=value2;...`
///
/// Returns [`None`] if the variable could not be found.
#[proc::context(GenericContext)]
#[proc::context(OptionsChangedContext)]
#[allow(clippy::needless_lifetimes)]
pub unsafe fn get_variable_or_environment<'a>(
    callback: retro_environment_t,
    key: Option<&'a str>,
) -> Result<Option<&'a str>, EnvironmentCallError> {
    let key = if let Some(key) = key {
        Some(CString::new(key).map_err(StringError::from)?)
    } else {
        None
    };

    let var = retro_variable {
        key: key
            .as_ref() // we use `as_ref()` to keep the CString in scope
            .map(|key| key.as_ptr())
            .unwrap_or(std::ptr::null()),
        value: std::ptr::null(),
    };

    // struct retro_variable *
    let var = get_mut(callback, RETRO_ENVIRONMENT_GET_VARIABLE, var)?;

    if var.value.is_null() {
        return Ok(None);
    }

    let value =
        get_str_from_pointer(var.value as *const c_char).map_err(EnvironmentCallError::from)?;

    Ok(Some(value))
}

/// Interface to acquire user-defined information from environment
/// that cannot feasibly be supported in a multi-system way.
///
/// See also [`get_variable_or_environment()`] and [`get_all_variables()`].
///
/// The `key` should be set to a key which has already been set by
/// [`set_variables`] or [`set_core_options`].
///
/// Returns [`None`] if the variable could not be found.
#[proc::context(GenericContext)]
#[proc::context(OptionsChangedContext)]
#[allow(clippy::needless_lifetimes)]
pub unsafe fn get_variable<'a>(
    callback: retro_environment_t,
    key: &'a str,
) -> Result<Option<&'a str>, EnvironmentCallError> {
    get_variable_or_environment(callback, Some(key))
}

/// Interface to acquire user-defined information from environment
/// that cannot feasibly be supported in a multi-system way.
///
/// See also [`get_variable()`] and [`get_variable_or_environment()`].
///
/// Note: At the moment RetroArch (1.14.0) does not implement this feature.
///
/// When a frontend does not implement this feature, returns `true` in the callback
/// and sets the variable’s value to NULL, we return [`None`].
/// This should be the case for all frontends that support [`RETRO_ENVIRONMENT_GET_VARIABLE`]
/// and handle a NULL value as key properly.
/// If a frontend indicates missing support for this feature or treats a NULL key as error and
/// returns `false` in the callback, we return [`EnvironmentCallError::Failure`].
#[proc::context(GenericContext)]
#[proc::context(OptionsChangedContext)]
#[allow(clippy::needless_lifetimes)]
pub unsafe fn get_all_variables<'a>(
    callback: retro_environment_t,
) -> Result<Option<HashMap<&'a str, &'a str>>, EnvironmentCallError> {
    if let Some(mut env_str) = get_variable_or_environment(callback, None)? {
        let mut map = HashMap::new();

        if let Some(stripped) = env_str.strip_suffix(';') {
            env_str = stripped;
        }

        for pair in env_str.split(';') {
            let [key, value]: [&'a str; 2] = pair
                .splitn(2, '=')
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| EnvironmentCallError::KeyValueError(pair.to_owned()))?;

            map.insert(key, value);
        }

        return Ok(Some(map));
    }

    // The frontend returned a null pointer as value
    Ok(None)
}

/// Allows an implementation to signal the environment
/// which variables it might want to check for later using
/// [`get_variable`].
/// This allows the frontend to present these variables to
/// a user dynamically.
/// This should be called the first time as early as
/// possible (ideally in [`Core::on_set_environment`]).
/// Afterward it may be called again for the core to communicate
/// updated options to the frontend, but the number of core
/// options must not change from the number in the initial call.
///
/// The passed array of [`retro_variable`] structs must be
/// terminated by a
/// ```
/// # use rust_libretro_sys::retro_variable;
/// retro_variable {
///     key:   0 as *const libc::c_char,
///     value: 0 as *const libc::c_char,
/// }
/// # ;
/// ```
/// element.
/// [`retro_variable::key`] should be namespaced to not collide
/// with other implementations' keys. E.g. A core called
/// 'foo' should use keys named as `foo_option`.
/// [`retro_variable::value`] should contain a human readable
/// description of the key as well as a `|` delimited list
/// of expected values.
///
/// The number of possible options should be very limited,
/// i.e. it should be feasible to cycle through options
/// without a keyboard.
///
/// First entry should be treated as a default.
///
/// # Examples
/// ```
/// # use rust_libretro_sys::retro_variable;
/// retro_variable {
///     key:   b"foo_option" as *const u8 as *const libc::c_char,
///     value: b"Speed hack coprocessor X; false|true" as *const u8 as *const libc::c_char,
/// }
/// # ;
/// ```
///
/// Text before the first `;` is a description. This `;` must be
/// followed by a space, and followed by a list of possible
/// values split up with `|`.
///
/// Only strings are operated on. The possible values will
/// generally be displayed and stored as-is by the frontend.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_variables(
    callback: retro_environment_t,
    variables: &[retro_variable],
) -> Result<(), EnvironmentCallError> {
    // const struct retro_variable *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_VARIABLES,
        variables.as_ptr(),
    )
}

/// Result is set to [`true`] if some variables are updated by
/// frontend since last call to [`get_variable`].
#[proc::context(GenericContext)]
pub unsafe fn get_variable_update(
    callback: retro_environment_t,
) -> Result<bool, EnvironmentCallError> {
    // bool *
    get(callback, RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE)
}

/// Tell the frontend whether this Core can run without particular game data.
///
/// If true, the [`Core`] implementation supports calls to
/// [`Core::on_load_game`] with [`None`] as argument.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_support_no_game(
    callback: retro_environment_t,
    value: bool,
) -> Result<(), EnvironmentCallError> {
    // const bool *
    set(callback, RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME, value)
}

/// Retrieves the absolute path from where this libretro
/// implementation was loaded.
/// [`None`] is returned if the libretro was loaded statically
/// (i.e. linked statically to frontend), or if the path cannot be
/// determined.
/// Mostly useful in cooperation with [`set_support_no_game`] as assets can
/// be loaded without ugly hacks.
#[proc::context(GenericContext)]
pub unsafe fn get_libretro_path<'a>(
    callback: retro_environment_t,
) -> Result<Option<&'a Path>, EnvironmentCallError> {
    // const char **
    get_optional_path(callback, RETRO_ENVIRONMENT_GET_LIBRETRO_PATH)
}

/// Lets the core know how much time has passed since last
/// invocation of [`Core::on_run`].
/// The frontend can tamper with the timing to fake fast-forward,
/// slow-motion, frame stepping, etc.
/// In this case the delta time will use the reference value
/// in [`retro_frame_time_callback`].
#[proc::context(LoadGameContext)]
#[proc::context(LoadGameSpecialContext)]
pub unsafe fn set_frame_time_callback(
    callback: retro_environment_t,
    data: retro_frame_time_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_frame_time_callback *
    set(callback, RETRO_ENVIRONMENT_SET_FRAME_TIME_CALLBACK, data)
}

/// Sets an interface which is used to notify a libretro core about audio
/// being available for writing.
/// The callback can be called from any thread, so a core using this must
/// have a thread safe audio implementation.
///
/// It is intended for games where audio and video are completely
/// asynchronous and audio can be generated on the fly.
/// This interface is not recommended for use with emulators which have
/// highly synchronous audio.
///
/// The callback only notifies about writability; the libretro core still
/// has to call the normal audio callbacks
/// to write audio. The audio callbacks must be called from within the
/// notification callback.
/// The amount of audio data to write is up to the implementation.
/// Generally, the audio callback will be called continously in a loop.
///
/// Due to thread safety guarantees and lack of sync between audio and
/// video, a frontend  can selectively disallow this interface based on
/// internal configuration. A core using this interface must also
/// implement the "normal" audio interface.
///
/// A libretro core using [`set_audio_callback`] should also make use of
/// [`set_frame_time_callback`].
#[proc::context(GenericContext)]
pub unsafe fn set_audio_callback(
    callback: retro_environment_t,
    data: retro_audio_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_audio_callback *
    set(callback, RETRO_ENVIRONMENT_SET_AUDIO_CALLBACK, data)
}

/// Gets an interface which is used by a libretro core to set
/// state of rumble motors in controllers.
/// A strong and weak motor is supported, and they can be
/// controlled indepedently.
/// Should be called from either [`Core::on_init`] or [`Core::on_load_game`].
/// Should not be called from [`Core::on_set_environment`].
/// Returns false if rumble functionality is unavailable.
#[proc::context(InitContext)]
#[proc::context(LoadGameContext)]
pub unsafe fn get_rumble_interface(
    callback: retro_environment_t,
) -> Result<retro_rumble_interface, EnvironmentCallError> {
    // struct retro_rumble_interface *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE)
}

/// Gets a bitmask telling which device type are expected to be
/// handled properly in a call to [`retro_input_state_t`].
/// Devices which are not handled or recognized always return
/// 0 in [`retro_input_state_t`].
/// Example bitmask: `RetroDevice::JOYPAD | RetroDevice::ANALOG`.
#[proc::context(RunContext)]
pub unsafe fn get_input_device_capabilities(
    callback: retro_environment_t,
) -> Result<RetroDevice, EnvironmentCallError> {
    // uint64_t *
    let caps = get::<u64>(callback, RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES)?;

    // I’m not entirely sure why this call returns a 64 bit value when the `RETRO_DEVICE_MASK`
    // allows only eight distinct types.
    let caps = caps as u8;

    validate_bitflags!(RetroDevice, u8, caps)
}

/// Gets access to the sensor interface.
/// The purpose of this interface is to allow
/// setting state related to sensors such as polling rate,
/// enabling/disable it entirely, etc.
/// Reading sensor state is done via the normal
/// input_state_callback API.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_sensor_interface(
    callback: retro_environment_t,
) -> Result<retro_sensor_interface, EnvironmentCallError> {
    // const struct retro_sensor_interface *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE)
}

/// Gets an interface to a video camera driver.
/// A libretro core can use this interface to get access to a
/// video camera.
/// New video frames are delivered in a callback in same
/// thread as [`Core::on_run`].
///
/// [`get_camera_interface`] should be called in [`Core::on_load_game`].
///
/// Depending on the camera implementation used, camera frames
/// will be delivered as a raw framebuffer,
/// or as an OpenGL texture directly.
///
/// The core has to tell the frontend here which types of
/// buffers can be handled properly.
/// An OpenGL texture can only be handled when using a
/// libretro GL core ([`set_hw_render`]).
/// It is recommended to use a libretro GL core when
/// using camera interface.
///
/// The camera is not started automatically. The retrieved start/stop
/// functions must be used to explicitly
#[proc::context(LoadGameContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_camera_interface(
    callback: retro_environment_t,
    data: retro_camera_callback,
) -> Result<retro_camera_callback, EnvironmentCallError> {
    // struct retro_camera_callback *
    get_mut(callback, RETRO_ENVIRONMENT_GET_CAMERA_INTERFACE, data)
}

/// Gets an interface for logging. This is useful for
/// logging in a cross-platform way
/// as certain platforms cannot use `stderr` for logging.
/// It also allows the frontend to
/// show logging information in a more suitable way.
/// If this interface is not used, libretro cores should
/// log to `stderr` as desired.
#[proc::context(GenericContext)]
pub unsafe fn get_log_callback(
    callback: retro_environment_t,
) -> Result<retro_log_callback, EnvironmentCallError> {
    // struct retro_log_callback *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_LOG_INTERFACE)
}

/// Gets an interface for performance counters. This is useful
/// for performance logging in a cross-platform way and for detecting
/// architecture-specific features, such as SIMD support.
#[proc::context(GenericContext)]
pub unsafe fn get_perf_interface(
    callback: retro_environment_t,
) -> Result<retro_perf_callback, EnvironmentCallError> {
    // struct retro_perf_callback *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_PERF_INTERFACE)
}

/// Gets access to the location interface.
/// The purpose of this interface is to be able to retrieve
/// location-based information from the host device,
/// such as current latitude / longitude.
#[proc::context(GenericContext)]
pub unsafe fn get_location_callback(
    callback: retro_environment_t,
) -> Result<retro_location_callback, EnvironmentCallError> {
    // struct retro_location_callback *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_LOCATION_INTERFACE)
}

/// Returns the "core assets" directory of the frontend.
/// This directory can be used to store specific assets that the
/// core relies upon, such as art assets,
/// input data, etc etc.
/// The returned value can be [`None`].
/// If so, no such directory is defined,
/// and it's up to the implementation to find a suitable directory.
#[proc::context(GenericContext)]
pub unsafe fn get_core_assets_directory<'a>(
    callback: retro_environment_t,
) -> Result<Option<&'a Path>, EnvironmentCallError> {
    // const char **
    get_optional_path(callback, RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY)
}

/// Returns the "save" directory of the frontend, unless there is no
/// save directory available. The save directory should be used to
/// store SRAM, memory cards, high scores, etc, if the libretro core
/// cannot use the regular memory interface ([`Core::get_memory_data`]).
///
/// If the frontend cannot designate a save directory, it will return
/// [`None`] to indicate that the core should attempt to operate without a
/// save directory set.
///
/// NOTE: early libretro cores used the system directory for save
/// files. Cores that need to be backwards-compatible can still check
/// [`get_system_directory`].
#[proc::context(GenericContext)]
pub unsafe fn get_save_directory<'a>(
    callback: retro_environment_t,
) -> Result<Option<&'a Path>, EnvironmentCallError> {
    // const char **
    get_optional_path(callback, RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY)
}

/// Sets a new av_info structure.
///
/// This should **only** be used if the core is completely altering the
/// internal resolutions, aspect ratios, timings, sampling rate, etc.
/// Calling this can require a full reinitialization of video/audio
/// drivers in the frontend,
///
/// so it is important to call it very sparingly, and usually only with
/// the users explicit consent.
/// An eventual driver reinitialize will happen so that video and
/// audio callbacks
/// happening after this call within the same [`Core::on_run`] call will
/// target the newly initialized driver.
///
/// This callback makes it possible to support configurable resolutions
/// in games, which can be useful to
/// avoid setting the "worst case" in max_width/max_height.
///
/// **HIGHLY RECOMMENDED**
/// Do not call this callback every time
/// resolution changes in an emulator core if it's
/// expected to be a temporary change, for the reasons of possible
/// driver reinitialization.
/// This call is not a free pass for not trying to provide
/// correct values in [`Core::on_get_av_info`]. If you need to change
/// things like aspect ratio or nominal width/height,
/// use [`set_game_geometry`], which is a softer variant
/// of [`set_system_av_info`].
///
/// If this returns [`false`], the frontend does not acknowledge a
/// changed [`retro_system_av_info`] struct.
#[proc::context(RunContext)]
pub unsafe fn set_system_av_info(
    callback: retro_environment_t,
    av_info: retro_system_av_info,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_system_av_info *
    set(callback, RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO, av_info)
}

/// Allows a libretro core to announce support for the
/// [`retro_get_proc_address_interface`] interface.
/// This interface allows for a standard way to extend libretro where
/// use of environment calls are too indirect,
/// e.g. for cases where the frontend wants to call directly into the core.
///
/// If a core wants to expose this interface, [`set_proc_address_callback`]
/// **MUST** be called from within [`Core::on_set_environment`].
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_proc_address_callback(
    callback: retro_environment_t,
    data: retro_get_proc_address_interface,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_get_proc_address_interface *
    set(callback, RETRO_ENVIRONMENT_SET_PROC_ADDRESS_CALLBACK, data)
}

/// This environment call introduces the concept of libretro "subsystems".
/// A subsystem is a variant of a libretro core which supports
/// different kinds of games.
/// The purpose of this is to support e.g. emulators which might
/// have special needs, e.g. Super Nintendo's Super GameBoy, Sufami Turbo.
/// It can also be used to pick among subsystems in an explicit way
/// if the libretro implementation is a multi-system emulator itself.
///
/// Loading a game via a subsystem is done with [`Core::on_load_game_special`],
/// and this environment call allows a libretro core to expose which
/// subsystems are supported for use with [`Core::on_load_game_special`].
/// A core passes an array of [`retro_game_info`] which is terminated
/// with a zeroed out [`retro_game_info`] struct.
///
/// If a core wants to use this functionality, [`set_subsystem_info`]
/// **MUST** be called from within [`Core::on_set_environment`].
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_subsystem_info(
    callback: retro_environment_t,
    data: &[retro_subsystem_info],
) -> Result<(), EnvironmentCallError> {
    // const struct retro_subsystem_info *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO,
        data.as_ptr(),
    )
}

/// This environment call lets a libretro core tell the frontend
/// which controller subclasses are recognized in calls to
/// [`Core::on_set_controller_port_device`].
///
/// Some emulators such as Super Nintendo support multiple lightgun
/// types which must be specifically selected from. It is therefore
/// sometimes necessary for a frontend to be able to tell the core
/// about a special kind of input device which is not specifcally
/// provided by the Libretro API.
///
/// In order for a frontend to understand the workings of those devices,
/// they must be defined as a specialized subclass of the generic device
/// types already defined in the libretro API.
///
/// The core must pass an **array** of `const struct` [`retro_controller_info`] which
/// is **terminated with a blanked out struct**.
/// Each element of the [`retro_controller_info`] struct corresponds to the
/// ascending port index that is passed to [`Core::on_set_controller_port_device`]
/// when that function is called to indicate to the core that the frontend has
/// changed the active device subclass.
/// **SEE ALSO**: [`Core::on_set_controller_port_device`]
///
/// The ascending input port indexes provided by the core in the struct
/// are generally presented by frontends as ascending User # or Player #,
/// such as Player 1, Player 2, Player 3, etc. Which device subclasses are
/// supported can vary per input port.
///
/// The first inner element of each entry in the [`retro_controller_info`] array
/// is a [`retro_controller_description`] struct that specifies the names and
/// codes of all device subclasses that are available for the corresponding
/// User or Player, beginning with the generic Libretro device that the
/// subclasses are derived from. The second inner element of each entry is the
/// total number of subclasses that are listed in the [`retro_controller_description`].
///
/// NOTE: Even if special device types are set in the libretro core,
/// libretro should only poll input based on the base input device types.
#[proc::context(GenericContext)]
pub unsafe fn set_controller_info(
    callback: retro_environment_t,
    data: &[retro_controller_info],
) -> Result<(), EnvironmentCallError> {
    // const struct retro_controller_info *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_CONTROLLER_INFO,
        data.as_ptr(),
    )
}

/// This environment call lets a libretro core tell the frontend
/// about the memory maps this core emulates.
/// This can be used to implement, for example, cheats in a core-agnostic way.
///
/// Should only be used by emulators; it doesn't make much sense for
/// anything else.
/// It is recommended to expose all relevant pointers through
/// retro_get_memory_* as well.
///
/// Can be called from [`Core::on_init`] and [`Core::on_load_game`].
#[proc::context(InitContext)]
#[proc::context(LoadGameContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn set_memory_maps(
    callback: retro_environment_t,
    data: retro_memory_map,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_memory_map *
    set(callback, RETRO_ENVIRONMENT_SET_MEMORY_MAPS, data)
}

/// Sets a new game_geometry structure.
///
/// This environment call is similar to [`set_system_av_info`] for changing
/// video parameters, but provides a guarantee that drivers will not be
/// reinitialized.
///
/// The purpose of this call is to allow a core to alter nominal
/// width/heights as well as aspect ratios on-the-fly, which can be
/// useful for some emulators to change in run-time.
///
/// max_width/max_height arguments are ignored and cannot be changed
/// with this call as this could potentially require a reinitialization or a
/// non-constant time operation.
/// If max_width/max_height are to be changed, [`set_system_av_info`] is required.
///
/// A frontend must guarantee that this environment call completes in
/// constant time.
#[proc::context(RunContext)]
pub unsafe fn set_game_geometry(
    callback: retro_environment_t,
    geometry: retro_game_geometry,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_game_geometry *
    set(callback, RETRO_ENVIRONMENT_SET_GEOMETRY, geometry)
}

/// Returns the specified username of the frontend, if specified by the user.
/// This username can be used as a nickname for a core that has online facilities
/// or any other mode where personalization of the user is desirable.
/// The returned value can be [`None`].
/// If this environment callback is used by a core that requires a valid username,
/// a default username should be specified by the core.
#[proc::context(GenericContext)]
pub unsafe fn get_username<'a>(
    callback: retro_environment_t,
) -> Result<&'a str, EnvironmentCallError> {
    let ptr: *mut c_void = std::ptr::null_mut();

    // const char **
    let ptr = get_mut(callback, RETRO_ENVIRONMENT_GET_USERNAME, ptr)?;

    get_str_from_pointer(ptr as *const c_char).map_err(Into::into)
}

/// Returns the language of the frontend, if specified by the user.
/// It can be used by the core for localization purposes.
#[proc::context(GenericContext)]
pub unsafe fn get_language(
    callback: retro_environment_t,
) -> Result<retro_language, EnvironmentCallError> {
    // unsigned *
    let id = get::<u32>(callback, RETRO_ENVIRONMENT_GET_LANGUAGE)?;

    if id < retro_language::RETRO_LANGUAGE_LAST as u32 {
        // This is safe because all values from 0 to RETRO_LANGUAGE_LAST have defined values
        return Ok(std::mem::transmute(id));
    }

    Err(EnvironmentCallError::InvalidEnumValue(id.to_string()))
}

/// Returns a preallocated framebuffer which the core can use for rendering
/// the frame into when not using [`set_hw_render`].
/// The framebuffer returned from this call must not be used
/// after the current call to [`Core::on_run`] returns.
///
/// The goal of this call is to allow zero-copy behavior where a core
/// can render directly into video memory, avoiding extra bandwidth cost by copying
/// memory from core to video memory.
///
/// If this call succeeds and the core renders into it,
/// the framebuffer pointer and pitch can be passed to [`RunContext::draw_framebuffer`].
/// If the buffer from [`get_current_software_framebuffer`] is to be used,
/// the core must pass the exact
/// same pointer as returned by [`get_current_software_framebuffer`];
/// i.e. passing a pointer which is offset from the
/// buffer is undefined. The width, height and pitch parameters
/// must also match exactly to the values obtained from [`get_current_software_framebuffer`].
///
/// It is possible for a frontend to return a different pixel format
/// than the one used in [`set_pixel_format`]. This can happen if the frontend
/// needs to perform conversion.
///
/// It is still valid for a core to render to a different buffer
/// even if [`get_current_software_framebuffer`] succeeds.
///
/// A frontend must make sure that the pointer obtained from this function is
/// writeable (and readable).
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_current_software_framebuffer(
    callback: retro_environment_t,
    data: retro_framebuffer,
) -> Result<retro_framebuffer, EnvironmentCallError> {
    // struct retro_framebuffer *
    get_mut(
        callback,
        RETRO_ENVIRONMENT_GET_CURRENT_SOFTWARE_FRAMEBUFFER,
        data,
    )
}

/// Returns an API specific rendering interface for accessing API specific data.
/// Not all HW rendering APIs support or need this.
/// The contents of the returned pointer is specific to the rendering API
/// being used. See the various headers like libretro_vulkan.h, etc.
///
/// [`get_hw_render_interface`] cannot be called before [`retro_hw_context_reset_callback`] has been called.
///
/// Similarly, after [`retro_hw_context_destroyed_callback`] returns, the contents of the HW_RENDER_INTERFACE are invalidated.
///
/// **TODO:** Set a status flag in [`retro_hw_context_reset_callback`] and [`retro_hw_context_destroyed_callback`] to force the mentioned call restrictions.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_hw_render_interface(
    callback: retro_environment_t,
) -> Result<retro_hw_render_interface, EnvironmentCallError> {
    // const struct retro_hw_render_interface **
    let ptr: *const retro_hw_render_interface = get_mut(
        callback,
        RETRO_ENVIRONMENT_GET_HW_RENDER_INTERFACE,
        std::ptr::null(),
    )?;

    if ptr.is_null() {
        return Err(EnvironmentCallError::NullPointer(
            "retro_hw_render_interface",
        ));
    }

    Ok(*ptr)
}

/// See [`get_hw_render_interface`].
#[cfg(feature = "vulkan")]
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_hw_render_interface_vulkan(
    callback: retro_environment_t,
) -> Result<retro_hw_render_interface_vulkan, EnvironmentCallError> {
    // const struct retro_hw_render_interface_vulkan **
    let ptr: *const retro_hw_render_interface_vulkan = get_mut(
        callback,
        RETRO_ENVIRONMENT_GET_HW_RENDER_INTERFACE,
        std::ptr::null(),
    )?;

    if ptr.is_null() {
        return Err(EnvironmentCallError::NullPointer(
            "retro_hw_render_interface_vulkan",
        ));
    }

    Ok((*ptr).clone())
}

/// If true, the Core implementation supports achievements.
///
/// Either via memory descriptors set with [`RETRO_ENVIRONMENT_SET_MEMORY_MAPS`]
/// or via [`Core::get_memory_data`] / [`Core::get_memory_size`].
#[proc::context(InitContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn set_support_achievements(
    callback: retro_environment_t,
    value: bool,
) -> Result<(), EnvironmentCallError> {
    // const bool *
    set(callback, RETRO_ENVIRONMENT_SET_SUPPORT_ACHIEVEMENTS, value)
}

/// Sets an interface which lets the libretro core negotiate with frontend how a context is created.
/// The semantics of this interface depends on which API is used in [`set_hw_render`] earlier.
/// This interface will be used when the frontend is trying to create a HW rendering context,
/// so it will be used after [`set_hw_render`], but before the context_reset callback.
#[proc::context(LoadGameContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn set_hw_render_context_negotiation_interface(
    callback: retro_environment_t,
    interface: &retro_hw_render_context_negotiation_interface,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_hw_render_context_negotiation_interface *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE,
        interface,
    )
}

/// Sets quirk flags associated with serialization.
/// The frontend will zero any flags it doesn't recognize or support.
///
/// **Should be set in either [`Core::on_init`] or [`Core::on_load_game`], but not both.**
#[proc::context(InitContext)]
#[proc::context(LoadGameContext)]
pub unsafe fn set_serialization_quirks(
    callback: retro_environment_t,
    quirks: SerializationQuirks,
) -> Result<(), EnvironmentCallError> {
    // uint64_t *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS,
        quirks.bits() as u64,
    )
}

/// The frontend will try to use a 'shared' hardware context (mostly applicable
/// to OpenGL) when a hardware context is being set up.
///
/// Returns [`true`] if the frontend supports shared hardware contexts and [`false`]
/// if the frontend does not support shared hardware contexts.
///
/// This will do nothing on its own until `SET_HW_RENDER` environment callbacks are
/// being used.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn set_hw_shared_context(
    callback: retro_environment_t,
) -> Result<(), EnvironmentCallError> {
    // N/A (null) *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_HW_SHARED_CONTEXT,
        std::ptr::null() as *const c_void,
    )
}

/// Gets access to the VFS interface.
/// VFS presence needs to be queried prior to load_game or any
/// get_system/save/other_directory being called to let front end know
/// core supports VFS before it starts handing out paths.
/// It is recomended to do so in [`Core::on_set_environment`].
#[proc::context(SetEnvironmentContext)]
#[proc::unstable(feature = "env-commands")]
pub fn get_vfs_interface(
    callback: retro_environment_t,
    data: retro_vfs_interface_info,
) -> Result<retro_vfs_interface_info, EnvironmentCallError> {
    // struct retro_vfs_interface_info *
    get_mut(callback, RETRO_ENVIRONMENT_GET_VFS_INTERFACE, data)
}

/// Gets an interface which is used by a libretro core to set state of LEDs.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub fn get_led_interface(
    callback: retro_environment_t,
) -> Result<retro_led_interface, EnvironmentCallError> {
    // struct retro_led_interface *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_LED_INTERFACE)
}

/// Tells the core if the frontend wants audio or video.
/// If disabled, the frontend will discard the audio or video,
/// so the core may decide to skip generating a frame or generating audio.
/// This is mainly used for increasing performance.
///
/// See [`AudioVideoEnable`] for descriptions of the flags.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_audio_video_enable(
    callback: retro_environment_t,
) -> Result<AudioVideoEnable, EnvironmentCallError> {
    // int *
    let info = get(callback, RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE)?;

    validate_bitflags!(AudioVideoEnable, u32, info)
}

/// Returns a MIDI interface that can be used for raw data I/O.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub fn get_midi_interface(
    callback: retro_environment_t,
) -> Result<retro_midi_interface, EnvironmentCallError> {
    // struct retro_midi_interface **
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_MIDI_INTERFACE)
}

/// Boolean value that indicates whether or not the frontend is in fastforwarding mode.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_fastforwarding(
    callback: retro_environment_t,
) -> Result<(), EnvironmentCallError> {
    // bool *
    get(callback, RETRO_ENVIRONMENT_GET_FASTFORWARDING)
}

/// Float value that lets us know what target refresh rate
/// is curently in use by the frontend.
///
/// The core can use the returned value to set an ideal
/// refresh rate/framerate.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_target_refresh_rate(
    callback: retro_environment_t,
) -> Result<f32, EnvironmentCallError> {
    // float *
    get(callback, RETRO_ENVIRONMENT_GET_TARGET_REFRESH_RATE)
}

/// Boolean value that indicates whether or not the frontend supports
/// input bitmasks being returned by [`retro_input_state_t`]. The advantage
/// of this is that [`retro_input_state_t`] has to be only called once to
/// grab all button states instead of multiple times.
///
/// If it returns true, you can pass [`RETRO_DEVICE_ID_JOYPAD_MASK`] as `id`
/// to [`retro_input_state_t`] (make sure `device` is set to [`RETRO_DEVICE_JOYPAD`]).
/// It will return a bitmask of all the digital buttons.
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_input_bitmasks(
    callback: retro_environment_t,
) -> Result<(), EnvironmentCallError> {
    // bool *
    // get(callback, RETRO_ENVIRONMENT_GET_INPUT_BITMASKS)

    // RetroArch uses the callback’s return value instead
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_GET_INPUT_BITMASKS,
        std::ptr::null() as *const c_void,
    )
}

/// The returned value is the API version number of the core options
/// interface supported by the frontend.
/// If the underlying callback failed, API version is assumed to be 0.
///
/// In legacy code, core options are set by passing an array of
/// retro_variable structs to [`set_variables`].
/// This may be still be done regardless of the core options
/// interface version.
///
/// If version is `>= 1` however, core options may instead be set by
/// passing an array of [`retro_core_option_definition`] structs to
/// [`set_core_options`], or a 2D array of
/// [`retro_core_option_definition`] structs to [`RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL`].
/// This allows the core to additionally set option sublabel information
/// and/or provide localisation support.
///
/// If version is `>= 2,` core options may instead be set by passing
/// a `retro_core_options_v2` struct to [`set_core_options_v2`],
/// or an array of [`retro_core_options_v2`] structs to
/// [`set_core_options_v2_intl`]. This allows the core
/// to additionally set optional core option category information
/// for frontends with core option category support.
#[proc::context(GenericContext)]
pub unsafe fn get_core_options_version(callback: retro_environment_t) -> u32 {
    // unsigned *
    get(callback, RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION).unwrap_or(0)
}

/// Checks whether the frontend supports the [`set_core_options`] interface.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn supports_set_core_options(callback: retro_environment_t) -> bool {
    get_core_options_version(callback) >= 1
}

/// Checks whether the frontend supports the [`set_core_options_v2`] interface.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn supports_set_core_options_v2(callback: retro_environment_t) -> bool {
    get_core_options_version(callback) >= 2
}

/// Allows an implementation to signal the environment
/// which variables it might want to check for later using
/// [`get_variable`].
/// This allows the frontend to present these variables to
/// a user dynamically.
/// This should only be called if [`get_core_options_version`]
/// returns an API version of >= 1.
/// This should be called instead of [`set_variables`].
/// This should be called the first time as early as
/// possible (ideally in [`Core::on_set_environment`]).
/// Afterwards it may be called again for the core to communicate
/// updated options to the frontend, but the number of core
/// options must not change from the number in the initial call.
///
/// 'data' points to an array of [`retro_core_option_definition`] structs
/// terminated by a `{ NULL, NULL, NULL, {{0}}, NULL }` element.
/// [`retro_core_option_definition::key`] should be namespaced to not collide
/// with other implementations' keys. e.g. A core called
/// `foo` should use keys named as `foo_option`.
/// [`retro_core_option_definition::desc`] should contain a human readable
/// description of the key.
/// [`retro_core_option_definition::info`] should contain any additional human
/// readable information text that a typical user may need to
/// understand the functionality of the option.
/// [`retro_core_option_definition::values`] is an array of [`retro_core_option_value`]
/// structs terminated by a `{ NULL, NULL }` element.
/// > `retro_core_option_definition::values[index].value` is an expected option
///   value.
/// > `retro_core_option_definition::values[index].label` is a human readable
///   label used when displaying the value on screen. If `NULL`,
///   the value itself is used.
/// [`retro_core_option_definition::default_value`] is the default core option
/// setting. It must match one of the expected option values in the
/// [`retro_core_option_definition::values`] array. If it does not, or the
/// default value is `NULL`, the first entry in the
/// [`retro_core_option_definition::values`] array is treated as the default.
///
/// The number of possible option values should be very limited,
/// and must be less than [`RETRO_NUM_CORE_OPTION_VALUES_MAX`].
/// i.e. it should be feasible to cycle through options
/// without a keyboard.
///
/// # Examples
/// ```c
/// {
///     "foo_option",
///     "Speed hack coprocessor X",
///     "Provides increased performance at the expense of reduced accuracy",
///     {
///         { "false",    NULL },
///         { "true",     NULL },
///         { "unstable", "Turbo (Unstable)" },
///         { NULL, NULL },
///     },
///     "false"
/// }
/// ```
///
/// Only strings are operated on. The possible values will
/// generally be displayed and stored as-is by the frontend.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_core_options(
    callback: retro_environment_t,
    options: &[retro_core_option_definition],
) -> Result<(), EnvironmentCallError> {
    if !supports_set_core_options(callback) {
        let supported = get_core_options_version(callback);

        return Err(EnvironmentCallError::Unsupported(
            format!("set_core_options() requires at least API version 1, but the frontend reports support for version {supported}."),
        ));
    }

    // const struct retro_core_option_definition **
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS,
        options.as_ptr(),
    )
}

/// Allows an implementation to signal the environment
/// which variables it might want to check for later using
/// [`get_variable`].
/// This allows the frontend to present these variables to
/// a user dynamically.
///
/// This should only be called if [`get_core_options_version`]
/// returns an API version of `>= 2`.
///
/// This should be called instead of [`set_variables`].
///
/// This should be called instead of [`set_core_options`].
///
/// This should be called the first time as early as
/// possible (ideally in [`Core::on_set_environment`]).
///
/// Afterwards it may be called again for the core to communicate
/// updated options to the frontend, but the number of core
/// options must not change from the number in the initial call.
///
/// If [`get_core_options_version`] returns an API
/// version of `>= 2`, this callback is guaranteed to succeed
/// (i.e. callback return value does not indicate success)
///
/// If callback returns [`true`], frontend has core option category
/// support.
///
/// If callback returns [`false`], frontend does not have core option
/// category support.
///
/// 'data' points to a [`retro_core_options_v2`] struct, containing
/// of two pointers:
/// - [`retro_core_options_v2::categories`] is an array of
///   [`retro_core_option_v2_category`] structs terminated by a
///   `{ NULL, NULL, NULL }` element. If [`retro_core_options_v2::categories`]
///   is `NULL , all core options will have no category and will be shown
///   at the top level of the frontend core option interface. If frontend
///   does not have core option category support, categories array will
///   be ignored.
/// - [`retro_core_options_v2::definitions`] is an array of
///   [`retro_core_option_v2_definition`] structs terminated by a
///   `{ NULL, NULL, NULL, NULL, NULL, NULL, {{0}}, NULL }`
///   element.
///
/// ## [`retro_core_option_v2_category`] notes:
///
/// - [`retro_core_option_v2_category::key`] should contain string
///   that uniquely identifies the core option category. Valid
///   key characters are `[a-z, A-Z, 0-9, _, -]`.
///
///   Namespace collisions with other implementations' category
///   keys are permitted.
/// - [`retro_core_option_v2_category::desc`] should contain a human
///   readable description of the category key.
/// - [`retro_core_option_v2_category::info`] should contain any
///   additional human readable information text that a typical
///   user may need to understand the nature of the core option
///   category.
///
/// ### Examples
/// ```c
/// {
///     "advanced_settings",
///     "Advanced",
///     "Options affecting low-level emulation performance and accuracy."
/// }
/// ```
///
/// ## [`retro_core_option_v2_definition`] notes:
///
/// - [`retro_core_option_v2_definition::key`] should be namespaced to not
///   collide with other implementations' keys. e.g. A core called
///   `foo` should use keys named as `foo_option`. Valid key characters
///   are `[a-z, A-Z, 0-9, _, -]`.
/// - [`retro_core_option_v2_definition::desc`] should contain a human readable
///   description of the key. Will be used when the frontend does not
///   have core option category support. Examples: `Aspect Ratio` or
///   `Video > Aspect Ratio`.
/// - [`retro_core_option_v2_definition::desc_categorized`] should contain a
///   human readable description of the key, which will be used when
///   frontend has core option category support. Example: `Aspect Ratio`,
///   where associated [`retro_core_option_v2_category::desc`] is `Video`.
///
///   If empty or `NULL`, the string specified by
///   [`retro_core_option_v2_definition::desc`] will be used instead.
///
///   [`retro_core_option_v2_definition::desc_categorized`] will be ignored
///   if [`retro_core_option_v2_definition::category_key`] is empty or `NULL`.
/// - [`retro_core_option_v2_definition::info`] should contain any additional
///   human readable information text that a typical user may need to
///   understand the functionality of the option.
/// - [`retro_core_option_v2_definition::info_categorized`] should contain
///   any additional human readable information text that a typical user
///   may need to understand the functionality of the option, and will be
///   used when frontend has core option category support. This is provided
///   to accommodate the case where info text references an option by
///   name/desc, and the desc/desc_categorized text for that option differ.
///
///   If empty or `NULL`, the string specified by
///   [`retro_core_option_v2_definition::info`] will be used instead.
///
///   [`retro_core_option_v2_definition::info_categorized`] will be ignored
///   if [`retro_core_option_v2_definition::category_key`] is empty or `NULL`.
/// - [`retro_core_option_v2_definition::category_key`] should contain a
///   category identifier (e.g. `video` or `audio`) that will be
///   assigned to the core option if frontend has core option category
///   support. A categorized option will be shown in a subsection/
///   submenu of the frontend core option interface.
///
///   If key is empty or `NULL`, or if key does not match one of the
///   [`retro_core_option_v2_category::key`] values in the associated
///   [`retro_core_option_v2_category`] array, option will have no category
///   and will be shown at the top level of the frontend core option
///   interface.
/// - [`retro_core_option_v2_definition::values`] is an array of
///   retro_core_option_value structs terminated by a `{ NULL, NULL }`
///   element.
///
///     - [`retro_core_option_v2_definition::values[index].value`](retro_core_option_value::value) is an
///     expected option value.
///
///     - [`retro_core_option_v2_definition::values[index].label`](retro_core_option_value::label) is a
///     human readable label used when displaying the value on screen.
///     If `NULL`, the value itself is used.
/// - [`retro_core_option_v2_definition::default_value`] is the default
///   core option setting.
///
///   It must match one of the expected option
///   values in the [`retro_core_option_v2_definition::values`] array.
///
///   If it does not, or the default value is `NULL`, the first entry in the
///   [`retro_core_option_v2_definition::values`] array is treated as the
///   default.
///
/// The number of possible option values should be very limited,
/// and must be less than [`RETRO_NUM_CORE_OPTION_VALUES_MAX`].
/// i.e. it should be feasible to cycle through options
/// without a keyboard.
///
/// ### Examples
///
/// - Uncategorized:
///
///```c
/// {
///     "foo_option",
///     "Speed hack coprocessor X",
///     NULL,
///     "Provides increased performance at the expense of reduced accuracy.",
///     NULL,
///     NULL,
///     {
///         { "false",    NULL },
///         { "true",     NULL },
///         { "unstable", "Turbo (Unstable)" },
///         { NULL, NULL },
///     },
///     "false"
/// }
///```
///
/// - Categorized:
///
///```c
/// {
///     "foo_option",
///     "Advanced > Speed hack coprocessor X",
///     "Speed hack coprocessor X",
///     "Setting 'Advanced > Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
///     "Setting 'Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
///     "advanced_settings",
///     {
///         { "false",    NULL },
///         { "true",     NULL },
///         { "unstable", "Turbo (Unstable)" },
///         { NULL, NULL },
///     },
///     "false"
/// }
///```
///
/// Only strings are operated on. The possible values will
/// generally be displayed and stored as-is by the frontend.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_core_options_v2(
    callback: retro_environment_t,
    options: &retro_core_options_v2,
) -> Result<bool, EnvironmentCallError> {
    if !supports_set_core_options_v2(callback) {
        let supported = get_core_options_version(callback);

        return Err(EnvironmentCallError::Unsupported(
            format!("set_core_options_v2() requires at least API version 2, but the frontend reports support for version {supported}."),
        ));
    }

    // const struct retro_core_options_v2 *
    let result = set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2,
        options as *const _,
    );

    match result {
        Ok(()) => Ok(true),
        Err(EnvironmentCallError::Failure) => Ok(false),
        Err(err) => Err(err),
    }
}

/// Allows an implementation to signal the environment
/// which variables it might want to check for later using
/// [`get_variable`].
///
/// This allows the frontend to present these variables to
/// a user dynamically.
///
/// This should only be called if [`get_core_options_version`]
/// returns an API version of `>= 1`.
///
/// This should be called instead of [`set_variables`].
///
/// This should be called instead of [`set_core_options`].
///
/// This should be called the first time as early as
/// possible (ideally in [`Core::on_set_environment`]).
///
/// Afterwards it may be called again for the core to communicate
/// updated options to the frontend, but the number of core
/// options must not change from the number in the initial call.
///
/// This is fundamentally the same as [`set_core_options`],
/// with the addition of localisation support. The description of
/// [`set_core_options`] callback should be consulted for further details.
///
/// 'data' points to a [`retro_core_options_intl`] struct.
///
/// [`retro_core_options_intl::us`] is a pointer to an array of
/// [`retro_core_option_definition`] structs defining the US English
/// core options implementation. It must point to a valid array.
///
/// [`retro_core_options_intl::local`] is a pointer to an array of
/// [`retro_core_option_definition`] structs defining core options for
/// the current frontend language. It may be `NULL` (in which case
/// [`retro_core_options_intl::us`] is used by the frontend). Any items
/// missing from this array will be read from [`retro_core_options_intl::us`]
/// instead.
///
/// NOTE: Default core option values are always taken from the
/// [`retro_core_options_intl::us`] array. Any default values in
/// [`retro_core_options_intl::local`] array will be ignored.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_core_options_intl(
    callback: retro_environment_t,
    options: retro_core_options_intl,
) -> Result<(), EnvironmentCallError> {
    if !supports_set_core_options(callback) {
        let supported = get_core_options_version(callback);

        return Err(EnvironmentCallError::Unsupported(
            format!("set_core_options_intl() requires at least API version 1, but the frontend reports support for version {supported}."),
        ));
    }

    // const struct retro_core_options_intl *
    set(callback, RETRO_ENVIRONMENT_SET_CORE_OPTIONS_INTL, options)
}

/// Allows an implementation to signal the environment
/// which variables it might want to check for later using
/// [`get_variable`].
///
/// This allows the frontend to present these variables to
/// a user dynamically.
///
/// This should only be called if [`get_core_options_version`]
/// returns an API version of >= 2.
///
/// This should be called instead of [`set_variables`].
///
/// This should be called instead of [`set_core_options`].
///
/// This should be called instead of [`set_core_options_intl`].
///
/// This should be called instead of [`set_core_options_v2`].
///
/// This should be called the first time as early as
/// possible (ideally in [`Core::on_set_environment`]).
///
/// Afterwards it may be called again for the core to communicate
/// updated options to the frontend, but the number of core
/// options must not change from the number in the initial call.
///
/// If [`get_core_options_version`] returns an API
/// version of `>= 2`, this callback is guaranteed to succeed
/// (i.e. callback return value does not indicate success)
///
/// If callback returns [`true`], frontend has core option category
/// support.
///
/// If callback returns [`false`], frontend does not have core option
/// category support.
///
/// This is fundamentally the same as [`set_core_options_v2`],
/// with the addition of localisation support. The description of the
/// [`set_core_options_v2`] callback should be consulted
/// for further details.
///
/// 'data' points to a [`retro_core_options_v2_intl`] struct.
///
/// - [`retro_core_options_v2_intl::us`] is a pointer to a
///   [`retro_core_options_v2`] struct defining the US English
///   core options implementation. It must point to a valid struct.
///
/// - [`retro_core_options_v2_intl::local`] is a pointer to a
///   [`retro_core_options_v2`] struct defining core options for
///   the current frontend language.
///
///   It may be `NULL` (in which case [`retro_core_options_v2_intl::us`] is used by the frontend).
///   Any items missing from this struct will be read from
///   [`retro_core_options_v2_intl::us`] instead.
///
/// NOTE: Default core option values are always taken from the
/// [`retro_core_options_v2_intl::us`] struct. Any default values in
/// the [`retro_core_options_v2_intl::local`] struct will be ignored.
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_core_options_v2_intl(
    callback: retro_environment_t,
    options: retro_core_options_v2_intl,
) -> Result<bool, EnvironmentCallError> {
    if !supports_set_core_options_v2(callback) {
        let supported = get_core_options_version(callback);

        return Err(EnvironmentCallError::Unsupported(
            format!("set_core_options_v2_intl() requires at least API version 2, but the frontend reports support for version {supported}."),
        ));
    }

    // const struct retro_core_options_v2_intl *
    let result = set(
        callback,
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL,
        options,
    );

    match result {
        Ok(()) => Ok(true),
        Err(EnvironmentCallError::Failure) => Ok(false),
        Err(err) => Err(err),
    }
}

/// Allows an implementation to signal the environment to show
/// or hide a variable when displaying core options. This is
/// considered a **suggestion**. The frontend is free to ignore
/// this callback, and its implementation not considered mandatory.
///
/// 'data' points to a [`retro_core_option_display`] struct
///
/// [`retro_core_option_display::key`] is a variable identifier
/// which has already been set by [`set_variables`] / [`set_core_options`].
///
/// [`retro_core_option_display::visible`] is a boolean, specifying
/// whether variable should be displayed
///
/// Note that all core option variables will be set visible by
/// default when calling [`set_variables`] / [`set_core_options`].
#[proc::context(GenericContext)]
pub unsafe fn set_core_options_display(
    callback: retro_environment_t,
    options: retro_core_option_display,
) -> Result<(), EnvironmentCallError> {
    // struct retro_core_option_display *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_DISPLAY,
        options,
    )
}

/// Allows an implementation to ask frontend preferred hardware
/// context to use. Core should use this information to deal
/// with what specific context to request with SET_HW_RENDER.
///
/// 'data' points to an unsigned variable
#[proc::context(GenericContext)]
pub unsafe fn get_preferred_hw_render(
    callback: retro_environment_t,
) -> Result<retro_hw_context_type, EnvironmentCallError> {
    // unsigned *

    let value = get::<retro_hw_context_type_REPR_TYPE>(
        callback,
        RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER,
    )?;

    retro_hw_context_type::try_from(value).map_err(Into::into)
}

/// Unsigned value is the API version number of the disk control
/// interface supported by the frontend. If callback return false,
/// API version is assumed to be 0.
///
/// In legacy code, the disk control interface is defined by passing
/// a struct of type [`retro_disk_control_callback] to
/// [`set_disk_control_interface`].
/// This may be still be done regardless of the disk control
/// interface version.
///
/// If version is >= 1 however, the disk control interface may
/// instead be defined by passing a struct of type
/// [`retro_disk_control_ext_callback`] to
/// [`set_disk_control_ext_interface`].
/// This allows the core to provide additional information about
/// disk images to the frontend and/or enables extra
/// disk control functionality by the frontend.
#[proc::context(GenericContext)]
pub unsafe fn get_disk_control_interface_version(callback: retro_environment_t) -> u32 {
    // unsigned *
    get(
        callback,
        RETRO_ENVIRONMENT_GET_DISK_CONTROL_INTERFACE_VERSION,
    )
    .unwrap_or(0)
}

/// Sets an interface which frontend can use to eject and insert
/// disk images, and also obtain information about individual
/// disk image files registered by the core.
/// This is used for games which consist of multiple images and
/// must be manually swapped out by the user (e.g. PSX, floppy disk
/// based systems).
#[proc::context(GenericContext)]
pub unsafe fn set_disk_control_ext_interface(
    callback: retro_environment_t,
    data: retro_disk_control_ext_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_disk_control_ext_callback *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_DISK_CONTROL_EXT_INTERFACE,
        data,
    )
}

/// The returned value is the API version number of the message
/// interface supported by the frontend.
/// If the underlying callback failed, API version is assumed to be 0.
///
/// In legacy code, messages may be displayed in an
/// implementation-specific manner by passing a struct
/// of type retro_message to [`set_message`].
/// This may be still be done regardless of the message
/// interface version.
///
/// If version is >= 1 however, messages may instead be
/// displayed by calling [`set_message_ext`].
/// This allows the core to specify message logging level, priority and
/// destination (OSD, logging interface or both).
#[proc::context(GenericContext)]
pub unsafe fn get_message_interface_version(callback: retro_environment_t) -> u32 {
    // unsigned *
    get(callback, RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION).unwrap_or(0)
}

/// Sets a message to be displayed in an implementation-specific
/// manner for a certain duration of milliseconds.
/// Additionally allows the core to specify message logging level, priority and
/// destination (OSD, logging interface or both).
/// Should not be used for trivial messages, which should simply be
/// logged via [`RETRO_ENVIRONMENT_GET_LOG_INTERFACE`] (or as a fallback, stderr).
#[allow(clippy::too_many_arguments)]
#[proc::context(GenericContext)]
pub unsafe fn set_message_ext(
    callback: retro_environment_t,
    message: &str,
    duration: u32,
    priority: u32,
    level: retro_log_level,
    target: retro_message_target,
    type_: retro_message_type,
    progress: MessageProgress,
) -> Result<(), EnvironmentCallError> {
    let msg = CString::new(message).map_err(StringError::from)?;

    // const struct retro_message_ext *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_MESSAGE_EXT,
        retro_message_ext {
            msg: msg.as_ptr(),
            duration,
            priority,
            level,
            target,
            type_,
            progress: progress.as_i8(),
        },
    )
}

/// The first returned value is the number of active input devices
/// provided by the frontend. This may change between
/// frames, but will remain constant for the duration
/// of each frame.
///
/// If the second return value is [`true`], a core does not need to
/// poll any input device with an index greater than or equal to
/// the number of active devices.
///
/// If the second return value is [`false`], the number of active input
/// devices is unknown. In this case, all input devices
/// should be considered active.
#[proc::context(GenericContext)]
pub unsafe fn get_input_max_users(callback: retro_environment_t) -> (u32, bool) {
    // unsigned *
    get(callback, RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS).unwrap_or((0, false))
}

/// Lets the core know the occupancy level of the frontend
/// audio buffer. Can be used by a core to attempt frame
/// skipping in order to avoid buffer under-runs.
/// A core may pass `NULL` to disable buffer status reporting
/// in the frontend.
#[proc::context(GenericContext)]
pub unsafe fn set_audio_buffer_status_callback(
    callback: retro_environment_t,
    data: retro_audio_buffer_status_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_audio_buffer_status_callback *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK,
        data,
    )
}

/// Sets minimum frontend audio latency in milliseconds.
/// Resultant audio latency may be larger than set value,
/// or smaller if a hardware limit is encountered. A frontend
/// is expected to honour requests up to 512 ms.
///
/// * If value is less than current frontend
///   audio latency, callback has no effect
/// * If value is zero, default frontend audio
///   latency is set
///
/// May be used by a core to increase audio latency and
/// therefore decrease the probability of buffer under-runs
/// (crackling) when performing 'intensive' operations.
/// A core utilising [`RETRO_ENVIRONMENT_SET_AUDIO_BUFFER_STATUS_CALLBACK`]
/// to implement audio-buffer-based frame skipping may achieve
/// optimal results by setting the audio latency to a 'high'
/// (typically 6x or 8x) integer multiple of the expected
/// frame time.
///
/// Calling this can require a full reinitialization of audio
/// drivers in the frontend, so it is important to call it very
/// sparingly, and usually only with the users explicit consent.
/// An eventual driver reinitialize will happen so that audio
/// callbacks happening after this call within the same [`Core::on_run`]
/// call will target the newly initialized driver.
#[proc::context(RunContext)]
pub unsafe fn set_minimum_audio_latency(
    callback: retro_environment_t,
    latency: u32,
) -> Result<(), EnvironmentCallError> {
    // const unsigned *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY,
        latency,
    )
}

/// Checks whether the frontend supports the [`set_fastforwarding_override`] interface.
#[proc::context(GenericContext)]
pub unsafe fn supports_fastforwarding_override(
    callback: retro_environment_t,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_fastforwarding_override *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE,
        std::ptr::null() as *const c_void,
    )
}

/// Used by a libretro core to override the current
/// fastforwarding mode of the frontend.
#[proc::context(GenericContext)]
pub unsafe fn set_fastforwarding_override(
    callback: retro_environment_t,
    value: retro_fastforwarding_override,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_fastforwarding_override *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_FASTFORWARDING_OVERRIDE,
        value,
    )
}

///  Allows an implementation to override 'global' content
///  info parameters reported by [`Core::get_info`].
///  Overrides also affect subsystem content info parameters
///  set via [`set_subsystem_info`].
///  This function must be called inside [`Core::on_set_environment`].
///  If callback returns [`false`], content info overrides
///  are unsupported by the frontend, and will be ignored.
///  If callback returns [`true`], extended game info may be
///  retrieved by calling [`get_game_info_ext`]
///  in [`Core::on_load_game`] or [`Core::on_load_game_special`].
///
///  'data' points to an array of [`retro_system_content_info_override`]
///  structs terminated by a `{ NULL, false, false }` element.
///  If 'data' is `NULL`, no changes will be made to the frontend;
///  a core may therefore pass `NULL` in order to test whether
///  the [`set_content_info_override`] and
///  [`get_game_info_ext`] callbacks are supported
///  by the frontend.
///
///  For struct member descriptions, see the definition of
///  struct [`retro_system_content_info_override`].
///
///  Example:
///
///  - struct retro_system_info:
///  ```c
///  {
///     "My Core",                      // library_name
///     "v1.0",                         // library_version
///     "m3u|md|cue|iso|chd|sms|gg|sg", // valid_extensions
///     true,                           // need_fullpath
///     false                           // block_extract
///  }
///  ```
///
///  - Array of struct retro_system_content_info_override:
///  ```c
///  {
///     {
///        "md|sms|gg", // extensions
///        false,       // need_fullpath
///        true         // persistent_data
///     },
///     {
///        "sg",        // extensions
///        false,       // need_fullpath
///        false        // persistent_data
///     },
///     { NULL, false, false }
///  }
///  ```
///
///  Result:
///  - Files of type `m3u`, `cue`, `iso`, `chd` will not be
///    loaded by the frontend. Frontend will pass a
///    valid path to the core, and core will handle
///    loading internally
///  - Files of type `md`, `sms`, `gg` will be loaded by
///    the frontend. A valid memory buffer will be
///    passed to the core. This memory buffer will
///    remain valid until [`Core::on_deinit`] returns
///  - Files of type `sg` will be loaded by the frontend.
///    A valid memory buffer will be passed to the core.
///    This memory buffer will remain valid until
///    [`Core::on_load_game`] (or [`Core::on_load_game_special`])
///    returns
///
///  NOTE: If an extension is listed multiple times in
///  an array of [`retro_system_content_info_override`]
///  structs, only the **first** instance will be registered
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_content_info_override(
    callback: retro_environment_t,
    value: retro_system_content_info_override,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_system_content_info_override *
    set(callback, RETRO_ENVIRONMENT_SET_CONTENT_INFO_OVERRIDE, value)
}

/// Allows an implementation to fetch extended game
/// information, providing additional content path
/// and memory buffer status details.
/// This function may only be called inside
/// [`Core::on_load_game`] or [`Core::on_load_game_special`].
///
/// If callback returns `false`, extended game information
/// is unsupported by the frontend. In this case, only
/// regular [`retro_game_info`] will be available.
/// [`get_game_info_ext`] is guaranteed
/// to return true if [`set_content_info_override`]
/// returns [`true`].
///
/// 'data' points to an array of [`retro_game_info_ext structs`].
///
/// For struct member descriptions, see the definition of
/// struct [`retro_game_info_ext`].
///
/// - If function is called inside [`Core::on_load_game`],
///   the [`retro_game_info_ext`] array is guaranteed to
///   have a size of 1 - i.e. the returned pointer may
///   be used to access directly the members of the
///   first [`retro_game_info_ext`] struct, for example:
///
/// ```c
///     struct retro_game_info_ext *game_info_ext;
///     if (environ_cb(RETRO_ENVIRONMENT_GET_GAME_INFO_EXT, &game_info_ext))
///      printf("Content Directory: %s\n", game_info_ext->dir);
///```
///
/// - If the function is called inside [`Core::on_load_game_special`],
///   the [`retro_game_info_ext`] array is guaranteed to have a
///   size equal to the `num_info` argument passed to
///   [`Core::on_load_game_special`]
#[proc::context(LoadGameContext)]
#[proc::context(LoadGameSpecialContext)]
pub unsafe fn get_game_info_ext(
    callback: retro_environment_t,
) -> Result<retro_game_info_ext, EnvironmentCallError> {
    // const struct retro_game_info_ext **
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_GAME_INFO_EXT)
}

/// Allows a frontend to signal that a core must update
/// the visibility of any dynamically hidden core options,
/// and enables the frontend to detect visibility changes.
///
/// Used by the frontend to update the menu display status
/// of core options without requiring a call of [`Core::on_run`].
/// Must be called in [`Core::on_set_environment`].
#[proc::context(SetEnvironmentContext)]
pub unsafe fn set_core_options_update_display_callback(
    callback: retro_environment_t,
    data: retro_core_options_update_display_callback,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_core_options_update_display_callback *
    set(
        callback,
        RETRO_ENVIRONMENT_SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK,
        data,
    )
}

/// Allows the core to test whether [`RETRO_ENVIRONMENT_SET_VARIABLE`]
/// is supported by the frontend.
#[proc::context(GenericContext)]
pub unsafe fn supports_set_variable(callback: retro_environment_t) -> bool {
    // const struct retro_variable *
    set_ptr(
        callback,
        RETRO_ENVIRONMENT_SET_VARIABLE,
        std::ptr::null() as *const c_void,
    )
    .map(|_| true)
    .unwrap_or(false)
}

/// Allows an implementation to notify the frontend
/// that a core option value has changed.
///
/// [`retro_variable::key`] and [`retro_variable::value`]
/// must match strings that have been set previously
/// via one of the following:
///
/// - [`set_variables`]
/// - [`set_core_options`]
/// - [`set_core_options_intl`]
/// - [`set_core_options_v2`]
/// - [`set_core_options_v2_intl`]
///
/// After changing a core option value via this
/// callback, [`get_variable_update`]
/// will return [`true`].
#[proc::context(GenericContext)]
pub unsafe fn set_variable(
    callback: retro_environment_t,
    value: retro_variable,
) -> Result<(), EnvironmentCallError> {
    // const struct retro_variable *
    set(callback, RETRO_ENVIRONMENT_SET_VARIABLE, value)
}

/// Allows an implementation to get details on the actual rate
/// the frontend is attempting to call [`Core::on_run`].
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_throttle_state(
    callback: retro_environment_t,
) -> Result<retro_throttle_state, EnvironmentCallError> {
    // struct retro_throttle_state *
    get_unchecked(callback, RETRO_ENVIRONMENT_GET_THROTTLE_STATE)
}

/// Tells the core about the context the frontend is asking for savestate.
//  See [`retro_savestate_context`]
#[proc::context(GenericContext)]
#[proc::unstable(feature = "env-commands")]
pub unsafe fn get_savestate_context(
    callback: retro_environment_t,
) -> Result<retro_savestate_context, EnvironmentCallError> {
    // int *

    let value = get::<retro_savestate_context_REPR_TYPE>(
        callback,
        RETRO_ENVIRONMENT_GET_SAVESTATE_CONTEXT,
    )?;

    retro_savestate_context::try_from(value).map_err(Into::into)
}
