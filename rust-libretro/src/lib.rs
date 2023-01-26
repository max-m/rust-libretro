#![allow(clippy::missing_safety_doc)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/favicon.png"
)]

#[cfg(feature = "log")]
mod logger;

mod core_wrapper;
mod macros;

pub mod contexts;
pub mod core;
pub mod environment;
pub mod error;
pub mod types;
pub mod util;

pub use anyhow;
pub use const_str;

pub use macros::*;
pub use rust_libretro_proc as proc;
pub use rust_libretro_sys as sys;

use crate::{contexts::*, core::Core, core_wrapper::CoreWrapper, sys::*, types::*, util::*};
use std::{ffi::*, sync::Arc};

#[doc(hidden)]
static mut RETRO_INSTANCE: Option<CoreWrapper> = None;

/// This macro must be used to initialize your [`Core`].
///
/// # Examples
/// ```rust
/// # use rust_libretro::{contexts::*, core::{Core, CoreOptions}, sys::*, types::*, retro_core};
/// # use std::ffi::CString;
/// struct ExampleCore {
///     option_1: bool,
///     option_2: bool,
///
///     pixels: [u8; 800 * 600 * 4],
///     timer: i64,
///     even: bool,
/// }
/// retro_core!(ExampleCore {
///     option_1: false,
///     option_2: true,
///
///     pixels: [0; 800 * 600 * 4],
///     timer: 5_000_001,
///     even: true,
/// });
///
/// /// Dummy implementation
/// impl CoreOptions for ExampleCore {}
/// impl Core for ExampleCore {
///     fn get_info(&self) -> SystemInfo {
///         SystemInfo {
///             library_name: CString::new("ExampleCore").unwrap(),
///             library_version: CString::new("1.0.0").unwrap(),
///             valid_extensions: CString::new("").unwrap(),
///             need_fullpath: false,
///             block_extract: false,
///         }
///     }
///     fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
///         retro_system_av_info {
///             geometry: retro_game_geometry {
///                 base_width: 800,
///                 base_height: 600,
///                 max_width: 800,
///                 max_height: 600,
///                 aspect_ratio: 0.0,
///             },
///             timing: retro_system_timing {
///                 fps: 60.0,
///                 sample_rate: 0.0,
///             },
///         }
///     }
///     fn on_init(&mut self, ctx: &mut InitContext) { }
/// }
/// ```
#[macro_export]
macro_rules! retro_core {
    ( $( $definition:tt )+ ) => {
        #[doc(hidden)]
        #[inline(never)]
        #[no_mangle]
        pub unsafe extern "Rust" fn __retro_init_core() {
            $crate::set_core($($definition)+);
        }
    }
}

