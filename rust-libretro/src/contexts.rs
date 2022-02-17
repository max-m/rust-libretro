//! This module contains abstractions of the libretro environment callbacks.
use crate::core_wrapper::Interfaces;
use std::collections::HashMap;

use super::*;

#[doc(hidden)]
macro_rules! into_generic {
    ($type:ty, $lifetime:tt) => {
        into_generic!($type, GenericContext, $lifetime);
    };
    ($type:ty, $other:ident, $lifetime:tt) => {
        impl<$lifetime> From<&$type> for $other<$lifetime> {
            fn from(other: &$type) -> $other<$lifetime> {
                $other::new(other.environment_callback, Arc::clone(&other.interfaces))
            }
        }

        impl<$lifetime> From<&mut $type> for $other<$lifetime> {
            fn from(other: &mut $type) -> $other<$lifetime> {
                $other::new(other.environment_callback, Arc::clone(&other.interfaces))
            }
        }
    };
}

#[doc(hidden)]
macro_rules! make_context {
    ($name:ident $(, #[doc = $doc:tt ])?) => {
        $(#[doc = $doc])?
        pub struct $name<'a> {
            pub(crate) environment_callback: &'a retro_environment_t,
            pub(crate) interfaces: Interfaces,
        }

        impl<'a> $name<'a> {
            pub(crate) fn new(environment_callback: &'a retro_environment_t, interfaces: Interfaces) -> Self {
                Self {
                    environment_callback,
                    interfaces
                }
            }
        }

        into_generic!($name<'a>, 'a);
    };
}

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
    pub fn enable_keyboard_callback(&self) -> bool {
        self.set_keyboard_callback(retro_keyboard_callback {
            callback: Some(retro_keyboard_callback_fn),
        })
    }

    /// Enables the [`Core::on_write_audio`] and [`Core::on_audio_set_state`] callbacks.
    pub fn enable_audio_callback(&self) -> bool {
        self.set_audio_callback(retro_audio_callback {
            callback: Some(retro_audio_callback_fn),
            set_state: Some(retro_audio_set_state_callback_fn),
        })
    }

    pub fn enable_disk_control_interface(&self) -> bool {
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

    pub fn enable_extended_disk_control_interface(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.get_disk_control_interface_version() >= 1 {
            let success = self.set_disk_control_ext_interface(retro_disk_control_ext_callback {
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

            if !success {
                return Err("Failed to enable the extended disk control interface.".into());
            }
        } else {
            return Err("The extended disk control interface is unsupported.".into());
        }

        Ok(())
    }

    pub fn enable_audio_buffer_status_callback(&self) -> bool {
        let data = retro_audio_buffer_status_callback {
            callback: Some(retro_audio_buffer_status_callback_fn),
        };

        self.set_audio_buffer_status_callback(data)
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn set_led_state(&self, led: i32, state: i32) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.led_interface {
            if let Some(set_led_state) = interface.set_led_state {
                unsafe { set_led_state(led, state) };
            }
        }
    }

    pub fn set_rumble_state(&self, port: u32, effect: retro_rumble_effect, strength: u16) -> bool {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.rumble_interface {
            if let Some(set_rumble_state) = interface.set_rumble_state {
                return unsafe { set_rumble_state(port, effect, strength) };
            }
        }

        false
    }

    pub fn start_perf_counter(&mut self, name: &'static str) {
        let mut interfaces = self.interfaces.write().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(start) = interface.perf_start {
                if let Some(register) = interface.perf_register {
                    let counter = interfaces
                        .perf_interface
                        .counters
                        .entry(name)
                        .or_insert_with(|| {
                            let ident = CString::new(name).unwrap();
                            let ptr = ident.as_ptr();

                            PerfCounter {
                                ident,
                                counter: retro_perf_counter {
                                    ident: ptr,
                                    start: 0,
                                    total: 0,
                                    call_cnt: 0,
                                    registered: false,
                                },
                            }
                        });

                    if !counter.counter.registered {
                        unsafe {
                            register(&mut counter.counter as *mut _);
                        }
                    }

                    unsafe {
                        start(&mut counter.counter as *mut _);
                    }
                }
            }
        }
    }

    pub fn stop_perf_counter(&mut self, name: &'static str) {
        let mut interfaces = self.interfaces.write().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(stop) = interface.perf_stop {
                use std::collections::hash_map::Entry;

                if let Entry::Occupied(counter) = interfaces.perf_interface.counters.entry(name) {
                    let counter = counter.into_mut();

                    if counter.counter.registered {
                        unsafe {
                            stop(&mut counter.counter as *mut _);
                        }
                    }
                }
            }
        }
    }

    pub fn perf_log(&self) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(log) = interface.perf_log {
                unsafe {
                    log();
                }
            }
        }
    }

    pub fn perf_get_time_usec(&self) -> i64 {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(get_time_usec) = interface.get_time_usec {
                return unsafe { get_time_usec() };
            }
        }

        0
    }

    pub fn perf_get_counter(&self) -> u64 {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(get_perf_counter) = interface.get_perf_counter {
                return unsafe { get_perf_counter() };
            }
        }

        0
    }

    pub fn get_cpu_features(&self) -> CpuFeatures {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.perf_interface.interface {
            if let Some(get_cpu_features) = interface.get_cpu_features {
                return unsafe { CpuFeatures::from_bits_unchecked(get_cpu_features()) };
            }
        }

        CpuFeatures::empty()
    }

    pub fn location_service_start(&self) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.location_interface {
            if let Some(start) = interface.start {
                unsafe { start() };
            }
        }
    }

    pub fn location_service_stop(&self) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.location_interface {
            if let Some(stop) = interface.stop {
                unsafe { stop() };
            }
        }
    }

    pub fn location_service_get_position(&self) -> Option<Position> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.location_interface {
            if let Some(get_position) = interface.get_position {
                let mut lat = 0f64;
                let mut lon = 0f64;
                let mut horiz_accuracy = 0f64;
                let mut vert_accuracy = 0f64;

                unsafe {
                    if !get_position(
                        &mut lat as *mut f64,
                        &mut lon as *mut f64,
                        &mut horiz_accuracy as *mut f64,
                        &mut vert_accuracy as *mut f64,
                    ) {
                        return None;
                    }
                };

                return Some(Position {
                    lat,
                    lon,
                    horiz_accuracy,
                    vert_accuracy,
                });
            }
        }

        None
    }

    pub fn location_service_set_interval(&self, interval_ms: u32, interval_distance: u32) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.location_interface {
            if let Some(set_interval) = interface.set_interval {
                unsafe { set_interval(interval_ms, interval_distance) };
            }
        }
    }

    pub fn midi_input_enabled(&self) -> bool {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.midi_interface {
            if let Some(input_enabled) = interface.input_enabled {
                return unsafe { input_enabled() };
            }
        }

        false
    }

    pub fn midi_output_enabled(&self) -> bool {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.midi_interface {
            if let Some(output_enabled) = interface.output_enabled {
                return unsafe { output_enabled() };
            }
        }

        false
    }

    pub fn midi_read_next(&self) -> Option<u8> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.midi_interface {
            if let Some(read) = interface.read {
                let mut value = 0;
                unsafe {
                    if read(&mut value as *mut u8) {
                        return Some(value);
                    }
                }
            }
        }

        None
    }

    pub fn midi_write_byte(&self, value: u8, delta_time: u32) -> bool {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.midi_interface {
            if let Some(write) = interface.write {
                return unsafe { write(value, delta_time) };
            }
        }

        false
    }

    pub fn midi_flush(&self) -> bool {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.midi_interface {
            if let Some(flush) = interface.flush {
                return unsafe { flush() };
            }
        }

        false
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_get_path(&self, handle: &mut retro_vfs_file_handle) -> Option<CString> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(get_path) = interface.get_path {
                let ptr = unsafe { get_path(handle) };
                if !ptr.is_null() {
                    let path = CStr::from_ptr(ptr).to_owned();
                    return Some(path);
                }
            }
        }

        None
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_open(
        &self,
        path: &str,
        mode: VfsFileOpenFlags,
        hints: VfsFileOpenHints,
    ) -> Result<retro_vfs_file_handle, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(open) = interface.open {
                let path = CString::new(path)?;

                let handle = unsafe { open(path.as_ptr(), mode.bits(), hints.bits()) };
                if !handle.is_null() {
                    return Ok(*handle);
                }
            }
        }

        Err("Failed to open file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_close(
        &self,
        mut handle: retro_vfs_file_handle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(close) = interface.close {
                if unsafe { close(&mut handle) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to close file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_size(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(size) = interface.size {
                let size = unsafe { size(handle) };
                if size >= 0 {
                    return Ok(size as u64);
                }
            }
        }

        Err("Failed to get file size".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_truncate(
        &self,
        handle: &mut retro_vfs_file_handle,
        length: i64, // no idea why the API wants signed values
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 2 {
            return Err(format!(
                "VFS interface version 2 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(truncate) = interface.truncate {
                if unsafe { truncate(handle, length) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to truncate file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_tell(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(tell) = interface.tell {
                let position = unsafe { tell(handle) };
                if position >= 0 {
                    return Ok(position as u64);
                }
            }
        }

        Err("Failed to get cursor position".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_seek(
        &self,
        handle: &mut retro_vfs_file_handle,
        offset: i64,
        seek_position: VfsSeekPosition,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(seek) = interface.seek {
                let position = unsafe { seek(handle, offset, seek_position as i32) };
                if position >= 0 {
                    return Ok(position as u64);
                }
            }
        }

        Err("Failed to seek into file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_read(
        &self,
        handle: &mut retro_vfs_file_handle,
        length: usize,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(read) = interface.read {
                let mut buffer = Vec::with_capacity(length);

                let read_length =
                    unsafe { read(handle, buffer.as_mut_ptr() as *mut _, length as u64) };
                if read_length >= 0 {
                    return Ok(buffer);
                }
            }
        }

        Err("Failed to read from file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_write(
        &self,
        handle: &mut retro_vfs_file_handle,
        buffer: &mut [u8],
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(write) = interface.write {
                let bytes_written =
                    unsafe { write(handle, buffer.as_mut_ptr() as *mut _, buffer.len() as u64) };
                if bytes_written >= 0 {
                    return Ok(bytes_written as u64);
                }
            }
        }

        Err("Failed to write to file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_flush(
        &self,
        handle: &mut retro_vfs_file_handle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(flush) = interface.flush {
                if unsafe { flush(handle) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to flush file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_remove(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(remove) = interface.remove {
                let path = CString::new(path)?;

                if unsafe { remove(path.as_ptr()) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to remove file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_rename(
        &self,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(rename) = interface.rename {
                let old_path = CString::new(old_path)?;
                let new_path = CString::new(new_path)?;

                if unsafe { rename(old_path.as_ptr(), new_path.as_ptr()) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to rename file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_stat(&self, path: &str) -> Result<(VfsStat, u64), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(stat) = interface.stat {
                let path = CString::new(path)?;
                let (stat, size) = unsafe {
                    let mut size = 0i32;
                    let value = stat(path.as_ptr(), &mut size);

                    (VfsStat::from_bits_unchecked(value), size)
                };

                if stat.is_empty() {
                    return Err(format!("Invalid stat bitmask: {stat:#?}").into());
                } else if size < 0 {
                    return Err(format!("Invalid file size: {size}").into());
                }

                return Ok((stat, size as u64));
            }
        }

        Err("Failed to stat file".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_mkdir(&self, dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(mkdir) = interface.mkdir {
                let dir = CString::new(dir)?;

                match unsafe { mkdir(dir.as_ptr()) } {
                    0 => return Ok(()),
                    -2 => return Err("Failed to create directory: Exists already".into()),
                    _ => (),
                }
            }
        }

        Err("Failed to create directory".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_opendir(
        &self,
        dir: &str,
        include_hidden: bool,
    ) -> Result<retro_vfs_dir_handle, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(opendir) = interface.opendir {
                let dir = CString::new(dir)?;

                let handle = unsafe { opendir(dir.as_ptr(), include_hidden) };
                if !handle.is_null() {
                    return Ok(*handle);
                }
            }
        }

        Err("Failed to open directory".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_readdir(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(readdir) = interface.readdir {
                if unsafe { readdir(handle) } {
                    return Ok(());
                }
            }
        }

        Err("Failed to read directory".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_dirent_get_name(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<CString, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(dirent_get_name) = interface.dirent_get_name {
                let ptr = unsafe { dirent_get_name(handle) };
                if !ptr.is_null() {
                    let name = CStr::from_ptr(ptr).to_owned();
                    return Ok(name);
                }
            }
        }

        Err("Failed to get entry name".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_dirent_is_dir(
        &self,
        handle: &mut retro_vfs_dir_handle,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(dirent_is_dir) = interface.dirent_is_dir {
                return Ok(unsafe { dirent_is_dir(handle) });
            }
        }

        Err("Failed to check if the entry is a directory".into())
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn vfs_closedir(
        &self,
        mut handle: retro_vfs_dir_handle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let interfaces = self.interfaces.read().unwrap();

        if interfaces.vfs_interface_info.supported_version < 3 {
            return Err(format!(
                "VFS interface version 3 required, but the frontend only supports version {}",
                interfaces.vfs_interface_info.supported_version
            )
            .into());
        }

        if let Some(interface) = interfaces.vfs_interface_info.interface {
            if let Some(closedir) = interface.closedir {
                if unsafe { closedir(&mut handle) } == 0 {
                    return Ok(());
                }
            }
        }

        Err("Failed to close directory".into())
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
// into_generic!(LoadGameSpecialContext<'a>, LoadGameContext, 'a);

make_context!(SetEnvironmentContext, #[doc = "Functions that are safe to be called in [`Core::on_set_environment`]"]);

impl<'a> SetEnvironmentContext<'a> {
    pub fn enable_proc_address_interface(&mut self) -> bool {
        self.set_proc_address_callback(retro_get_proc_address_interface {
            get_proc_address: Some(retro_get_proc_address_callback),
        })
    }

    pub fn enable_options_update_display_callback(&mut self) -> bool {
        self.set_core_options_update_display_callback(retro_core_options_update_display_callback {
            callback: Some(retro_core_options_update_display_callback_fn),
        })
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_vfs_interface(
        &mut self,
        min_version: u32,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let mut interfaces = self.interfaces.write().unwrap();

        let info = self.get_vfs_interface(retro_vfs_interface_info {
            required_interface_version: min_version,
            iface: std::ptr::null_mut(),
        });

        if let Some(info) = info {
            if !info.iface.is_null() && info.required_interface_version >= min_version {
                interfaces.vfs_interface_info = VfsInterfaceInfo {
                    supported_version: info.required_interface_version,
                    interface: Some(*info.iface),
                }
            }
        }

        if interfaces.vfs_interface_info.interface.is_some() {
            Ok(interfaces.vfs_interface_info.supported_version)
        } else {
            Err("Failed to enable VFS interface".into())
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
    pub fn enable_frame_time_callback(&self, reference: i64) {
        self.set_frame_time_callback(retro_frame_time_callback {
            callback: Some(retro_frame_time_callback_fn),
            reference,
        });
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_camera_interface(
        &mut self,
        caps: u64,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use retro_camera_buffer::*;

        let enable_raw = caps & (1 << RETRO_CAMERA_BUFFER_RAW_FRAMEBUFFER as u64) > 0;
        let enable_opengl = caps & (1 << RETRO_CAMERA_BUFFER_OPENGL_TEXTURE as u64) > 0;

        let mut interfaces = self.interfaces.write().unwrap();

        interfaces.camera_interface = self.get_camera_interface(retro_camera_callback {
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
        });

        if interfaces.camera_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable camera interface".into())
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_sensor_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.sensor_interface = ctx.get_sensor_interface();

        if interfaces.sensor_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable sensor interface".into())
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_led_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.led_interface = ctx.get_led_interface();

        if interfaces.led_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable led interface".into())
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn enable_midi_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.midi_interface = ctx.get_midi_interface();

        if interfaces.midi_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable MIDI interface".into())
        }
    }

    pub fn enable_location_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.location_interface = ctx.get_location_callback();

        if let Some(mut interface) = interfaces.location_interface {
            interface.initialized = Some(retro_location_lifetime_status_initialized_callback);
            interface.deinitialized = Some(retro_location_lifetime_status_deinitialized_callback);
            Ok(())
        } else {
            Err("Failed to enable location interface".into())
        }
    }

    pub fn enable_rumble_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.rumble_interface = self.get_rumble_interface();

        if interfaces.rumble_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable rumble interface".into())
        }
    }

    pub fn enable_perf_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        let mut interfaces = self.interfaces.write().unwrap();
        interfaces.perf_interface = PerfCounters {
            interface: ctx.get_perf_interface(),
            counters: HashMap::new(),
        };

        if interfaces.perf_interface.interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable perf interface".into())
        }
    }

    pub unsafe fn enable_hw_render(
        &mut self,
        context_type: retro_hw_context_type,
        bottom_left_origin: bool,
        version_major: u32,
        version_minor: u32,
        debug_context: bool,
    ) -> bool {
        let data = retro_hw_render_callback {
            context_type,
            bottom_left_origin,
            version_major,
            version_minor,
            cache_context: true,
            debug_context,

            depth: false,   // obsolete
            stencil: false, // obsolete

            context_reset: Some(retro_hw_context_reset_callback),
            context_destroy: Some(retro_hw_context_destroyed_callback),

            // Set by the frontend
            get_current_framebuffer: None,
            get_proc_address: None,
        };

        self.set_hw_render(data)
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
            let len = samples.len() as u64;

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
    pub(crate) last_pitch: &'a mut u64,

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

impl<'a> RunContext<'_> {
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
    pub fn get_input_state(&self, port: u32, device: u32, index: u32, id: u32) -> u16 {
        if let Some(callback) = self.input_state_callback {
            unsafe { (callback)(port, device, index, id) as u16 }
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
                return JoypadState::from_bits_truncate((callback)(
                    port,
                    RETRO_DEVICE_JOYPAD,
                    index,
                    RETRO_DEVICE_ID_JOYPAD_MASK,
                ) as u16);
            }

            // Fallback
            return self.get_joypad_state(port, index);
        }

        JoypadState::empty()
    }

    /// Draws a new frame if [`RunContext::video_refresh_callback`] has been set
    pub fn draw_frame(&mut self, data: &[u8], width: u32, height: u32, pitch: u64) {
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

    pub fn draw_framebuffer(&mut self, framebuffer: retro_framebuffer, pitch: u64) {
        if let Some(callback) = self.video_refresh_callback {
            *self.had_frame = true;
            *self.last_width = framebuffer.width;
            *self.last_height = framebuffer.height;
            *self.last_pitch = pitch;

            unsafe {
                (callback)(
                    framebuffer.data,
                    framebuffer.width,
                    framebuffer.height,
                    pitch,
                )
            }
        }
    }

    pub fn draw_hardware_frame(&mut self, width: u32, height: u32, pitch: u64) {
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
    pub fn camera_start(&self) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.camera_interface {
            if let Some(start) = interface.start {
                unsafe { start() };
            }
        }
    }

    #[proc::unstable(feature = "env-commands")]
    pub fn camera_stop(&self) {
        let interfaces = self.interfaces.read().unwrap();

        if let Some(interface) = interfaces.camera_interface {
            if let Some(stop) = interface.stop {
                unsafe { stop() };
            }
        }
    }
}
