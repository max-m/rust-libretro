use super::*;

macro_rules! into_generic {
    ($type:ty, $lifetime:tt) => {
        into_generic!($type, GenericContext, $lifetime);
    };
    ($type:ty, $other:ident, $lifetime:tt) => {
        impl<$lifetime> From<&$type> for $other<$lifetime> {
            fn from(other: &$type) -> $other<$lifetime> {
                $other::new(other.environment_callback)
            }
        }

        impl<$lifetime> From<&mut $type> for $other<$lifetime> {
            fn from(other: &mut $type) -> $other<$lifetime> {
                $other::new(other.environment_callback)
            }
        }
    };
}

macro_rules! make_context {
    ($name:ident $(, #[doc = $doc:tt ])?) => {
        $(#[doc = $doc])?
        pub struct $name<'a> {
            pub(crate) environment_callback: &'a retro_environment_t,
        }

        impl<'a> $name<'a> {
            pub fn new(environment_callback: &'a retro_environment_t) -> Self {
                Self {
                    environment_callback
                }
            }
        }

        into_generic!($name<'a>, 'a);
    };
}

/// Exposes environment callbacks that are safe to call in every context.
pub struct GenericContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,
}

impl<'a> GenericContext<'a> {
    pub fn new(environment_callback: &'a retro_environment_t) -> Self {
        Self {
            environment_callback,
        }
    }

    pub unsafe fn environment_callback(&self) -> &'a retro_environment_t {
        self.environment_callback
    }

    pub fn enable_keyboard_callback(&self) -> bool {
        self.set_keyboard_callback(retro_keyboard_callback {
            callback: Some(retro_keyboard_callback_fn),
        })
    }

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
}

pub type ResetContext<'a> = GenericContext<'a>;
pub type DeinitContext<'a> = GenericContext<'a>;
pub type GetSerializeSizeContext<'a> = GenericContext<'a>;
pub type SerializeContext<'a> = GenericContext<'a>;
pub type UnserializeContext<'a> = GenericContext<'a>;
pub type UnloadGameContext<'a> = GenericContext<'a>;
pub type CheatResetContext<'a> = GenericContext<'a>;
pub type CheatSetContext<'a> = GenericContext<'a>;
pub type GetRegionContext<'a> = GenericContext<'a>;
pub type GetMemoryDataContext<'a> = GenericContext<'a>;
pub type GetMemorySizeContext<'a> = GenericContext<'a>;

make_context!(GetAvInfoContext);
make_context!(InitContext);
make_context!(OptionsChangedContext);

make_context!(LoadGameSpecialContext);
// into_generic!(LoadGameSpecialContext<'a>, LoadGameContext, 'a);

make_context!(SetEnvironmentContext);

impl<'a> SetEnvironmentContext<'a> {
    pub fn enable_proc_address_interface(&mut self) -> bool {
        self.set_proc_address_callback(retro_get_proc_address_interface {
            get_proc_address: Some(retro_get_proc_address_callback),
        })
    }
}

pub struct LoadGameContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,

    pub(crate) camera_interface: &'a mut Option<retro_camera_callback>,
    pub(crate) perf_interface: &'a mut Option<retro_perf_callback>,
    pub(crate) location_interface: &'a mut Option<retro_location_callback>,
    pub(crate) rumble_interface: &'a mut Option<retro_rumble_interface>,

    #[cfg(feature = "unstable-env-commands")]
    pub(crate) sensor_interface: &'a mut Option<retro_sensor_interface>,
}

impl<'a> LoadGameContext<'a> {
    pub fn new(
        environment_callback: &'a retro_environment_t,
        camera_interface: &'a mut Option<retro_camera_callback>,
        perf_interface: &'a mut Option<retro_perf_callback>,
        location_interface: &'a mut Option<retro_location_callback>,
        rumble_interface: &'a mut Option<retro_rumble_interface>,

        #[cfg(feature = "unstable-env-commands")] sensor_interface: &'a mut Option<
            retro_sensor_interface,
        >,
    ) -> Self {
        Self {
            environment_callback,

            camera_interface,
            perf_interface,
            location_interface,
            rumble_interface,

            #[cfg(feature = "unstable-env-commands")]
            sensor_interface,
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
        *self.camera_interface = self.get_camera_interface(retro_camera_callback {
            caps,
            width,
            height,

            start: None,
            stop: None,

            frame_raw_framebuffer: Some(retro_camera_frame_raw_framebuffer_callback),
            frame_opengl_texture: Some(retro_camera_frame_opengl_texture_callback),
            initialized: Some(retro_camera_initialized_callback),
            deinitialized: Some(retro_camera_deinitialized_callback),
        });

        if self.camera_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable camera interface".into())
        }
    }

    #[cfg(feature = "unstable-env-commands")]
    #[proc::unstable(feature = "env-commands")]
    pub fn enable_sensor_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        *self.sensor_interface = ctx.get_sensor_interface();

        if self.sensor_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable sensor interface".into())
        }
    }

    pub fn enable_perf_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        *self.perf_interface = ctx.get_perf_interface();

        if self.perf_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable performance interface".into())
        }
    }

    pub fn enable_location_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ctx: GenericContext = self.into();
        *self.location_interface = ctx.get_location_callback();

        if self.location_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to enable location interface".into())
        }
    }

    pub fn enable_rumble_interface(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.rumble_interface = self.get_rumble_interface();

        if self.rumble_interface.is_some() {
            Ok(())
        } else {
            Err("Failed to rumble location interface".into())
        }
    }
}
into_generic!(LoadGameContext<'a>, 'a);

pub struct AudioContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,

    pub(crate) audio_sample_batch_callback: &'a retro_audio_sample_batch_t,
    pub(crate) audio_sample_callback: &'a retro_audio_sample_t,
}

impl AudioContext<'_> {
    /// Renders multiple audio frames in one go if [`CoreWrapper::audio_sample_batch_callback`] has been set.
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

    /// Renders a single audio frame if [`CoreWrapper::audio_sample_callback`] has been set.
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

/// Exposes callbacks that are safe to call in [`Core::on_run`]
///
/// For a description of the callbacks see [`CoreWrapper`].
pub struct RunContext<'a> {
    pub(crate) environment_callback: &'a retro_environment_t,

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

    /// Polls for input if [`CoreWrapper::input_poll_callback`] has been set
    pub fn poll_input(&self) {
        if let Some(callback) = self.input_poll_callback {
            unsafe {
                (callback)();
            }
        }
    }

    /// Gets the input state for the given player and device if [`CoreWrapper::input_state_callback`] has been set
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

    /// Draws a new frame if [`CoreWrapper::video_refresh_callback`] has been set
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
}