#[doc(hidden)]
macro_rules! forward {
    ($(#[doc = $doc:tt ], )* $wrapper:ident, $name:ident, $handler:ident $(-> $return_type:ty)?, $($context:tt)+) => {
        #[no_mangle]
        $(#[doc = $doc])*
        pub unsafe extern "C" fn $name() $(-> $return_type)? {
            // Check that the instance has been created
            if let Some($wrapper) = RETRO_INSTANCE.as_mut() {
                // Forward to the Core implementation
                let mut ctx = $($context)+;
                return $wrapper.core.$handler(&mut ctx);
            }

            panic!(concat!(stringify!($name), ": Core has not been initialized yet!"));
        }
    };
}

#[doc(hidden)]
macro_rules! callback {
    ($(#[doc = $doc:tt ], )* $name:ident, $arg:ident, $handler:ident) => {
        #[no_mangle]
        $(#[doc = $doc])*
        pub unsafe extern "C" fn $name(arg1: $arg) {
            // Check that the instance has been created
            if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
                if arg1.is_some() {
                    // We were given a callback, make sure that it’s not a NULL pointer
                    if (arg1.unwrap() as *const c_void).is_null() {
                        panic!(concat!(
                            "Expected ",
                            stringify!($arg),
                            " got NULL pointer instead!"
                        ));
                    }
                }

                // The callback is safe to set. Either it’s None or not a NULL pointer
                return wrapper.$handler(arg1);
            }

            panic!(concat!(
                stringify!($name),
                ": Core has not been initialized yet!"
            ));
        }
    };
}

#[doc(hidden)]
macro_rules! log_result {
    (
        $level:ident,
        $expr:expr,
        Ok( $($ok_expr:tt )* ) => { $( $ok:tt )* },
        Err( err ) => { $( $err:tt )* },
        $msg:literal
    ) => {{
        match $expr {
            Ok( $($ok_expr)* ) => {
                $( $ok )*
            },
            #[cfg(feature = "log")]
            Err(err) => {
                log::$level!(concat!($msg, ": {}"), err);

                $( $err )*
            }
            #[cfg(not(feature = "log"))]
            Err(_) => {
                $( $err )*
            }
        }
    }};

    (
        $expr:expr,
        Ok( $($ok_expr:tt )* ) => { $( $ok:tt )* },
        Err( err ) => { $( $err:tt )* },
        $msg:literal
    ) => {{
        log_result!(
            error,
            $expr,
            Ok( $($ok_expr)* ) => { $($ok)* },
            Err( err ) => { $($err)* },
            $msg
        )
    }};

    (
        $level:ident,
        $expr:expr,
        { $( $ok:tt )* },
        { $( $err:tt )* },
        $msg:literal
    ) => {{
        log_result!(
            $level,
            $expr,
            Ok(()) => { $( $ok )* },
            Err(err) => { $( $ok )* },
            $msg
        )
    }};

    (
        $expr:expr,
        { $( $ok:tt )* },
        { $( $err:tt )* },
        $msg:literal
    ) => {{
        log_result!(
            error,
            $expr,
            Ok(()) => { $( $ok )* },
            Err(err) => { $( $ok )* },
            $msg
        )
    }};
}

#[doc(hidden)]
pub fn set_core<C: 'static + Core>(core: C) {
    unsafe {
        if RETRO_INSTANCE.is_some() {
            let core = &RETRO_INSTANCE.as_ref().unwrap().core;
            let info = core.get_info();
            let name = info.library_name.into_string().unwrap();
            let version = info.library_version.into_string().unwrap();

            panic!("Attempted to set a core after the system was already initialized.\nAlready registered core: {name} {version}")
        }

        RETRO_INSTANCE.replace(CoreWrapper::new(core));
    }
}

#[cfg(feature = "log")]
#[doc(hidden)]
fn try_init_log(wrapper: &mut CoreWrapper, fallback: bool) {
    if wrapper.logger_initialized {
        return;
    }

    let log_callback = wrapper
        .environment_callback
        .and_then(|environment_callback| unsafe {
            environment::get_log_callback(Some(environment_callback)).ok()
        });

    let logger = if let Some(log_callback) = log_callback {
        logger::RetroLogger::new(log_callback)
    } else if fallback {
        logger::RetroLogger::new(retro_log_callback { log: None })
    } else {
        return;
    };

    log::set_max_level(log::LevelFilter::Trace);
    log::set_boxed_logger(Box::new(logger)).expect("could not set logger");
    wrapper.logger_initialized = true;

    log::info!("Logger is ready");
}

/*****************************************************************************\
|                              CORE API FUNCTIONS                             |
\*****************************************************************************/

forward!(
    #[doc = "Notifies the [`Core`] when all cheats should be unapplied."],
    wrapper,
    retro_cheat_reset,
    on_cheat_reset,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);
forward!(
    #[doc = "Notifies the [`Core`] when it is being closed and its resources should be freed."],
    wrapper,
    retro_deinit,
    on_deinit,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);
forward!(
    #[doc = "Called when the frontend needs region information from the [`Core`]."],
    #[doc = ""],
    #[doc = "## Note about RetroArch:"],
    #[doc = "RetroArch doesn’t use this interface anymore, because [`retro_get_system_av_info`] provides similar information."],
    wrapper,
    retro_get_region,
    on_get_region -> std::os::raw::c_uint,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);
forward!(
    #[doc = "Notifies the [`Core`] when the current game should be reset."],
    wrapper,
    retro_reset,
    on_reset,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);
forward!(
    #[doc = "Called when the frontend needs to know how large a buffer to allocate for save states."],
    #[doc = ""],
    #[doc = "See also [`rust_libretro_sys::retro_serialize_size`]."],
    wrapper,
    retro_serialize_size,
    get_serialize_size -> usize,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);
forward!(
    #[doc = "Notifies the [`Core`] when the currently loaded game should be unloaded. Called before [`retro_deinit`]."],
    wrapper,
    retro_unload_game,
    on_unload_game,
    GenericContext::new(&wrapper.environment_callback, Arc::clone(&wrapper.interfaces))
);

callback!(
    #[doc = "Provides the audio sample callback to the [`Core`]."],
    #[doc = ""],
    #[doc = "Guaranteed to have been called before the first call to [`retro_run`] is made."],
    retro_set_audio_sample,
    retro_audio_sample_t,
    on_set_audio_sample
);
callback!(
    #[doc = "Provides the batched audio sample callback to the [`Core`]."],
    #[doc = ""],
    #[doc = "Guaranteed to have been called before the first call to [`retro_run`] is made."],
    retro_set_audio_sample_batch,
    retro_audio_sample_batch_t,
    on_set_audio_sample_batch
);
callback!(
    #[doc = "Provides the input polling callback to the [`Core`]."],
    #[doc = ""],
    #[doc = "Guaranteed to have been called before the first call to [`retro_run`] is made."],
    retro_set_input_poll,
    retro_input_poll_t,
    on_set_input_poll
);
callback!(
    #[doc = "Provides the input state request callback to the [`Core`]."],
    #[doc = ""],
    #[doc = "Guaranteed to have been called before the first call to [`retro_run`] is made."],
    retro_set_input_state,
    retro_input_state_t,
    on_set_input_state
);
callback!(
    #[doc = "Provides the frame drawing callback to the [`Core`]."],
    #[doc = ""],
    #[doc = "Guaranteed to have been called before the first call to [`retro_run`] is made."],
    retro_set_video_refresh,
    retro_video_refresh_t,
    on_set_video_refresh
);

/// Tells the frontend which API version this [`Core`] implements.
#[no_mangle]
pub unsafe extern "C" fn retro_api_version() -> std::os::raw::c_uint {
    #[cfg(feature = "log")]
    log::trace!("retro_api_version()");

    RETRO_API_VERSION
}

/// Initializes the [`Core`].
///
/// Called after the environment callbacks have been set.
#[no_mangle]
pub unsafe extern "C" fn retro_init() {
    #[cfg(feature = "log")]
    log::trace!("retro_init()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        // Try really hard to initialize the logging interface here
        #[cfg(feature = "log")]
        try_init_log(wrapper, true);

        wrapper.can_dupe = log_result!(
            warn,
            environment::can_dupe(wrapper.environment_callback),
            Ok(can_dupe) => { can_dupe },
            Err(err) => { false },
            "environment::can_dupe() failed"
        );

        let mut ctx = InitContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.on_init(&mut ctx);
    }

    panic!("retro_init: Core has not been initialized yet!");
}

/// Provides _statically known_ system info to the frontend.
///
/// See also [`rust_libretro_sys::retro_get_system_info`].
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut retro_system_info) {
    #[cfg(feature = "log")]
    log::trace!("retro_get_system_info(info = {info:#?})");

    // Make sure that the pointer we got is plausible
    if info.is_null() {
        panic!("Expected retro_system_info, got NULL pointer instead!");
    }

    // We didn’t get a NULL pointer, so this should be safe
    let info = &mut *info;

    // retro_get_system_info requires statically allocated data
    // This is unsafe because we mutate a static value.
    //
    // TODO: Should this be put behind an Arc<Mutex> or Arc<RwLock>?
    static mut SYS_INFO: Option<*const SystemInfo> = None;

    let sys_info = {
        if SYS_INFO.is_none() {
            extern "Rust" {
                fn __retro_init_core();
            }
            __retro_init_core();

            if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
                SYS_INFO = Some(Box::into_raw(Box::new(wrapper.core.get_info())));
            } else {
                panic!("No core instance found!");
            }
        }

        &*SYS_INFO.unwrap()
    };

    info.library_name = sys_info.library_name.as_ptr();
    info.library_version = sys_info.library_version.as_ptr();
    info.valid_extensions = sys_info.valid_extensions.as_ptr();
    info.need_fullpath = sys_info.need_fullpath;
    info.block_extract = sys_info.block_extract;
}

