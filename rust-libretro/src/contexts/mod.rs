//! This module contains abstractions of the libretro environment callbacks.
use crate::{
    core_wrapper::Interfaces,
    error::{
        EnvironmentCallError, LocationServiceError, PerformanceServiceError, StringError, VfsError,
    },
    util::*,
    *,
};
use once_cell::unsync::Lazy;
use std::{path::PathBuf, sync::Arc};

#[macro_use]
mod macros;

/// This would only be used in [`Core::on_run`] from a single thread.
static mut FALLBACK_FRAMEBUFFER: Lazy<Vec<u8>> = Lazy::new(Vec::new);

/// Exposes environment callbacks that are safe to call in every context.
pub struct GenericContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,
    pub(crate) interfaces: Interfaces,
}

impl<'a> GenericContext<'a> {
    pub(crate) fn new(
        environment_callback: &'a retro_environment_t,
        interfaces: Interfaces,
    ) -> Self {
        Self {
            environment_callback,
            interfaces,
        }
    }

    pub unsafe fn environment_callback(&self) -> &'a retro_environment_t {
        self.environment_callback
    }

    pub unsafe fn interfaces(&self) -> Interfaces {
        Arc::clone(&self.interfaces)
    }

    /// Enables the [`Core::on_keyboard_event`] callback.
    pub fn enable_keyboard_callback(&self) -> Result<(), EnvironmentCallError> {
        self.set_keyboard_callback(retro_keyboard_callback {
            callback: Some(retro_keyboard_callback_fn),
        })
    }

    /// Enables the [`Core::on_write_audio`] and [`Core::on_audio_set_state`] callbacks.
    pub fn enable_audio_callback(&self) -> Result<(), EnvironmentCallError> {
        self.set_audio_callback(retro_audio_callback {
            callback: Some(retro_audio_callback_fn),
            set_state: Some(retro_audio_set_state_callback_fn),
        })
    }

    pub fn enable_disk_control_interface(&self) -> Result<(), EnvironmentCallError> {
        self.set_disk_control_interface(retro_disk_control_callback {
            set_eject_state: Some(retro_set_eject_state_callback),
            get_eject_state: Some(retro_get_eject_state_callback),
            get_image_index: Some(retro_get_image_index_callback),
            set_image_index: Some(retro_set_image_index_callback),
            get_num_images: Some(retro_get_num_images_callback),
            replace_image_index: Some(retro_replace_image_index_callback),
            add_image_index: Some(retro_add_image_index_callback),
        })
    }

    pub fn enable_extended_disk_control_interface(&self) -> Result<(), EnvironmentCallError> {
        let version = self.get_disk_control_interface_version();

        if version < 1 {
            let reason = format!("The extended disk control interface is unsupported. The frontend reported support for version “{version}”.");
            return Err(EnvironmentCallError::Unsupported(reason));
        }

        let result = self.set_disk_control_ext_interface(retro_disk_control_ext_callback {
            set_eject_state: Some(retro_set_eject_state_callback),
            get_eject_state: Some(retro_get_eject_state_callback),
            get_image_index: Some(retro_get_image_index_callback),
            set_image_index: Some(retro_set_image_index_callback),
            get_num_images: Some(retro_get_num_images_callback),
            replace_image_index: Some(retro_replace_image_index_callback),
            add_image_index: Some(retro_add_image_index_callback),

            set_initial_image: Some(retro_set_initial_image_callback),
            get_image_path: Some(retro_get_image_path_callback),
            get_image_label: Some(retro_get_image_label_callback),
        });

        if result.is_err() {
            return Err(EnvironmentCallError::FailedToEnable(
                "the extended disk control interface.",
            ));
        }

        Ok(())
    }

    pub fn enable_audio_buffer_status_callback(&self) -> Result<(), EnvironmentCallError> {
        let data = retro_audio_buffer_status_callback {
            callback: Some(retro_audio_buffer_status_callback_fn),
        };

        self.set_audio_buffer_status_callback(Some(data))
    }

    pub fn disable_audio_buffer_status_callback(&self) -> Result<(), EnvironmentCallError> {
        self.set_audio_buffer_status_callback(None)
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn set_led_state(&self, led: i32, state: i32) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let set_led_state = get_led_interface_function!(interfaces, set_led_state);

        unsafe {
            // no return value
            set_led_state(led, state);
        }

        Ok(())
    }

    pub fn set_rumble_state(
        &self,
        port: u32,
        effect: retro_rumble_effect,
        strength: u16,
    ) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let set_rumble_state = get_rumble_interface_function!(interfaces, set_rumble_state);

        let request_was_honored = unsafe { set_rumble_state(port, effect, strength) };

        Ok(request_was_honored)
    }

    pub fn start_perf_counter(&mut self, name: &'static str) -> Result<(), EnvironmentCallError> {
        use std::collections::hash_map::Entry;

        let mut interfaces = self.interfaces.write().unwrap();

        let interface = get_perf_interface!(interfaces);

        let counter = match interfaces.perf_interface.counters.entry(name) {
            Entry::Occupied(counter) => counter.into_mut(),
            Entry::Vacant(entry) => {
                let ident = CString::new(name).map_err(StringError::from)?;
                let ptr = ident.as_ptr();

                entry.insert(PerfCounter {
                    ident,
                    counter: retro_perf_counter {
                        ident: ptr,
                        start: 0,
                        total: 0,
                        call_cnt: 0,
                        registered: false,
                    },
                })
            }
        };

        if !counter.counter.registered {
            let perf_register = interface
                .perf_register
                .ok_or(EnvironmentCallError::NullPointer("perf_register"))?;

            unsafe {
                perf_register(&mut counter.counter as *mut _);
            }
        }

        let perf_start = interface
            .perf_start
            .ok_or(EnvironmentCallError::NullPointer("perf_start"))?;

        unsafe {
            // no return value
            perf_start(&mut counter.counter as *mut _);
        }

        Ok(())
    }

    pub fn stop_perf_counter(&mut self, name: &'static str) -> Result<(), EnvironmentCallError> {
        use std::collections::hash_map::Entry;

        let mut interfaces = self.interfaces.write().unwrap();

        let interface = get_perf_interface!(interfaces);

        match interfaces.perf_interface.counters.entry(name) {
            Entry::Occupied(counter) => {
                let counter = counter.into_mut();

                if counter.counter.registered {
                    let perf_stop = interface
                        .perf_stop
                        .ok_or(EnvironmentCallError::NullPointer("perf_stop"))?;

                    unsafe {
                        // no return value
                        perf_stop(&mut counter.counter as *mut _);
                    }

                    return Ok(());
                }

                Err(PerformanceServiceError::UnregisteredPerformanceCounter(name).into())
            }
            _ => Err(PerformanceServiceError::UnknownPerformanceCounter(name).into()),
        }
    }

    pub fn perf_log(&self) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let perf_log = get_perf_interface_function!(interfaces, perf_log);

        unsafe {
            // no return value
            perf_log();
        }

        Ok(())
    }

    pub fn perf_get_time_usec(&self) -> Result<i64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_time_usec = get_perf_interface_function!(interfaces, get_time_usec);

        Ok(unsafe { get_time_usec() })
    }

    pub fn perf_get_counter(&self) -> Result<u64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_perf_counter = get_perf_interface_function!(interfaces, get_perf_counter);

        Ok(unsafe { get_perf_counter() })
    }

    pub fn get_cpu_features(&self) -> Result<CpuFeatures, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_cpu_features = get_perf_interface_function!(interfaces, get_cpu_features);

        let bits = unsafe { get_cpu_features() };

        validate_bitflags!(CpuFeatures, u64, bits)
    }

    pub fn location_service_start(&self) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let start = get_location_interface_function!(interfaces, start);

        let started = unsafe { start() };

        if !started {
            return Err(LocationServiceError::FailedToStart.into());
        }

        Ok(())
    }

    pub fn location_service_stop(&self) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let stop = get_location_interface_function!(interfaces, stop);

        unsafe {
            // no return value
            stop();
        }

        Ok(())
    }

    pub fn location_service_get_position(&self) -> Result<Position, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_position = get_location_interface_function!(interfaces, get_position);

        let mut lat = 0f64;
        let mut lon = 0f64;
        let mut horiz_accuracy = 0f64;
        let mut vert_accuracy = 0f64;

        let success = unsafe {
            get_position(
                &mut lat as *mut f64,
                &mut lon as *mut f64,
                &mut horiz_accuracy as *mut f64,
                &mut vert_accuracy as *mut f64,
            )
        };

        if !success {
            return Err(LocationServiceError::FailedToGetPosition.into());
        }

        Ok(Position {
            lat,
            lon,
            horiz_accuracy,
            vert_accuracy,
        })
    }

    pub fn location_service_set_interval(
        &self,
        interval_ms: u32,
        interval_distance: u32,
    ) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let set_interval = get_location_interface_function!(interfaces, set_interval);

        unsafe {
            // no return value
            set_interval(interval_ms, interval_distance);
        }

        Ok(())
    }

    pub fn midi_input_enabled(&self) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let input_enabled = get_midi_interface_function!(interfaces, input_enabled);

        Ok(unsafe { input_enabled() })
    }

    pub fn midi_output_enabled(&self) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let output_enabled = get_midi_interface_function!(interfaces, output_enabled);

        Ok(unsafe { output_enabled() })
    }

    pub fn midi_read_next(&self) -> Result<u8, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let read = get_midi_interface_function!(interfaces, read);

        let mut value = 0;
        let success = unsafe { read(&mut value as *mut u8) };

        if success {
            return Ok(value);
        }

        Err(EnvironmentCallError::Failure)
    }

    pub fn midi_write_byte(&self, value: u8, delta_time: u32) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let write = get_midi_interface_function!(interfaces, write);

        let success = unsafe { write(value, delta_time) };

        if success {
            return Ok(());
        }

        Err(EnvironmentCallError::Failure)
    }

    pub fn midi_flush(&self) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let flush = get_midi_interface_function!(interfaces, flush);

        let flushed = unsafe { flush() };

        if flushed {
            return Ok(());
        }

        Err(EnvironmentCallError::Failure)
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_get_path(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<PathBuf, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_path = get_vfs_function!(interfaces, get_path);

        let path = unsafe { get_path(handle) };

        util::get_path_buf_from_pointer(path).map_err(Into::into)
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_open(
        &self,
        path: &str,
        mode: VfsFileOpenFlags,
        hints: VfsFileOpenHints,
    ) -> Result<retro_vfs_file_handle, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let open = get_vfs_function!(interfaces, open);

        let c_path = CString::new(path).map_err(StringError::from)?;

        let handle = unsafe { open(c_path.as_ptr(), mode.bits(), hints.bits()) };

        if !handle.is_null() {
            let handle = unsafe { *handle };
            return Ok(handle);
        }

        Err(VfsError::FailedToOpen(path.to_owned()).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_close(&self, mut handle: retro_vfs_file_handle) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let close = get_vfs_function!(interfaces, close);

        let status = unsafe { close(&mut handle) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToClose.into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_size(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<u64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let size = get_vfs_function!(interfaces, size);

        let file_size = unsafe { size(handle) };

        if file_size >= 0 {
            return Ok(file_size as u64);
        }

        Err(VfsError::FailedToGetFileSize.into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_truncate(
        &self,
        handle: &mut retro_vfs_file_handle,
        length: i64, // no idea why the API wants signed values
    ) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let truncate = get_vfs_function!(interfaces, truncate, 2);

        let status = unsafe { truncate(handle, length) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToTruncate(length).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_tell(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<u64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let tell = get_vfs_function!(interfaces, tell);

        let position = unsafe { tell(handle) };

        if position >= 0 {
            return Ok(position as u64);
        }

        Err(VfsError::FailedToTell.into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_seek(
        &self,
        handle: &mut retro_vfs_file_handle,
        offset: i64,
        seek_position: VfsSeekPosition,
    ) -> Result<u64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let seek = get_vfs_function!(interfaces, seek);

        let position = unsafe { seek(handle, offset, seek_position as i32) };

        if position >= 0 {
            return Ok(position as u64);
        }

        Err(VfsError::FailedToSeek(seek_position, offset).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_read(
        &self,
        handle: &mut retro_vfs_file_handle,
        length: usize,
    ) -> Result<Vec<u8>, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let read = get_vfs_function!(interfaces, read);

        let mut buffer = Vec::with_capacity(length);

        let read_length = unsafe { read(handle, buffer.as_mut_ptr() as *mut _, length as u64) };

        if read_length >= 0 {
            buffer.truncate(read_length as usize);

            return Ok(buffer);
        }

        Err(VfsError::FailedToRead(length).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_write(
        &self,
        handle: &mut retro_vfs_file_handle,
        buffer: &mut [u8],
    ) -> Result<u64, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let write = get_vfs_function!(interfaces, write);

        let bytes_written =
            unsafe { write(handle, buffer.as_mut_ptr() as *mut _, buffer.len() as u64) };

        if bytes_written >= 0 {
            return Ok(bytes_written as u64);
        }

        Err(VfsError::FailedToWrite(buffer.len()).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_flush(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let flush = get_vfs_function!(interfaces, flush);

        let status = unsafe { flush(handle) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToFlush.into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_remove(&self, path: &str) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let remove = get_vfs_function!(interfaces, remove);

        let c_path = CString::new(path).map_err(StringError::from)?;

        let status = unsafe { remove(c_path.as_ptr()) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToRemove(path.to_owned()).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_rename(&self, old_path: &str, new_path: &str) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let rename = get_vfs_function!(interfaces, rename);

        let old_c_path = CString::new(old_path).map_err(StringError::from)?;
        let new_c_path = CString::new(new_path).map_err(StringError::from)?;

        let status = unsafe { rename(old_c_path.as_ptr(), new_c_path.as_ptr()) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToRename(old_path.to_owned(), new_path.to_owned()).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_stat(&self, path: &str) -> Result<(VfsStat, u32), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let stat = get_vfs_function!(interfaces, stat, 3);

        let c_path = CString::new(path).map_err(StringError::from)?;

        let mut size = 0u32;

        // VFS’s stat function is based on POSIX `stat.h` which uses a signed offset
        // type but file sizes are unsigned
        let value = unsafe { stat(c_path.as_ptr(), &mut size as *mut _ as *mut i32) };

        let stat = validate_bitflags!(VfsStat, i32, value)?;

        if stat.bits() == 0 {
            return Err(VfsError::StatInvalidPath(path.to_owned()).into());
        }

        Ok((stat, size))
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_mkdir(&self, dir: &str) -> Result<VfsMkdirStatus, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let mkdir = get_vfs_function!(interfaces, mkdir, 3);

        let c_dir = CString::new(dir).map_err(StringError::from)?;

        let result = unsafe { mkdir(c_dir.as_ptr()) };

        match result {
            0 => Ok(VfsMkdirStatus::Success),
            -2 => Ok(VfsMkdirStatus::Exists),

            -1 => Err(VfsError::FailedToCreateDirectory(dir.to_owned()).into()),
            n => Err(VfsError::UnexpectedValue(n.to_string()).into()),
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_opendir(
        &self,
        dir: &str,
        include_hidden: bool,
    ) -> Result<retro_vfs_dir_handle, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let opendir = get_vfs_function!(interfaces, opendir, 3);

        let c_dir = CString::new(dir).map_err(StringError::from)?;

        let handle = unsafe { opendir(c_dir.as_ptr(), include_hidden) };

        if !handle.is_null() {
            let handle = unsafe { *handle };
            return Ok(handle);
        }

        Err(VfsError::FailedToOpen(dir.to_owned()).into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_readdir(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<VfsReadDirStatus, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let readdir = get_vfs_function!(interfaces, readdir, 3);

        let status = unsafe { readdir(handle) };

        if status {
            Ok(VfsReadDirStatus::Success)
        } else {
            Ok(VfsReadDirStatus::AlreadyOnLastEntry)
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_dirent_get_name(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<CString, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let dirent_get_name = get_vfs_function!(interfaces, dirent_get_name, 3);

        let ptr = unsafe { dirent_get_name(handle) };

        get_cstring_from_pointer(ptr).map_err(Into::into)
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_dirent_is_dir(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let dirent_is_dir = get_vfs_function!(interfaces, dirent_is_dir, 3);

        Ok(unsafe { dirent_is_dir(handle) })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_closedir(
        &self,
        mut handle: retro_vfs_dir_handle,
    ) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let closedir = get_vfs_function!(interfaces, closedir, 3);

        let status = unsafe { closedir(&mut handle) };

        if status == 0 {
            return Ok(());
        }

        Err(VfsError::FailedToClose.into())
    }

    /// Once [`Core::on_hw_context_reset()`] has been called this function
    /// can be used to ask the frontend for the addresses of functions like `glClear`.
    pub fn hw_render_get_proc_address(
        &self,
        symbol: &str,
    ) -> Result<unsafe extern "C" fn(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_proc_address = get_hw_render_callback_function!(interfaces, get_proc_address);

        let c_symbol = CString::new(symbol).map_err(StringError::from)?;
        let ptr = unsafe { get_proc_address(c_symbol.as_ptr()) };

        // Check if we got a null pointer, if not return the function pointer
        if let Some(ptr) = ptr {
            Ok(ptr)
        } else {
            Err(EnvironmentCallError::NullPointer2(format!(
                "get_proc_address(\"{symbol}\")"
            )))
        }
    }

    /// In [`Core::on_run()`], use [`GenericContext::hw_render_get_framebuffer()`] to get which FBO to render to,
    /// e.g. `glBindFramebuffer(GL_FRAMEBUFFER, ctc.hw_render_get_framebuffer())`.
    /// This is your "backbuffer". Do not attempt to render to the real backbuffer.
    /// You must call this every frame as it can change every frame.
    /// The dimensions of this FBO are at least as big as declared in `max_width` and `max_height`.
    /// If desired, the FBO also has a depth buffer attached (see [`RETRO_ENVIRONMENT_SET_HW_RENDER`]).
    ///
    /// Note taken from <https://docs.libretro.com/development/cores/opengl-cores/>
    pub fn hw_render_get_framebuffer(&self) -> Result<usize, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_framebuffer = get_hw_render_callback_function!(interfaces, get_current_framebuffer);

        let fbo_id = unsafe { get_framebuffer() };

        Ok(fbo_id)
    }
}

/// Functions that are safe to be called in [`Core::on_reset`].
pub type ResetContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_deinit`].
pub type DeinitContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::get_serialize_size`].
pub type GetSerializeSizeContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_serialize`].
pub type SerializeContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_unserialize`].
pub type UnserializeContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_unload_game`].
pub type UnloadGameContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_cheat_reset`].
pub type CheatResetContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_cheat_set`].
pub type CheatSetContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::on_get_region`].
pub type GetRegionContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::get_memory_data`].
pub type GetMemoryDataContext<'a> = GenericContext<'a>;

/// Functions that are safe to be called in [`Core::get_memory_size`].
pub type GetMemorySizeContext<'a> = GenericContext<'a>;

make_context!(GetAvInfoContext, #[doc = "Functions that are safe to be called in [`Core::on_get_av_info`]"]);
make_context!(InitContext, #[doc = "Functions that are safe to be called in [`Core::on_init`]"]);
make_context!(OptionsChangedContext, #[doc = "Functions that are safe to be called in [`Core::on_options_changed`]"]);

make_context!(LoadGameSpecialContext, #[doc = "Functions that are safe to be called in [`Core::on_load_game_special`]"]);
into_generic!(LoadGameSpecialContext<'a>, LoadGameContext, 'a);

make_context!(SetEnvironmentContext, #[doc = "Functions that are safe to be called in [`Core::on_set_environment`]"]);

impl<'a> SetEnvironmentContext<'a> {
    pub fn enable_proc_address_interface(&mut self) -> Result<(), EnvironmentCallError> {
        self.set_proc_address_callback(retro_get_proc_address_interface {
            get_proc_address: Some(retro_get_proc_address_callback),
        })
    }

    pub fn enable_options_update_display_callback(&mut self) -> Result<(), EnvironmentCallError> {
        self.set_core_options_update_display_callback(retro_core_options_update_display_callback {
            callback: Some(retro_core_options_update_display_callback_fn),
        })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_vfs_interface(&mut self, min_version: u32) -> Result<u32, EnvironmentCallError> {
        let mut interfaces = self.interfaces.write().unwrap();

        let info = unsafe {
            self.get_vfs_interface(retro_vfs_interface_info {
                required_interface_version: min_version,
                iface: std::ptr::null_mut(),
            })
        };

        if let Ok(info) = info {
            if !info.iface.is_null() && info.required_interface_version >= min_version {
                let iface = unsafe { *info.iface };

                interfaces.vfs_interface_info = VfsInterfaceInfo {
                    supported_version: info.required_interface_version,
                    interface: Some(iface),
                }
            }
        }

        if interfaces.vfs_interface_info.interface.is_some() {
            Ok(interfaces.vfs_interface_info.supported_version)
        } else {
            Err(EnvironmentCallError::NullPointer(
                "vfs_interface_info.interface",
            ))
        }
    }
}

/// Functions that are safe to be called in [`Core::on_load_game`].
///
/// For a description of the callbacks see [`CoreWrapper`].
pub struct LoadGameContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,
    pub(crate) interfaces: Interfaces,
}

impl<'a> LoadGameContext<'a> {
    pub(crate) fn new(
        environment_callback: &'a retro_environment_t,
        interfaces: Interfaces,
    ) -> Self {
        Self {
            environment_callback,
            interfaces,
        }
    }

    /// The reference represents the time of one frame.
    /// It is computed as `1000000 / fps`, but the implementation will resolve the
    /// rounding to ensure that framestepping, etc is exact.
    pub fn enable_frame_time_callback(&self, reference: i64) -> Result<(), EnvironmentCallError> {
        self.set_frame_time_callback(retro_frame_time_callback {
            callback: Some(retro_frame_time_callback_fn),
            reference,
        })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_camera_interface(
        &mut self,
        caps: u64,
        width: u32,
        height: u32,
    ) -> Result<(), EnvironmentCallError> {
        use retro_camera_buffer::*;

        let enable_raw = caps & (1 << RETRO_CAMERA_BUFFER_RAW_FRAMEBUFFER as u64) > 0;
        let enable_opengl = caps & (1 << RETRO_CAMERA_BUFFER_OPENGL_TEXTURE as u64) > 0;

        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.camera_interface.take();

        let callback = retro_camera_callback {
            caps,
            width,
            height,

            start: None,
            stop: None,

            frame_raw_framebuffer: if enable_raw {
                Some(retro_camera_frame_raw_framebuffer_callback)
            } else {
                None
            },
            frame_opengl_texture: if enable_opengl {
                Some(retro_camera_frame_opengl_texture_callback)
            } else {
                None
            },
            initialized: Some(retro_camera_initialized_callback),
            deinitialized: Some(retro_camera_deinitialized_callback),
        };

        let iface = unsafe { self.get_camera_interface(callback)? };
        interfaces.camera_interface.replace(iface);

        Ok(())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_sensor_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.sensor_interface.take();
        let iface = unsafe { ctx.get_sensor_interface()? };
        interfaces.sensor_interface.replace(iface);

        Ok(())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_led_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.led_interface.take();
        let iface = unsafe { ctx.get_led_interface()? };
        interfaces.led_interface.replace(iface);

        Ok(())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_midi_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.midi_interface.take();
        let iface = unsafe { ctx.get_midi_interface()? };
        interfaces.midi_interface.replace(iface);

        Ok(())
    }

    pub fn enable_location_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.location_interface.take();
        let mut iface = ctx.get_location_callback()?;

        iface.initialized = Some(retro_location_lifetime_status_initialized_callback);
        iface.deinitialized = Some(retro_location_lifetime_status_deinitialized_callback);

        interfaces.location_interface.replace(iface);

        Ok(())
    }

    pub fn enable_rumble_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.rumble_interface.take();
        let iface = self.get_rumble_interface()?;
        interfaces.rumble_interface.replace(iface);

        Ok(())
    }

    pub fn enable_perf_interface(&mut self) -> Result<(), EnvironmentCallError> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.perf_interface.interface.take();
        interfaces.perf_interface.counters.clear();

        let iface = ctx.get_perf_interface()?;
        interfaces.perf_interface.interface.replace(iface);

        Ok(())
    }

    pub unsafe fn enable_hw_render(
        &mut self,
        context_type: retro_hw_context_type,
        bottom_left_origin: bool,
        version_major: u32,
        version_minor: u32,
        debug_context: bool,
    ) -> Result<(), EnvironmentCallError> {
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.hw_render_callback.take();

        let data = retro_hw_render_callback {
            context_type,
            bottom_left_origin,
            version_major,
            version_minor,
            debug_context,

            cache_context: true, // “probably obsolete”
            depth: false,        // obsolete
            stencil: false,      // obsolete

            context_reset: Some(retro_hw_context_reset_callback),
            context_destroy: Some(retro_hw_context_destroyed_callback),

            // Set by the frontend
            get_current_framebuffer: None,
            get_proc_address: None,
        };

        let iface = self.set_hw_render(data)?;
        interfaces.hw_render_callback.replace(iface);

        Ok(())
    }

    #[proc::unstable(feature = "env-commands")]
    pub unsafe fn set_hw_render_context_negotiation_interface_data<
        T: HwRenderContextNegotiationInterface + 'static,
    >(
        &mut self,
        interface: T,
    ) -> Result<(), EnvironmentCallError> {
        assert!(
            std::mem::size_of::<T>()
                >= std::mem::size_of::<retro_hw_render_context_negotiation_interface>()
        );

        let mut interfaces = self.interfaces.write().unwrap();

        let data = Box::new(interface);

        interfaces
            .hw_render_context_negotiation_interface
            .replace(data);

        let interface = interfaces
            .hw_render_context_negotiation_interface
            .as_ref()
            .unwrap()
            .as_any()
            .downcast_ref::<T>()
            .unwrap();

        let interface =
            interface as *const _ as *const retro_hw_render_context_negotiation_interface;

        self.set_hw_render_context_negotiation_interface(&*interface)
    }

    #[proc::unstable(feature = "env-commands")]
    pub unsafe fn enable_hw_render_negotiation_interface(
        &mut self,
        interface_type: retro_hw_render_context_negotiation_interface_type,
        interface_version: u32,
    ) -> Result<(), EnvironmentCallError> {
        self.set_hw_render_context_negotiation_interface_data(
            retro_hw_render_context_negotiation_interface {
                interface_type,
                interface_version,
            },
        )
    }

    #[cfg(feature = "vulkan")]
    #[proc::unstable(feature = "env-commands")]
    pub unsafe fn enable_hw_render_negotiation_interface_vulkan(
        &mut self,
        get_application_info: retro_vulkan_get_application_info_t,
        create_device: retro_vulkan_create_device_t,
        destroy_device: retro_vulkan_destroy_device_t,
    ) -> Result<(), EnvironmentCallError> {
        self.set_hw_render_context_negotiation_interface_data(retro_hw_render_context_negotiation_interface_vulkan {
            interface_type: retro_hw_render_context_negotiation_interface_type::RETRO_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE_VULKAN,
            interface_version: RETRO_HW_RENDER_CONTEXT_NEGOTIATION_INTERFACE_VULKAN_VERSION,
            get_application_info,
            create_device,
            destroy_device,
        })
    }
}
into_generic!(LoadGameContext<'a>, 'a);

/// Functions that are safe to be called in [`Core::on_write_audio`].
///
/// For a description of the callbacks see [`CoreWrapper`].
pub struct AudioContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,
    pub(crate) interfaces: Interfaces,

    pub(crate) audio_sample_batch_callback: &'a retro_audio_sample_batch_t,
    pub(crate) audio_sample_callback: &'a retro_audio_sample_t,
}

impl AudioContext<'_> {
    /// Renders multiple audio frames in one go if [`AudioContext::audio_sample_batch_callback`] has been set.
    ///
    /// One frame is defined as a sample of left and right channels, interleaved.
    /// I.e. `let buf: [u16; 4] = [ l, r, l, r ];` would be 2 frames.
    ///
    /// Only one of the audio callbacks must ever be used.
    pub fn batch_audio_samples(&self, samples: &[i16]) {
        if let Some(callback) = self.audio_sample_batch_callback {
            let len = samples.len();

            unsafe {
                (callback)(samples.as_ptr(), len / 2);
            }
        }
    }

    /// Renders a single audio frame if [`AudioContext::audio_sample_callback`] has been set.
    /// Should only be used if implementation generates a single sample at a time.
    /// Format is signed 16-bit native endian.
    ///
    /// Only one of the audio callbacks must ever be used.
    pub fn queue_audio_sample(&self, left: i16, right: i16) {
        if let Some(callback) = self.audio_sample_callback {
            unsafe {
                (callback)(left, right);
            }
        }
    }
}

into_generic!(AudioContext<'a>, 'a);

/// Functions that are safe to be called in [`Core::on_run`].
///
/// For a description of the callbacks see [`CoreWrapper`].
pub struct RunContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,
    pub(crate) interfaces: Interfaces,

    pub(crate) audio_sample_batch_callback: &'a retro_audio_sample_batch_t,
    pub(crate) audio_sample_callback: &'a retro_audio_sample_t,
    pub(crate) input_poll_callback: &'a retro_input_poll_t,
    pub(crate) input_state_callback: &'a retro_input_state_t,
    pub(crate) video_refresh_callback: &'a retro_video_refresh_t,

    pub(crate) can_dupe: bool,
    pub(crate) had_frame: &'a mut bool,
    pub(crate) last_width: &'a mut u32,
    pub(crate) last_height: &'a mut u32,
    pub(crate) last_pitch: &'a mut usize,

    pub(crate) supports_bitmasks: bool,
}

into_generic!(RunContext<'a>, 'a);

impl<'a> From<&mut RunContext<'a>> for AudioContext<'a> {
    fn from(other: &mut RunContext<'a>) -> AudioContext<'a> {
        AudioContext {
            environment_callback: other.environment_callback,
            interfaces: Arc::clone(&other.interfaces),

            audio_sample_batch_callback: other.audio_sample_batch_callback,
            audio_sample_callback: other.audio_sample_callback,
        }
    }
}

impl RunContext<'_> {
    #[inline(always)]
    pub fn can_dupe(&self) -> bool {
        self.can_dupe
    }

    /// Polls for input if [`RunContext::input_poll_callback`] has been set
    pub fn poll_input(&self) {
        if let Some(callback) = self.input_poll_callback {
            unsafe {
                (callback)();
            }
        }
    }

    /// Gets the input state for the given player and device if [`RunContext::input_state_callback`] has been set
    pub fn get_input_state(&self, port: u32, device: u32, index: u32, id: u32) -> i16 {
        if let Some(callback) = self.input_state_callback {
            unsafe { (callback)(port, device, index, id) }
        } else {
            0
        }
    }

    /// Queries the libretro frontend for the state of each joypad button
    /// by making an environment call for every button separately.
    ///
    /// See also [`Self::get_joypad_bitmask`].
    pub fn get_joypad_state(&self, port: u32, index: u32) -> JoypadState {
        if let Some(callback) = self.input_state_callback {
            let mut mask = JoypadState::empty();

            unsafe {
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_B) != 0 {
                    mask |= JoypadState::B
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_Y) != 0 {
                    mask |= JoypadState::Y
                }
                if (callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_SELECT,
                ) != 0
                {
                    mask |= JoypadState::SELECT
                }
                if (callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_START,
                ) != 0
                {
                    mask |= JoypadState::START
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_UP) != 0 {
                    mask |= JoypadState::UP
                }
                if (callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_DOWN,
                ) != 0
                {
                    mask |= JoypadState::DOWN
                }
                if (callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_LEFT,
                ) != 0
                {
                    mask |= JoypadState::LEFT
                }
                if (callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_RIGHT,
                ) != 0
                {
                    mask |= JoypadState::RIGHT
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_A) != 0 {
                    mask |= JoypadState::A
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_X) != 0 {
                    mask |= JoypadState::X
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_L) != 0 {
                    mask |= JoypadState::L
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_R) != 0 {
                    mask |= JoypadState::R
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_L2) != 0 {
                    mask |= JoypadState::L2
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_R2) != 0 {
                    mask |= JoypadState::R2
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_L3) != 0 {
                    mask |= JoypadState::L3
                }
                if (callback)(port, RETRO_DEVICE_JOYPAD, index, RETRO_DEVICE_ID_JOYPAD_R3) != 0 {
                    mask |= JoypadState::R3
                }
            }

            return mask;
        }

        JoypadState::empty()
    }

    /// Queries the frontend for the joypad state with the more efficient, but currently experimental,
    /// joypad bitmask feature. Only a single call into the frontend gets made.
    #[proc::unstable(feature = "env-commands")]
    pub fn get_joypad_bitmask(&self, port: u32, index: u32) -> JoypadState {
        if let Some(callback) = self.input_state_callback {
            if self.supports_bitmasks {
                let bits = unsafe {
                    (callback)(
                        port,
                        RETRO_DEVICE_JOYPAD,
                        index,
                        RETRO_DEVICE_ID_JOYPAD_MASK,
                    ) as u16
                };

                return JoypadState::from_bits_truncate(bits);
            }

            // Fallback
            return self.get_joypad_state(port, index);
        }

        JoypadState::empty()
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn get_current_framebuffer(
        &self,
        width: u32,
        height: u32,
        access_flags: MemoryAccess,
        format: PixelFormat,
    ) -> Result<Framebuffer, EnvironmentCallError> {
        let ctx: GenericContext = self.into();

        let fb = unsafe {
            ctx.get_current_software_framebuffer(retro_framebuffer {
                data: std::ptr::null_mut(),
                width,
                height,
                pitch: 0,
                format: format.into(),
                access_flags: access_flags.bits(),
                memory_flags: 0,
            })?
        };

        if fb.data.is_null() {
            return Err(EnvironmentCallError::NullPointer("framebuffer.data"));
        }

        // TODO: Can we get rid of the raw pointer and PhantomData in an ergonomic way?
        // When defining `data` as `&'a mut [u8]` it has the same lifetime as `self`,
        // which means we borrow `self` for as long as this `FrameBuffer` exists.
        // Thus we cannot pass the `FrameBuffer` to `Self::draw_frame` for example.
        Ok(Framebuffer {
            data: fb.data as *mut u8,
            data_len: fb.height as usize * fb.pitch,
            phantom: ::core::marker::PhantomData,

            width: fb.width,
            height: fb.height,
            pitch: fb.pitch,
            format: fb.format.into(),
            access_flags: unsafe { MemoryAccess::from_bits_unchecked(fb.access_flags) },
            memory_flags: unsafe { MemoryType::from_bits_unchecked(fb.memory_flags) },
        })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn get_current_framebuffer_or_fallback(
        &self,
        width: u32,
        height: u32,
        access_flags: MemoryAccess,
        format: PixelFormat,
    ) -> Framebuffer {
        match self.get_current_framebuffer(width, height, access_flags, format) {
            Ok(fb) if fb.access_flags.intersects(access_flags) => fb,
            _ => {
                let data = unsafe { &mut FALLBACK_FRAMEBUFFER };

                let pitch = width as usize * format.bit_per_pixel();
                let data_len = width as usize * height as usize * pitch;

                if data.len() < data_len {
                    data.resize(data_len, 0);
                }

                Framebuffer {
                    data: data.as_mut_ptr(),
                    data_len,
                    phantom: ::core::marker::PhantomData,

                    width,
                    height,
                    pitch,
                    format,
                    access_flags: MemoryAccess::READ | MemoryAccess::WRITE,
                    memory_flags: MemoryType::UNCACHED,
                }
            }
        }
    }

    /// Draws a new frame if [`RunContext::video_refresh_callback`] has been set
    pub fn draw_frame(&mut self, data: &[u8], width: u32, height: u32, pitch: usize) {
        if let Some(callback) = self.video_refresh_callback {
            *self.had_frame = true;
            *self.last_width = width;
            *self.last_height = height;
            *self.last_pitch = pitch;

            unsafe { (callback)(data.as_ptr() as *const c_void, width, height, pitch) }
        }
    }

    /// Duplicates the previous frame
    pub fn dupe_frame(&self) {
        if !self.can_dupe {
            eprintln!("[ERROR] This frontend does not support frame duping!");
            return;
        } else if !*self.had_frame {
            eprintln!("[ERROR] Cannot dupe frame, no previous frame has been drawn!");
            return;
        }

        if let Some(callback) = self.video_refresh_callback {
            unsafe {
                (callback)(
                    std::ptr::null() as *const c_void,
                    *self.last_width,
                    *self.last_height,
                    *self.last_pitch,
                )
            }
        }
    }

    pub fn draw_framebuffer(&mut self, framebuffer: retro_framebuffer) {
        if let Some(callback) = self.video_refresh_callback {
            *self.had_frame = true;
            *self.last_width = framebuffer.width;
            *self.last_height = framebuffer.height;
            *self.last_pitch = framebuffer.pitch;

            unsafe {
                (callback)(
                    framebuffer.data,
                    framebuffer.width,
                    framebuffer.height,
                    framebuffer.pitch,
                )
            }
        }
    }

    pub fn draw_hardware_frame(&mut self, width: u32, height: u32, pitch: usize) {
        if let Some(callback) = self.video_refresh_callback {
            *self.had_frame = true;
            *self.last_width = width;
            *self.last_height = height;
            *self.last_pitch = pitch;

            unsafe {
                (callback)(
                    RETRO_HW_FRAME_BUFFER_VALID as *const c_void,
                    width,
                    height,
                    pitch,
                )
            }
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn camera_start(&self) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let start = get_camera_interface_function!(interfaces, start);

        Ok(unsafe { start() })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn camera_stop(&self) -> Result<(), EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let stop = get_camera_interface_function!(interfaces, stop);

        unsafe {
            // no return value
            stop();
        }

        Ok(())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn set_sensor_state(
        &self,
        port: u32,
        action: retro_sensor_action,
        rate: u32,
    ) -> Result<bool, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let set_sensor_state = get_sensor_interface_function!(interfaces, set_sensor_state);

        Ok(unsafe { set_sensor_state(port, action, rate) })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn get_sensor_input(&self, port: u32, id: SensorType) -> Result<f32, EnvironmentCallError> {
        let interfaces = self.interfaces.read().unwrap();

        let get_sensor_input = get_sensor_interface_function!(interfaces, get_sensor_input);

        Ok(unsafe { get_sensor_input(port, id as u32) })
    }
}
