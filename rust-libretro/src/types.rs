//! Rust versions of libretro data structures.
use super::*;
use std::collections::HashMap;

/// Static information about the [`Core`] implementation.
#[derive(Debug, Default)]
pub struct SystemInfo {
    /// Descriptive name of library. Should not
    /// contain any version numbers, etc.
    pub library_name: CString,

    /// Descriptive version of the core.
    pub library_version: CString,

    /// A string listing probably content extensions the core will be able to
    /// load, separated with pipe. I.e. "bin|rom|iso".
    /// Typically used for a GUI to filter out extensions.
    pub valid_extensions: CString,

    /// libretro cores that need to have direct access to their content
    /// files, including cores which use the path of the content files to
    /// determine the paths of other files, should set `need_fullpath` to true.
    ///
    /// Cores should strive for setting `need_fullpath` to [`false`],
    /// as it allows the frontend to perform patching, etc.
    ///
    /// If `need_fullpath` is [`true`] and [`Core::on_load_game`] is called:
    ///    - [`retro_game_info::path`] is guaranteed to have a valid path
    ///    - [`retro_game_info::data`] and [`retro_game_info::size`] are invalid
    ///
    /// If `need_fullpath` is [`false`] and [`Core::on_load_game`] is called:
    ///    - [`retro_game_info::path`] may be NULL
    ///    - [`retro_game_info::data`] and [`retro_game_info::size`] are guaranteed
    ///      to be valid
    ///
    /// See also:
    ///    - [`environment::get_system_directory`]
    ///    - [`environment::get_save_directory`]
    pub need_fullpath: bool,

    /// If [`true`], the frontend is not allowed to extract any archives before
    /// loading the real content.
    /// Necessary for certain libretro implementations that load games
    /// from zipped archives.
    pub block_extract: bool,
}

bitflags::bitflags! {
    /// Bitflags indicating the type of input device
    pub struct RetroDevice: u8 {
        /// Input disabled
        const NONE = (1 << RETRO_DEVICE_NONE);

        /// The JOYPAD is called RetroPad. It is essentially a Super Nintendo
        /// controller, but with additional L2/R2/L3/R3 buttons, similar to a
        /// PS1 DualShock.
        const JOYPAD = (1 << RETRO_DEVICE_JOYPAD);

        /// The mouse is a simple mouse, similar to Super Nintendo's mouse.
        /// X and Y coordinates are reported relatively to last poll (poll callback).
        /// It is up to the libretro implementation to keep track of where the mouse
        /// pointer is supposed to be on the screen.
        /// The frontend must make sure not to interfere with its own hardware
        /// mouse pointer.
        const MOUSE = (1 << RETRO_DEVICE_MOUSE);

        /// KEYBOARD device lets one poll for raw key pressed.
        /// It is poll based, so input callback will return with the current
        /// pressed state.
        /// For event/text based keyboard input, see
        /// [`RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK`].
        const KEYBOARD = (1 << RETRO_DEVICE_KEYBOARD);

        /// LIGHTGUN device is similar to Guncon-2 for PlayStation 2.
        /// It reports X/Y coordinates in screen space (similar to the pointer)
        /// in the range `[-0x8000, 0x7fff]` in both axes, with zero being center and
        /// `-0x8000` being out of bounds.
        /// As well as reporting on/off screen state. It features a trigger,
        /// start/select buttons, auxiliary action buttons and a
        /// directional pad. A forced off-screen shot can be requested for
        /// auto-reloading function in some games.
        const LIGHTGUN = (1 << RETRO_DEVICE_LIGHTGUN);

        /// The ANALOG device is an extension to JOYPAD (RetroPad).
        /// Similar to DualShock2 it adds two analog sticks and all buttons can
        /// be analog. This is treated as a separate device type as it returns
        /// axis values in the full analog range of `[-0x7fff, 0x7fff]`,
        /// although some devices may return `-0x8000`.
        /// Positive X axis is right. Positive Y axis is down.
        /// Buttons are returned in the range `[0, 0x7fff]`.
        /// Only use ANALOG type when polling for analog values.
        const ANALOG = (1 << RETRO_DEVICE_ANALOG);

        /// Abstracts the concept of a pointing mechanism, e.g. touch.
        /// This allows libretro to query in absolute coordinates where on the
        /// screen a mouse (or something similar) is being placed.
        /// For a touch centric device, coordinates reported are the coordinates
        /// of the press.
        ///
        /// Coordinates in X and Y are reported as:
        /// `[-0x7fff, 0x7fff]`: `-0x7fff` corresponds to the far left/top of the screen,
        /// and `0x7fff` corresponds to the far right/bottom of the screen.
        /// The "screen" is here defined as area that is passed to the frontend and
        /// later displayed on the monitor.
        ///
        /// The frontend is free to scale/resize this screen as it sees fit, however,
        /// `(X, Y) = (-0x7fff, -0x7fff)` will correspond to the top-left pixel of the
        /// game image, etc.
        ///
        /// To check if the pointer coordinates are valid (e.g. a touch display
        /// actually being touched), PRESSED returns 1 or 0.
        ///
        /// If using a mouse on a desktop, PRESSED will usually correspond to the
        /// left mouse button, but this is a frontend decision.
        /// PRESSED will only return 1 if the pointer is inside the game screen.
        ///
        /// For multi-touch, the index variable can be used to successively query
        /// more presses.
        /// If `index = 0` returns `true` for `_PRESSED`, coordinates can be extracted
        /// with `_X, _Y` for `index = 0`. One can then query `_PRESSED, _X, _Y` with
        /// `index = 1`, and so on.
        /// Eventually `_PRESSED` will return `false` for an index. No further presses
        /// are registered at this point.
        const POINTER = (1 << RETRO_DEVICE_POINTER);
    }
}
#[test]
fn retro_device_struct_size() {
    assert_eq!(
        std::mem::size_of::<RetroDevice>(),
        ((RETRO_DEVICE_MASK + 1) >> RETRO_DEVICE_TYPE_SHIFT) as usize
    );
}