/// Provides audio/video timings and geometry info to the frontend.
///
/// Guaranteed to be called only after successful invocation of [`retro_load_game`].
///
/// See also [`rust_libretro_sys::retro_get_system_av_info`].
#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut retro_system_av_info) {
    #[cfg(feature = "log")]
    log::trace!("retro_get_system_av_info(info = {info:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        // Make sure that the pointer we got is plausible
        if info.is_null() {
            panic!("Expected retro_system_av_info, got NULL pointer instead!");
        }

        // We didn’t get a NULL pointer, so this should be safe
        let info = &mut *info;

        let mut ctx = GetAvInfoContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        let av_info = wrapper.core.on_get_av_info(&mut ctx);

        info.geometry = av_info.geometry;
        info.timing = av_info.timing;

        return;
    }

    panic!("retro_get_system_av_info: Core has not been initialized yet!");
}

/// Provides the environment callback to the [`Core`].
///
/// Guaranteed to have been called before [`retro_init`].
///
/// **TODO:** This method seems to get called multiple times by RetroArch
#[no_mangle]
pub unsafe extern "C" fn retro_set_environment(environment: retro_environment_t) {
    #[cfg(feature = "log")]
    log::trace!("retro_set_environment(environment = {environment:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        if let Some(callback) = environment {
            #[cfg(feature = "unstable-env-commands")]
            {
                wrapper.supports_bitmasks |= log_result!(
                    warn,
                    environment::get_input_bitmasks(Some(callback)),
                    { true },
                    { false },
                    "environment::get_input_bitmasks() failed"
                );
            }

            // `retro_set_environment()` gets called multiple times by RetroArch,
            // on some calls the environment callback can hand out the logging interface,
            // on some calls it can not. Try on every invocation and take the first valid
            // callback we can get.
            #[cfg(feature = "log")]
            try_init_log(wrapper, false);

            wrapper.environment_callback.replace(callback);
        } else {
            wrapper.environment_callback.take();
        }

        let mut ctx = SetEnvironmentContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        match wrapper.core.set_core_options(&ctx) {
            #[cfg(feature = "log")]
            Ok(true) => {
                log::debug!("Frontend supports option categories");
            }
            #[cfg(feature = "log")]
            Err(err) => {
                log::warn!("Failed to set core options: {}", err);
            }
            _ => (),
        }

        return wrapper.core.on_set_environment(&mut ctx);
    }

    panic!("retro_set_environment: Core has not been initialized yet!");
}

