use rust_libretro::{contexts::*, core::Core, proc::*, retro_core, sys::*, types::*};
use std::ffi::CString;

// TODO: Write a neater abstraction / macro
const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &[
    retro_input_descriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id: RETRO_DEVICE_ID_JOYPAD_UP,
        description: b"Up\0" as *const u8 as *const libc::c_char,
    },
    retro_input_descriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id: RETRO_DEVICE_ID_JOYPAD_DOWN,
        description: b"Down\0" as *const u8 as *const libc::c_char,
    },
    retro_input_descriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id: RETRO_DEVICE_ID_JOYPAD_LEFT,
        description: b"Left\0" as *const u8 as *const libc::c_char,
    },
    retro_input_descriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id: RETRO_DEVICE_ID_JOYPAD_RIGHT,
        description: b"Right\0" as *const u8 as *const libc::c_char,
    },
    retro_input_descriptor {
        port: 0,
        device: RETRO_DEVICE_JOYPAD,
        index: 0,
        id: RETRO_DEVICE_ID_JOYPAD_A,
        description: b"Action\0" as *const u8 as *const libc::c_char,
    },
    // End of list
    retro_input_descriptor {
        port: 0,
        device: 0,
        index: 0,
        id: 0,
        description: 0 as *const libc::c_char,
    },
];

#[derive(CoreOptions)]
#[categories({
    "advanced_settings",
    "Advanced",
    "Options affecting low-level emulation performance and accuracy."
},{
    "not_so_advanced_settings",
    "Not So Advanced",
    "Options not affecting low-level emulation performance and accuracy."
})]
#[options({
    "foo_option_1",
    "Advanced > Speed hack coprocessor X",
    "Speed hack coprocessor X",
    "Setting 'Advanced > Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "Setting 'Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "advanced_settings",
    {
        { "false" },
        { "true" },
        { "unstable", "Turbo (Unstable)" },
    }
}, {
    "foo_option_2",
    "Simple > Toggle Something",
    "Toggle Something",
    "Setting 'Simple > Toggle Something' to 'true' does something.",
    "Setting 'Toggle Something' to 'true' does something.",
    "not_so_advanced_settings",
    {
        { "false" },
        { "true" },
    }
})]
struct ExampleCore {
    option_1: bool,
    option_2: bool,

    pixels: [u8; 800 * 600 * 4],
    timer: i64,
    even: bool,
}

retro_core!(ExampleCore {
    option_1: false,
    option_2: true,

    pixels: [0; 800 * 600 * 4],
    timer: 5_000_001,
    even: true,
});

impl Core for ExampleCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("Example Core").unwrap(),
            library_version: CString::new("0.1.0").unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        if !initial {
            return;
        }

        ctx.set_support_no_game(true);
    }

    fn on_init(&mut self, ctx: &mut InitContext) {
        let gctx: GenericContext = ctx.into();
        gctx.set_input_descriptors(INPUT_DESCRIPTORS);
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: 800,
                base_height: 600,
                max_width: 800,
                max_height: 600,
                aspect_ratio: 0.0,
            },
            timing: retro_system_timing {
                fps: 60.0,
                sample_rate: 0.0,
            },
        }
    }

    fn on_load_game(&mut self, _info: Option<retro_game_info>, ctx: &mut LoadGameContext) -> bool {
        ctx.set_pixel_format(retro_pixel_format::RETRO_PIXEL_FORMAT_XRGB8888);
        ctx.set_performance_level(0);
        ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t);

        let gctx: GenericContext = ctx.into();
        gctx.enable_audio_callback();

        true
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("foo_option_1") {
            Some("true") => self.option_1 = true,
            Some("false") => self.option_1 = false,
            _ => (),
        }

        match ctx.get_variable("foo_option_2") {
            Some("true") => self.option_2 = true,
            Some("false") => self.option_2 = false,
            _ => (),
        }
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, delta_us: Option<i64>) {
        let gctx: GenericContext = ctx.into();

        self.timer += delta_us.unwrap_or(16_666);

        let input = unsafe { ctx.get_joypad_bitmask(0, 0) };

        if input.contains(JoypadState::START) && input.contains(JoypadState::SELECT) {
            return gctx.shutdown();
        }

        if !ctx.can_dupe() || self.timer >= 1_000_000 || input.contains(JoypadState::A) {
            self.timer = 0;
            self.even = !self.even;

            let width = 800u32;
            let height = 600u32;

            let color_a = if self.even { 0xFF } else { 0 };
            let color_b = !color_a;

            for (i, chunk) in self.pixels.chunks_exact_mut(4).enumerate() {
                let x = (i % width as usize) as f64 / width as f64;
                let y = (i / width as usize) as f64 / height as f64;

                let total = (50.0f64 * x).floor() + (37.5f64 * y).floor();
                let even = total as usize % 2 == 0;

                let color = if even { color_a } else { color_b };

                chunk.fill(color);
            }

            ctx.draw_frame(&self.pixels, width, height, width as u64 * 4);
        } else if ctx.can_dupe() {
            ctx.dupe_frame();
        }
    }

    fn on_write_audio(&mut self, ctx: &mut AudioContext) {
        ctx.queue_audio_sample(0, 0);
    }
}