bitflags::bitflags! {
    /// Signifies quirks of the [`Core`]’s serialization feature (if any).
    pub struct SerializationQuirks: u32 {
        /// Serialized state is incomplete in some way. Set if serialization is
        /// usable in typical end-user cases but should not be relied upon to
        /// implement frame-sensitive frontend features such as netplay or
        /// rerecording.
        const INCOMPLETE = RETRO_SERIALIZATION_QUIRK_INCOMPLETE;

        /// The core must spend some time initializing before serialization is
        /// supported. [`Core::on_serialize`] will initially fail; [`Core::on_unserialize`]
        /// and [`Core::get_serialize_size`] may or may not work correctly either.
        const MUST_INITIALIZE = RETRO_SERIALIZATION_QUIRK_MUST_INITIALIZE;

        /// Serialization size may change within a session.
        const CORE_VARIABLE_SIZE = RETRO_SERIALIZATION_QUIRK_CORE_VARIABLE_SIZE;

        /// Set by the frontend to acknowledge that it supports variable-sized
        /// states.
        const FRONT_VARIABLE_SIZE = RETRO_SERIALIZATION_QUIRK_FRONT_VARIABLE_SIZE;

        /// Serialized state can only be loaded during the same session.
        const SINGLE_SESSION = RETRO_SERIALIZATION_QUIRK_SINGLE_SESSION;

        /// Serialized state cannot be loaded on an architecture with a different
        /// endianness from the one it was saved on.
        const ENDIAN_DEPENDENT = RETRO_SERIALIZATION_QUIRK_ENDIAN_DEPENDENT;

        /// Serialized state cannot be loaded on a different platform from the one it
        /// was saved on for reasons other than endianness, such as word size
        /// dependence
        const PLATFORM_DEPENDENT = RETRO_SERIALIZATION_QUIRK_PLATFORM_DEPENDENT;
    }
}