/// Sets the device type to be used for player `port`.
///
/// See also [`rust_libretro_sys::retro_set_controller_port_device`].
#[no_mangle]
pub unsafe extern "C" fn retro_set_controller_port_device(
    port: std::os::raw::c_uint,
    device: std::os::raw::c_uint,
) {
    #[cfg(feature = "log")]
    log::trace!("retro_set_controller_port_device(port = {port}, device = {device})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper
            .core
            .on_set_controller_port_device(port, device, &mut ctx);
    }

    panic!("retro_set_controller_port_device: Core has not been initialized yet!");
}

/// Runs the game for one frame.
///
/// See also [`rust_libretro_sys::retro_run`].
#[no_mangle]
pub unsafe extern "C" fn retro_run() {
    #[cfg(feature = "log")]
    log::trace!("retro_run()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        log_result!(
            warn,
            environment::get_variable_update(wrapper.environment_callback),
            Ok(updated) => {
                if updated {
                    let mut ctx = OptionsChangedContext::new(
                        &wrapper.environment_callback,
                        Arc::clone(&wrapper.interfaces),
                    );

                    wrapper.core.on_options_changed(&mut ctx);
                }
            },
            Err(err) => {
                let mut ctx = OptionsChangedContext::new(
                    &wrapper.environment_callback,
                    Arc::clone(&wrapper.interfaces),
                );

                wrapper.core.on_options_changed(&mut ctx);
            },
            "environment::get_variable_update() failed, telling the core to check its variables"
        );

        if let Some(callback) = wrapper.input_poll_callback {
            (callback)();
        }

        let mut ctx = RunContext {
            environment_callback: &wrapper.environment_callback,
            interfaces: Arc::clone(&wrapper.interfaces),

            video_refresh_callback: &wrapper.video_refresh_callback,
            audio_sample_callback: &wrapper.audio_sample_callback,
            audio_sample_batch_callback: &wrapper.audio_sample_batch_callback,
            input_poll_callback: &wrapper.input_poll_callback,
            input_state_callback: &wrapper.input_state_callback,

            can_dupe: wrapper.can_dupe,
            had_frame: &mut wrapper.had_frame,
            last_width: &mut wrapper.last_width,
            last_height: &mut wrapper.last_height,
            last_pitch: &mut wrapper.last_pitch,

            supports_bitmasks: wrapper.supports_bitmasks,
        };

        return wrapper.core.on_run(&mut ctx, wrapper.frame_delta.take());
    }

    panic!("retro_run: Core has not been initialized yet!");
}

