use rust_libretro::{
    contexts::*, core::Core, env_version, input_descriptors, proc::*, retro_core, sys::*, types::*,
};
use std::ffi::CString;

const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &input_descriptors!(
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "Up" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "Down" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "Left" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "Right" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "Action" },
);

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

    pixels: Vec<u8>,
    timer: i64,
    even: bool,
}

retro_core!(ExampleCore {
    option_1: false,
    option_2: true,

    pixels: vec![0; 800 * 600 * 4],
    timer: 5_000_001,
    even: true,
});

impl Core for ExampleCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("Example Core").unwrap(),
            library_version: CString::new(env_version!("CARGO_PKG_VERSION").to_string()).unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        if !initial {
            return;
        }

        ctx.set_support_no_game(true)
            .expect("telling the frontend that we can run without content to succeed");
    }

    fn on_init(&mut self, ctx: &mut InitContext) {
        let gctx: GenericContext = ctx.into();
        gctx.set_input_descriptors(INPUT_DESCRIPTORS)
            .expect("setting input descriptors to succeed");
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

    fn on_load_game(
        &mut self,
        _info: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> rust_libretro::core::Result<()> {
        ctx.set_pixel_format(PixelFormat::XRGB8888).map_err(|_| {
            rust_libretro::anyhow::anyhow!("Required pixel format “XRGB8888” is not supported")
        })?;

        let _ = ctx.set_performance_level(0);

        if let Err(err) =
            ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t)
        {
            log::error!("Failed to enable frame time callback: {}", err);
        }

        let gctx: GenericContext = ctx.into();
        if let Err(err) = gctx.enable_audio_callback() {
            log::error!("Failed to enable audio callback: {}", err);
        }

        Ok(())
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("foo_option_1") {
            Ok(Some("true")) => self.option_1 = true,
            Ok(Some("false")) => self.option_1 = false,
            _ => (),
        }

        match ctx.get_variable("foo_option_2") {
            Ok(Some("true")) => self.option_2 = true,
            Ok(Some("false")) => self.option_2 = false,
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

            ctx.draw_frame(self.pixels.as_ref(), width, height, width as usize * 4);
        } else if ctx.can_dupe() {
            ctx.dupe_frame();
        }
    }

    fn on_write_audio(&mut self, ctx: &mut AudioContext) {
        ctx.queue_audio_sample(0, 0);
    }
}