bitflags::bitflags! {
    pub struct CpuFeatures: u64 {
        const SSE = RETRO_SIMD_SSE as u64;
        const SSE2 = RETRO_SIMD_SSE2 as u64;
        const VMX = RETRO_SIMD_VMX as u64;
        const VMX128 = RETRO_SIMD_VMX128 as u64;
        const AVX = RETRO_SIMD_AVX as u64;
        const NEON = RETRO_SIMD_NEON as u64;
        const SSE3 = RETRO_SIMD_SSE3 as u64;
        const SSSE3 = RETRO_SIMD_SSSE3 as u64;
        const MMX = RETRO_SIMD_MMX as u64;
        const MMXEXT = RETRO_SIMD_MMXEXT as u64;
        const SSE4 = RETRO_SIMD_SSE4 as u64;
        const SSE42 = RETRO_SIMD_SSE42 as u64;
        const AVX2 = RETRO_SIMD_AVX2 as u64;
        const VFPU = RETRO_SIMD_VFPU as u64;
        const PS = RETRO_SIMD_PS as u64;
        const AES = RETRO_SIMD_AES as u64;
        const VFPV3 = RETRO_SIMD_VFPV3 as u64;
        const VFPV4 = RETRO_SIMD_VFPV4 as u64;
        const POPCNT = RETRO_SIMD_POPCNT as u64;
        const MOVBE = RETRO_SIMD_MOVBE as u64;
        const CMOV = RETRO_SIMD_CMOV as u64;
        const ASIMD = RETRO_SIMD_ASIMD as u64;
    }
}

/// Used in [`environment::set_message_ext`] to signal some ongoing progress.
pub enum MessageProgress {
    /// The message is unmetered or the progress cannot be determined.
    Indeterminate,

    /// The progress as a percentage (0 - 100).
    Percentage(u8),
}

impl MessageProgress {
    pub fn indeterminate() -> Self {
        MessageProgress::Indeterminate
    }

    pub fn percentage(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(MessageProgress::Percentage(value))
        } else {
            None
        }
    }

    pub fn as_i8(&self) -> i8 {
        match *self {
            MessageProgress::Percentage(value) => value as i8,
            MessageProgress::Indeterminate => -1,
        }
    }
}

/// Screen rotation in degrees
pub enum Rotation {
    None,

    Clockwise90,
    Clockwise180,
    Clockwise270,

    CounterClockwise90,
    CounterClockwise180,
    CounterClockwise270,
}

impl Rotation {
    pub fn get_env_value(&self) -> u32 {
        match self {
            Rotation::None => 0,

            Rotation::Clockwise90 => 3,
            Rotation::Clockwise180 => 2,
            Rotation::Clockwise270 => 1,

            Rotation::CounterClockwise90 => 1,
            Rotation::CounterClockwise180 => 2,
            Rotation::CounterClockwise270 => 3,
        }
    }
}

#[derive(Debug)]
pub struct PerfCounter {
    #[allow(unused)]
    // Borrowed by the `retro_perf_counter`.
    pub(crate) ident: CString,
    pub(crate) counter: retro_perf_counter,
}

#[derive(Debug, Default)]
pub struct PerfCounters {
    pub interface: Option<retro_perf_callback>,
    pub counters: HashMap<&'static str, PerfCounter>,
}

#[derive(Debug, Default)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
    pub horiz_accuracy: f64,
    pub vert_accuracy: f64,
}

/// Data structures used by experimental libretro environment function calls
#[proc::unstable(feature = "env-commands")]
pub mod unstable {
    use core::marker::PhantomData;
    use rust_libretro_sys::*;

    bitflags::bitflags! {
        /// Tells the core if the frontend wants audio or video.
        pub struct AudioVideoEnable: u32 {
            /// When this bit is **not** set:
            /// * The frontend wants the core: to not generate any video,
            ///   including presenting frames via hardware acceleration.
            /// * The frontend's video frame callback will do nothing.
            /// * After running the frame, the video output of the next frame should be
            ///   no different than if video was enabled, and saving and loading state
            ///   should have no issues.
            const ENABLE_VIDEO = 0b0001;

            /// When this bit is **not** set:
            /// * The frontend wants the core to not generate any audio.
            /// * The frontend's audio callbacks will do nothing.
            /// * After running the frame, the audio output of the next frame should be
            ///   no different than if audio was enabled, and saving and loading state
            ///   should have no issues.
            const ENABLE_AUDIO = 0b0010;

            /// When this bit is set:
            /// * Guaranteed to be created by the same binary that will load them.
            /// * Will not be written to or read from the disk.
            /// * Suggest that the core assumes loading state will succeed.
            /// * Suggest that the core updates its memory buffers in-place if possible.
            /// * Suggest that the core skips clearing memory.
            /// * Suggest that the core skips resetting the system.
            /// * Suggest that the core may skip validation steps.
            const USE_FAST_SAVESTATES = 0b0100;

            /// When this bit is set:
            /// * Used for a secondary core when running ahead.
            /// * Indicates that the frontend will never need audio from the core.
            /// * Suggests that the core may stop synthesizing audio, but this should not
            ///   compromise emulation accuracy.
            /// * Audio output for the next frame does not matter, and the frontend will
            ///   never need an accurate audio state in the future.
            /// * State will never be saved when using Hard Disable Audio.
            const HARD_DISABLE_AUDIO = 0b1000;
        }
    }