/// Called by the frontend when the [`Core`]s state should be serialized (“save state”).
/// This function should return [`false`] on error.
///
/// This could also be used by a frontend to implement rewind.
#[no_mangle]
pub unsafe extern "C" fn retro_serialize(data: *mut std::os::raw::c_void, size: usize) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_serialize(data = {data:#?}, size = {size})");

    if data.is_null() {
        #[cfg(feature = "log")]
        log::warn!("retro_serialize: data is null");

        return false;
    }

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        // Convert the given buffer into a proper slice
        let slice = std::slice::from_raw_parts_mut(data as *mut u8, size);

        return log_result!(
            wrapper.core.on_serialize(slice, &mut ctx),
            { true },
            { false },
            "failed to serialize"
        );
    }

    panic!("retro_serialize: Core has not been initialized yet!");
}

/// Called by the frontend when a “save state” should be loaded.
/// This function should return [`false`] on error.
///
/// This could also be used by a frontend to implement rewind.
#[no_mangle]
pub unsafe extern "C" fn retro_unserialize(data: *const std::os::raw::c_void, size: usize) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_unserialize(data = {data:#?}, size = {size})");

    if data.is_null() {
        #[cfg(feature = "log")]
        log::warn!("retro_unserialize: data is null");

        return false;
    }

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        // Convert the given buffer into a proper slice
        let slice = std::slice::from_raw_parts_mut(data as *mut u8, size);

        return log_result!(
            wrapper.core.on_unserialize(slice, &mut ctx),
            { true },
            { false },
            "failed to deserialize"
        );
    }

    panic!("retro_unserialize: Core has not been initialized yet!");
}

/// Called by the frontend whenever a cheat should be applied.
///
/// The format is core-specific but this function lacks a return value,
/// so a [`Core`] can’t tell the frontend if it failed to parse a code.
#[no_mangle]
pub unsafe extern "C" fn retro_cheat_set(
    index: std::os::raw::c_uint,
    enabled: bool,
    code: *const std::os::raw::c_char,
) {
    #[cfg(feature = "log")]
    log::trace!("retro_cheat_set(index = {index}, enabled = {enabled}, code = {code:#?})");

    if code.is_null() {
        #[cfg(feature = "log")]
        log::warn!("retro_cheat_set: code is null");

        return;
    }

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        // Wrap the pointer into a `CStr`.
        // This assumes the pointer is valid and ends on a null byte.
        //
        // For now we’ll let the core handle conversion to Rust `str` or `String`,
        // as the lack of documentation doesn’t make it clear if the returned string
        // is encoded as valid UTF-8.
        let code = CStr::from_ptr(code);

        return wrapper.core.on_cheat_set(index, enabled, code, &mut ctx);
    }

    panic!("retro_cheat_set: Core has not been initialized yet!");
}

/// Called by the frontend when a game should be loaded.
///
/// A return value of [`true`] indicates success.
#[no_mangle]
pub unsafe extern "C" fn retro_load_game(game: *const retro_game_info) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_load_game(game_type = {game:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = OptionsChangedContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        wrapper.core.on_options_changed(&mut ctx);

        let mut ctx = LoadGameContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        let status = if game.is_null() {
            wrapper.core.on_load_game(None, &mut ctx)
        } else {
            wrapper.core.on_load_game(Some(*game), &mut ctx)
        };

        cfg_if::cfg_if! {
            if #[cfg(feature = "log")] {
                match status {
                    Ok(()) => return true,
                    Err(err) => {
                        log::error!("Failed to load game: {:?}", err);
                        return false;
                    }
                }
            }
            else {
                return status.is_ok();
            }
        }
    }

    panic!("retro_load_game: Core has not been initialized yet!");
}

/// See [`rust_libretro_sys::retro_load_game_special`].
#[no_mangle]
pub unsafe extern "C" fn retro_load_game_special(
    game_type: std::os::raw::c_uint,
    info: *const retro_game_info,
    num_info: usize,
) -> bool {
    #[cfg(feature = "log")]
    log::trace!(
        "retro_load_game_special(game_type = {game_type}, info = {info:#?}, num_info = {num_info})"
    );

    if info.is_null() {
        #[cfg(feature = "log")]
        log::warn!("retro_load_game_special: info is null");

        return false;
    }

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = OptionsChangedContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        wrapper.core.on_options_changed(&mut ctx);

        let mut ctx = LoadGameSpecialContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        let status = wrapper
            .core
            .on_load_game_special(game_type, info, num_info, &mut ctx);

        cfg_if::cfg_if! {
            if #[cfg(feature = "log")] {
                match status {
                    Ok(()) => return true,
                    Err(err) => {
                        log::error!("Failed to load special game: {:?}", err);
                        return false;
                    }
                }
            }
            else {
                return status.is_ok();
            }
        }
    }

    panic!("retro_load_game_special: Core has not been initialized yet!");
}

/// Returns a mutable pointer to queried memory type.
/// Return [`std::ptr::null()`] in case this doesn’t apply to your [`Core`].
///
/// `id` is one of the `RETRO_MEMORY_*` constants.
#[no_mangle]
pub unsafe extern "C" fn retro_get_memory_data(
    id: std::os::raw::c_uint,
) -> *mut std::os::raw::c_void {
    #[cfg(feature = "log")]
    log::trace!("retro_get_memory_data(id = {id})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.get_memory_data(id, &mut ctx);
    }

    panic!("retro_get_memory_data: Core has not been initialized yet!");
}

/// Returns the size (in bytes) of the queried memory type.
/// Return `0` in case this doesn’t apply to your [`Core`].
///
/// `id` is one of the `RETRO_MEMORY_*` constants.
#[no_mangle]
pub unsafe extern "C" fn retro_get_memory_size(id: std::os::raw::c_uint) -> usize {
    #[cfg(feature = "log")]
    log::trace!("retro_get_memory_size(id = {id})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.get_memory_size(id, &mut ctx);
    }

    panic!("retro_get_memory_size: Core has not been initialized yet!");
}

/*****************************************************************************\
|                            NON CORE API FUNCTIONS                           |
\*****************************************************************************/

/// If enabled by the [`Core`], notifies it when a keyboard button has been pressed or released.
///
/// # Parameters
/// - `down`: `true` if the key has been pressed, `false` if it has been released
/// - `keycode`: `retro_key` value
/// - `character`: The text character of the pressed key, encoded as UTF-32.
/// - `key_modifiers`: `retro_mod` value
#[no_mangle]
pub unsafe extern "C" fn retro_keyboard_callback_fn(
    down: bool,
    keycode: ::std::os::raw::c_uint,
    character: u32,
    key_modifiers: u16,
) {
    #[cfg(feature = "log")]
    log::trace!("retro_keyboard_callback_fn(down = {down}, keycode = {keycode}, character = {character}, key_modifiers = {key_modifiers})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        // Not sure why bindgen uses `c_int32` as value type
        // for the newtype enum on Windows but `c_uint32` on Unix.
        cfg_if::cfg_if! {
            if #[cfg(target_family = "windows")] {
                let keycode = keycode as i32;
            }
        };

        return wrapper.core.on_keyboard_event(
            down,
            retro_key(keycode),
            character,
            retro_mod(key_modifiers.into()),
        );
    }

    panic!("retro_keyboard_callback_fn: Core has not been initialized yet!");
}