    bitflags::bitflags! {
        /// Joypad button mask
        pub struct JoypadState: u16 {
            const B      = 0b0000_0000_0000_0001;
            const Y      = 0b0000_0000_0000_0010;
            const SELECT = 0b0000_0000_0000_0100;
            const START  = 0b0000_0000_0000_1000;

            const UP     = 0b0000_0000_0001_0000;
            const DOWN   = 0b0000_0000_0010_0000;
            const LEFT   = 0b0000_0000_0100_0000;
            const RIGHT  = 0b0000_0000_1000_0000;

            const A      = 0b0000_0001_0000_0000;
            const X      = 0b0000_0010_0000_0000;
            const L      = 0b0000_0100_0000_0000;
            const R      = 0b0000_1000_0000_0000;

            const L2     = 0b0001_0000_0000_0000;
            const R2     = 0b0010_0000_0000_0000;
            const L3     = 0b0100_0000_0000_0000;
            const R3     = 0b1000_0000_0000_0000;
        }
    }

    #[derive(Debug, Default)]
    pub struct VfsInterfaceInfo {
        pub(crate) supported_version: u32,
        pub(crate) interface: Option<retro_vfs_interface>,
    }

    bitflags::bitflags! {
        pub struct VfsFileOpenFlags: u32 {
            const READ = RETRO_VFS_FILE_ACCESS_READ;
            const WRITE = RETRO_VFS_FILE_ACCESS_WRITE;
            const READ_WRITE = RETRO_VFS_FILE_ACCESS_READ_WRITE;
            const UPDATE_EXISTING = RETRO_VFS_FILE_ACCESS_UPDATE_EXISTING;
        }
    }

    bitflags::bitflags! {
        pub struct VfsFileOpenHints: u32 {
            const NONE = RETRO_VFS_FILE_ACCESS_HINT_NONE;
            const FREQUENT_ACCESS = RETRO_VFS_FILE_ACCESS_HINT_FREQUENT_ACCESS;
        }
    }

    #[repr(i32)]
    pub enum VfsSeekPosition {
        Start = RETRO_VFS_SEEK_POSITION_START as i32,
        Current = RETRO_VFS_SEEK_POSITION_CURRENT as i32,
        End = RETRO_VFS_SEEK_POSITION_END as i32,
    }

    bitflags::bitflags! {
        pub struct VfsStat: i32 {
            const STAT_IS_VALID = RETRO_VFS_STAT_IS_VALID as i32;
            const STAT_IS_DIRECTORY = RETRO_VFS_STAT_IS_DIRECTORY as i32;
            const STAT_IS_CHARACTER_SPECIAL = RETRO_VFS_STAT_IS_CHARACTER_SPECIAL as i32;
        }
    }

    bitflags::bitflags! {
        pub struct MemoryAccess: u32 {
            const WRITE = RETRO_MEMORY_ACCESS_WRITE;
            const READ = RETRO_MEMORY_ACCESS_READ;
        }
    }

    bitflags::bitflags! {
        pub struct MemoryType: u32 {
            const UNCACHED = 0;
            const CACHED = RETRO_MEMORY_TYPE_CACHED;
        }
    }

    // TODO: Can we get rid of the raw pointer and PhantomData in an ergonomic way?
    pub struct Framebuffer<'a> {
        pub data: *mut u8,
        pub phantom: PhantomData<&'a mut [u8]>,

        pub width: u32,
        pub height: u32,
        pub pitch: usize,
        pub format: retro_pixel_format,
        pub access_flags: MemoryAccess,
        pub memory_flags: MemoryType,
    }
}
pub use unstable::*;