/// **TODO:** Documentation.
#[no_mangle]
pub unsafe extern "C" fn retro_hw_context_reset_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_hw_context_reset_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.on_hw_context_reset(&mut ctx);
    }

    panic!("retro_hw_context_reset_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation.
#[no_mangle]
pub unsafe extern "C" fn retro_hw_context_destroyed_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_hw_context_destroyed_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.on_hw_context_destroyed(&mut ctx);
    }

    panic!("retro_hw_context_destroyed_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_set_eject_state_callback(ejected: bool) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_set_eject_state_callback(ejected = {ejected})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_set_eject_state(ejected);
    }

    panic!("retro_set_eject_state_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_eject_state_callback() -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_get_eject_state_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_get_eject_state();
    }

    panic!("retro_get_eject_state_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_image_index_callback() -> ::std::os::raw::c_uint {
    #[cfg(feature = "log")]
    log::trace!("retro_get_image_index_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_get_image_index();
    }

    panic!("retro_get_image_index_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_set_image_index_callback(index: ::std::os::raw::c_uint) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_set_image_index_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_set_image_index(index);
    }

    panic!("retro_set_image_index_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_num_images_callback() -> ::std::os::raw::c_uint {
    #[cfg(feature = "log")]
    log::trace!("retro_get_num_images_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_get_num_images();
    }

    panic!("retro_get_num_images_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_replace_image_index_callback(
    index: ::std::os::raw::c_uint,
    info: *const retro_game_info,
) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_replace_image_index_callback(index = {index}, info = {info:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_replace_image_index(index, info);
    }

    panic!("retro_replace_image_index_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_add_image_index_callback() -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_add_image_index_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_add_image_index();
    }

    panic!("retro_add_image_index_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_set_initial_image_callback(
    index: ::std::os::raw::c_uint,
    path: *const ::std::os::raw::c_char,
) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_set_initial_image_callback(index = {index}, path = {path:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper
            .core
            .on_set_initial_image(index, CStr::from_ptr(path));
    }

    panic!("retro_set_initial_image_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_image_path_callback(
    index: ::std::os::raw::c_uint,
    path: *mut ::std::os::raw::c_char,
    len: usize,
) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_get_image_path_callback(index = {index}, path = {path:#?}, len = {len})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        match wrapper.core.on_get_image_path(index) {
            Some(image_path) => {
                let image_path = image_path.as_bytes();
                let buf = std::slice::from_raw_parts_mut(path as *mut u8, len);
                let len = image_path.len().min(buf.len());

                buf[..len].copy_from_slice(&image_path[..len]);
                return true;
            }
            None => return false,
        }
    }

    panic!("retro_get_image_path_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_image_label_callback(
    index: ::std::os::raw::c_uint,
    label: *mut ::std::os::raw::c_char,
    len: usize,
) -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_get_image_label_callback(index = {index}, label = {label:#?}, len = {len})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        match wrapper.core.on_get_image_label(index) {
            Some(image_label) => {
                let image_label = image_label.as_bytes();
                let buf = std::slice::from_raw_parts_mut(label as *mut u8, len);
                let len = image_label.len().min(buf.len());

                buf[..len].copy_from_slice(&image_label[..len]);
                return true;
            }
            None => return false,
        }
    }

    panic!("retro_get_image_label_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_frame_time_callback_fn(usec: retro_usec_t) {
    #[cfg(feature = "log")]
    log::trace!("retro_frame_time_callback_fn(usec = {usec})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        wrapper.frame_delta = Some(usec);
        return;
    }

    panic!("retro_frame_time_callback_fn: Core has not been initialized yet!");
}

/// Notifies the [`Core`] when audio data should be written.
#[no_mangle]
pub unsafe extern "C" fn retro_audio_callback_fn() {
    // This is just too noisy, even for trace logging
    // #[cfg(feature = "log")]
    // log::trace!("retro_audio_callback_fn()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = AudioContext {
            environment_callback: &wrapper.environment_callback,
            interfaces: Arc::clone(&wrapper.interfaces),

            audio_sample_callback: &wrapper.audio_sample_callback,
            audio_sample_batch_callback: &wrapper.audio_sample_batch_callback,
        };

        return wrapper.core.on_write_audio(&mut ctx);
    }

    panic!("retro_audio_callback_fn: Core has not been initialized yet!");
}

/// Notifies the [`Core`] about the state of the frontend’s audio system.
///
/// [`true`]: Audio driver in frontend is active, and callback is
/// expected to be called regularily.
///
/// [`false`]: Audio driver in frontend is paused or inactive.
///
/// Audio callback will not be called until set_state has been
/// called with [`true`].
///
/// Initial state is [`false`] (inactive).
#[no_mangle]
pub unsafe extern "C" fn retro_audio_set_state_callback_fn(enabled: bool) {
    #[cfg(feature = "log")]
    log::trace!("retro_audio_set_state_callback_fn(enabled = {enabled})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_audio_set_state(enabled);
    }

    panic!("retro_audio_set_state_callback_fn: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_camera_frame_raw_framebuffer_callback(
    buffer: *const u32,
    width: ::std::os::raw::c_uint,
    height: ::std::os::raw::c_uint,
    pitch: usize,
) {
    let buffer_size = height as usize * pitch;
    let buffer = std::slice::from_raw_parts(buffer, buffer_size);

    #[cfg(feature = "log")]
    log::trace!("retro_camera_frame_raw_framebuffer_callback(buffer = &[u32; {}], width = {width}, height = {height}, pitch = {pitch})", buffer.len());

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper
            .core
            .on_camera_raw_framebuffer(buffer, width, height, pitch);
    }

    panic!("retro_camera_frame_raw_framebuffer_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_camera_frame_opengl_texture_callback(
    texture_id: ::std::os::raw::c_uint,
    texture_target: ::std::os::raw::c_uint,
    affine: *const f32,
) {
    #[cfg(feature = "log")]
    log::trace!("retro_camera_frame_opengl_texture_callback(texture_id = {texture_id}, texture_target = {texture_target}, affine = {:#?})", std::slice::from_raw_parts(affine, 3 * 3));

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        // Packed 3x3 column-major matrix
        let matrix = std::slice::from_raw_parts(affine, 3 * 3);
        // Convert to fixed size array; we know it contains 9 elements
        let matrix: &[f32; 3 * 3] = matrix.try_into().unwrap();

        return wrapper
            .core
            .on_camera_gl_texture(texture_id, texture_target, matrix);
    }

    panic!("retro_camera_frame_opengl_texture_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_camera_initialized_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_camera_initialized_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.on_camera_initialized(&mut ctx);
    }

    panic!("retro_camera_initialized_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_camera_deinitialized_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_camera_deinitialized_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper.core.on_camera_deinitialized(&mut ctx);
    }

    panic!("retro_camera_deinitialized_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_location_lifetime_status_initialized_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_location_lifetime_status_initialized_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper
            .core
            .on_location_lifetime_status_initialized(&mut ctx);
    }

    panic!(
        "retro_location_lifetime_status_initialized_callback: Core has not been initialized yet!"
    );
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_location_lifetime_status_deinitialized_callback() {
    #[cfg(feature = "log")]
    log::trace!("retro_location_lifetime_status_deinitialized_callback()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        let mut ctx = GenericContext::new(
            &wrapper.environment_callback,
            Arc::clone(&wrapper.interfaces),
        );

        return wrapper
            .core
            .on_location_lifetime_status_deinitialized(&mut ctx);
    }

    panic!(
        "retro_location_lifetime_status_deinitialized_callback: Core has not been initialized yet!"
    );
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_get_proc_address_callback(
    sym: *const ::std::os::raw::c_char,
) -> retro_proc_address_t {
    #[cfg(feature = "log")]
    log::trace!("retro_get_proc_address_callback({sym:#?})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_get_proc_address(CStr::from_ptr(sym));
    }

    panic!("retro_get_proc_address_callback: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_audio_buffer_status_callback_fn(
    active: bool,
    occupancy: ::std::os::raw::c_uint,
    underrun_likely: bool,
) {
    #[cfg(feature = "log")]
    log::trace!("retro_audio_buffer_status_callback_fn(active = {active}, occupancy = {occupancy}, underrun_likely = {underrun_likely})");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper
            .core
            .on_audio_buffer_status(active, occupancy, underrun_likely);
    }

    panic!("retro_audio_buffer_status_callback_fn: Core has not been initialized yet!");
}

/// **TODO:** Documentation
#[no_mangle]
pub unsafe extern "C" fn retro_core_options_update_display_callback_fn() -> bool {
    #[cfg(feature = "log")]
    log::trace!("retro_core_options_update_display_callback_fn()");

    if let Some(wrapper) = RETRO_INSTANCE.as_mut() {
        return wrapper.core.on_core_options_update_display();
    }

    panic!("retro_core_options_update_display_callback_fn: Core has not been initialized yet!");
}
