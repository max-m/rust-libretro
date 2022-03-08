//! Port of <https://github.com/libretro/libretro-samples/tree/7418a585efd24c6506ca5f09f90c36268f0074ed/tests/test>
//!
//! Original license:
//! Copyright  (C) 2010-2015 The RetroArch team
//!
//! Permission is hereby granted, free of charge,
//! to any person obtaining a copy of this software and associated documentation files (the "Software"),
//! to deal in the Software without restriction, including without limitation the rights to
//! use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software,
//! and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
//! INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
//! WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use libc::c_char;
use rust_libretro::{contexts::*, core::Core, proc::CoreOptions, retro_core, sys::*, types::*};
use std::ffi::CString;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const PORTS: usize = 2;

#[derive(CoreOptions)]
#[categories({
    "video_settings",
    "Video",
    "Options related to video output."
},{
    "audio_settings",
    "Audio",
    "Options related to audio output."
},{
    "input_settings",
    "Input",
    "Options related to input devices."
})]
#[options({
    "test_aspect",
    "Video > Aspect Ratio",
    "Aspect Ratio",
    "Setting 'Video > Aspect Ratio' forces the aspect ratio to either 4:3 or 16:9.",
    "Setting 'Aspect Ratio' forces the aspect ratio to either 4:3 or 16:9.",
    "video_settings",
    {
        { "4:3" },
        { "16:9" },
    }
}, {
    "test_samplerate",
    "Audio > Sample Rate",
    "Sample Rate",
    "Setting 'Audio > Sample Rate' tells the Core how many audio samples per second it should generate.",
    "Setting 'Sample Rate' tells the Core how many audio samples per second it should generate.",
    "audio_settings",
    {
        { "30000" },
        { "20000" },
    }
}, {
    "test_analog_mouse",
    "Input > Left Analog as mouse",
    "Left Analog as mouse",
    "Enabling 'Input > Left Analog as mouse' turns the left analog stick into a virtual mouse.",
    "Enabling 'Left Analog as mouse' turns the left analog stick into a virtual mouse.",
    "input_settings",
    {
        { "true" },
        { "false" },
    }
}, {
    "test_analog_mouse_relative",
    "Input > Analog mouse is relative",
    "Analog mouse is relative",
    "Disabling 'Input > Analog mouse is relative' makes the virtual mouse an absolute pointer device.",
    "Disabling 'Analog mouse is relative' makes the virtual mouse an absolute pointer device.",
    "input_settings",
    {
        { "true" },
        { "false" },
    }
}, {
    "test_audio_enable",
    "Audio > Enable Audio",
    "Enable Audio",
    "'Audio > Enable Audio' determines whether to generate sound.",
    "'Enable Audio' determines whether to generate sound.",
    "audio_settings",
    {
        { "true" },
        { "false" },
    }
})]
struct TestCore {
    aspect: f32,
    sample_rate: f64,
    analog_mouse: bool,
    analog_mouse_relative: bool,
    audio_enable: bool,

    last_aspect: f32,
    last_samplerate: f64,

    phase: u32,

    x_coord: u16,
    y_coord: u16,
    mouse_rel_x: i16,
    mouse_rel_y: i16,

    old_start: [bool; PORTS],
    old_strength_strong: [u16; PORTS],
    old_select: [bool; PORTS],
    old_strength_weak: [u16; PORTS],
}

retro_core!(TestCore {
    aspect: 4.0 / 3.0,
    sample_rate: 30000.0,
    analog_mouse: true,
    analog_mouse_relative: true,
    audio_enable: true,

    last_aspect: 0.0,
    last_samplerate: 0.0,

    phase: 0,

    x_coord: 0,
    y_coord: 0,
    mouse_rel_x: WIDTH as i16 / 2,
    mouse_rel_y: HEIGHT as i16 / 2,

    old_start: [false; PORTS],
    old_strength_strong: [0; PORTS],
    old_select: [false; PORTS],
    old_strength_weak: [0; PORTS],
});

impl TestCore {
    fn get_av_info(&mut self) -> retro_system_av_info {
        self.last_samplerate = self.sample_rate;
        self.last_aspect = self.aspect;

        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: WIDTH,
                base_height: HEIGHT,
                max_width: WIDTH,
                max_height: HEIGHT,
                aspect_ratio: self.aspect,
            },
            timing: retro_system_timing {
                fps: 60.0,
                sample_rate: self.sample_rate,
            },
        }
    }

    fn update_input(&mut self, ctx: &mut RunContext) {
        let gctx: GenericContext = ctx.into();

        if ctx.get_input_state(
            0,
            RETRO_DEVICE_KEYBOARD,
            0,
            retro_key::RETROK_RETURN.0 as u32,
        ) != 0
        {
            log::info!("Return key is pressed!")
        }
        if ctx.get_input_state(0, RETRO_DEVICE_KEYBOARD, 0, retro_key::RETROK_x.0 as u32) != 0 {
            log::info!("x key is pressed!")
        }

        let mut dir_x: i16 = 0;
        let mut dir_y: i16 = 0;

        if self.analog_mouse && !self.analog_mouse_relative {
            self.mouse_rel_x = 0;
            self.mouse_rel_y = 0;
        }

        for port in 0..2 {
            if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP) != 0 {
                dir_y -= 1;
            }
            if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN) != 0 {
                dir_y += 1;
            }
            if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT) != 0 {
                dir_x -= 1;
            }
            if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT) != 0
            {
                dir_x += 1;
            }

            let mouse_l =
                ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_LEFT) != 0;
            let mouse_r =
                ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_RIGHT) != 0;
            let mouse_down =
                ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_WHEELDOWN)
                    != 0;
            let mouse_up =
                ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_WHEELUP)
                    != 0;
            let mouse_middle =
                ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_MIDDLE) != 0;

            let mouse_x;
            let mouse_y;

            if self.analog_mouse {
                let lx = ctx.get_input_state(
                    port,
                    RETRO_DEVICE_ANALOG,
                    RETRO_DEVICE_INDEX_ANALOG_LEFT,
                    RETRO_DEVICE_ID_ANALOG_X,
                ) as f32
                    / 32767.0;
                let ly = ctx.get_input_state(
                    port,
                    RETRO_DEVICE_ANALOG,
                    RETRO_DEVICE_INDEX_ANALOG_LEFT,
                    RETRO_DEVICE_ID_ANALOG_Y,
                ) as f32
                    / 32767.0;

                if self.analog_mouse_relative {
                    mouse_x = ((WIDTH as f32 * lx) / 32.0) as i16;
                    mouse_y = ((HEIGHT as f32 * ly) / 32.0) as i16;
                } else {
                    mouse_x = ((WIDTH as f32 * lx) / 2.0 + (WIDTH as f32 / 2.0)) as i16;
                    mouse_y = ((HEIGHT as f32 * ly) / 2.0 + (HEIGHT as f32 / 2.0)) as i16;
                }
            } else {
                mouse_x = ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_X);
                mouse_y = ctx.get_input_state(port, RETRO_DEVICE_MOUSE, 0, RETRO_DEVICE_ID_MOUSE_Y);
            }

            if mouse_l {
                log::info!("Mouse #: {port}     L pressed.   X: {mouse_x}   Y: {mouse_y}",);
            }
            if mouse_r {
                log::info!("Mouse #: {port}     R pressed.   X: {mouse_x}   Y: {mouse_y}",);
            }
            if mouse_down {
                log::info!("Mouse #: {port}     wheeldown pressed.   X: {mouse_x}   Y: {mouse_y}",);
            }
            if mouse_up {
                log::info!("Mouse #: {port}     wheelup pressed.     X: {mouse_x}   Y: {mouse_y}",);
            }
            if mouse_middle {
                log::info!("Mouse #: {port}     middle pressed.      X: {mouse_x}   Y: {mouse_y}",);
            }

            if !self.analog_mouse || self.analog_mouse_relative {
                self.mouse_rel_x += mouse_x;
                self.mouse_rel_y += mouse_y;
            } else {
                let x = mouse_x as f32 / WIDTH as f32;
                let y = mouse_y as f32 / HEIGHT as f32;

                if !(0.45..=0.55).contains(&x) {
                    self.mouse_rel_x = mouse_x;
                }

                if !(0.45..=0.55).contains(&y) {
                    self.mouse_rel_y = mouse_y;
                }
            }

            if self.mouse_rel_x >= WIDTH as i16 - 5 {
                self.mouse_rel_x = WIDTH as i16 - 5;
            } else if self.mouse_rel_x < 5 {
                self.mouse_rel_x = 5;
            }

            if self.mouse_rel_y >= HEIGHT as i16 - 5 {
                self.mouse_rel_y = HEIGHT as i16 - 5;
            } else if self.mouse_rel_y < 5 {
                self.mouse_rel_y = 5;
            }

            let pointer_pressed = ctx.get_input_state(
                port,
                RETRO_DEVICE_POINTER,
                0,
                RETRO_DEVICE_ID_POINTER_PRESSED,
            ) != 0;
            if pointer_pressed {
                let x =
                    ctx.get_input_state(port, RETRO_DEVICE_POINTER, 0, RETRO_DEVICE_ID_POINTER_X);
                let y =
                    ctx.get_input_state(port, RETRO_DEVICE_POINTER, 0, RETRO_DEVICE_ID_POINTER_Y);

                log::info!("Pointer Pressed #: {port}    : ({x:6}, {y:6}).");
            }

            if !self.analog_mouse {
                dir_x += ctx.get_input_state(
                    port,
                    RETRO_DEVICE_ANALOG,
                    RETRO_DEVICE_INDEX_ANALOG_LEFT,
                    RETRO_DEVICE_ID_ANALOG_X,
                ) / 5000;
                dir_y += ctx.get_input_state(
                    port,
                    RETRO_DEVICE_ANALOG,
                    RETRO_DEVICE_INDEX_ANALOG_LEFT,
                    RETRO_DEVICE_ID_ANALOG_Y,
                ) / 5000;
            }

            dir_x += ctx.get_input_state(
                port,
                RETRO_DEVICE_ANALOG,
                RETRO_DEVICE_INDEX_ANALOG_RIGHT,
                RETRO_DEVICE_ID_ANALOG_X,
            ) / 5000;
            dir_y += ctx.get_input_state(
                port,
                RETRO_DEVICE_ANALOG,
                RETRO_DEVICE_INDEX_ANALOG_RIGHT,
                RETRO_DEVICE_ID_ANALOG_Y,
            ) / 5000;

            self.x_coord = ((self.x_coord as i16 + dir_x) & 31) as u16;
            self.y_coord = ((self.y_coord as i16 + dir_y) & 31) as u16;

            let strength_strong =
                if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_R2) > 0
                {
                    0x4000
                } else {
                    0xFFFF
                };

            let strength_weak =
                if ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_L2) > 0
                {
                    0x4000
                } else {
                    0xFFFF
                };

            let start =
                ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_START)
                    != 0;
            let select =
                ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_SELECT)
                    != 0;

            if self.old_start[port as usize] != start
                || self.old_strength_strong[port as usize] != strength_strong
            {
                log::info!(
                    "Port #: {port}   Strong rumble: {} ({strength_strong:04X}).",
                    if start { "ON" } else { "OFF" }
                );
                gctx.set_rumble_state(
                    port,
                    retro_rumble_effect::RETRO_RUMBLE_STRONG,
                    if start { strength_strong } else { 0 },
                );
                self.old_start[port as usize] = start;
                self.old_strength_strong[port as usize] = strength_strong;
            }

            if self.old_select[port as usize] != select
                || self.old_strength_weak[port as usize] != strength_weak
            {
                log::info!(
                    "Port #: {port}   Weak rumble: {} ({strength_weak:04X}).",
                    if select { "ON" } else { "OFF" }
                );
                gctx.set_rumble_state(
                    port,
                    retro_rumble_effect::RETRO_RUMBLE_WEAK,
                    if select { strength_weak } else { 0 },
                );
                self.old_select[port as usize] = select;
                self.old_strength_weak[port as usize] = strength_weak;
            }

            let trigger_pressed = ctx.get_input_state(
                port,
                RETRO_DEVICE_LIGHTGUN,
                0,
                RETRO_DEVICE_ID_LIGHTGUN_TRIGGER,
            ) != 0;
            if trigger_pressed {
                let x = ctx.get_input_state(
                    port,
                    RETRO_DEVICE_LIGHTGUN,
                    0,
                    RETRO_DEVICE_ID_LIGHTGUN_SCREEN_X,
                );
                let y = ctx.get_input_state(
                    port,
                    RETRO_DEVICE_LIGHTGUN,
                    0,
                    RETRO_DEVICE_ID_LIGHTGUN_SCREEN_Y,
                );

                log::info!("Lightgun Trigger Pressed #: {port}    : ({x}, {y}).");
            }
        }
    }

    fn render(&mut self, ctx: &mut RunContext) {
        // try to get a software framebuffer from the frontend
        let fb = unsafe {
            ctx.get_current_framebuffer_or_fallback(
                WIDTH,
                HEIGHT,
                MemoryAccess::WRITE,
                PixelFormat::XRGB8888,
            )
        };
        let data = unsafe { fb.as_slice_mut() };

        for y in 0..HEIGHT {
            let y_index = ((y as i32 - self.y_coord as i32) >> 4) & 1;

            for x in 0..WIDTH {
                let x_index = ((x as i32 - self.x_coord as i32) >> 4) & 1;

                let index = (y as usize * fb.pitch) + x as usize * 4;

                if y_index ^ x_index > 0 {
                    data[index] = 0;
                    data[index + 1] = 0;
                    data[index + 2] = 0;
                } else {
                    data[index] = 0xFF;
                    data[index + 1] = 0xFF;
                    data[index + 2] = 0xFF;
                };
                data[index + 3] = 0xFF;
            }
        }

        for y in self.mouse_rel_y - 5..self.mouse_rel_y + 5 {
            for x in self.mouse_rel_x - 5..self.mouse_rel_x + 5 {
                let index = y as isize * fb.pitch as isize + x as isize * 4;
                if index < 0 || index as usize >= data.len() {
                    continue;
                }

                data[index as usize] = 0x00;
                data[index as usize + 1] = 0x00;
                data[index as usize + 2] = 0xFF;
                data[index as usize + 3] = 0xFF;
            }
        }

        let width = fb.width;
        let height = fb.height;
        let pitch = fb.pitch as u64;
        ctx.draw_frame(data, width, height, pitch);
    }

    fn set_sub_system_info(&self, ctx: &mut SetEnvironmentContext) {
        let mem1 = [
            retro_subsystem_memory_info {
                extension: b"ram1\0".as_ptr() as *const c_char,
                type_: 0x400,
            },
            retro_subsystem_memory_info {
                extension: b"ram2\0".as_ptr() as *const c_char,
                type_: 0x401,
            },
        ];

        let mem2 = [
            retro_subsystem_memory_info {
                extension: b"ram3\0".as_ptr() as *const c_char,
                type_: 0x402,
            },
            retro_subsystem_memory_info {
                extension: b"ram4\0".as_ptr() as *const c_char,
                type_: 0x403,
            },
        ];

        let content = [
            retro_subsystem_rom_info {
                desc: b"Test Rom #1\0".as_ptr() as *const c_char,
                valid_extensions: b"bin\0".as_ptr() as *const c_char,
                need_fullpath: false,
                block_extract: false,
                required: true,
                memory: mem1.as_ptr(),
                num_memory: 2,
            },
            retro_subsystem_rom_info {
                desc: b"Test Rom #2\0".as_ptr() as *const c_char,
                valid_extensions: b"bin\0".as_ptr() as *const c_char,
                need_fullpath: false,
                block_extract: false,
                required: true,
                memory: mem2.as_ptr(),
                num_memory: 2,
            },
        ];

        ctx.set_subsystem_info(&[
            retro_subsystem_info {
                desc: b"Foo\0".as_ptr() as *const c_char,
                ident: b"foo\0".as_ptr() as *const c_char,
                roms: content.as_ptr(),
                num_roms: 2,
                id: 0x200,
            },
            retro_subsystem_info {
                desc: std::ptr::null(),
                ident: std::ptr::null(),
                roms: std::ptr::null(),
                num_roms: 0,
                id: 0,
            },
        ]);
    }

    fn set_controller_info(&self, ctx: &mut SetEnvironmentContext) {
        const DUMMY1: u32 = RETRO_DEVICE_SUBCLASS!(RETRO_DEVICE_ANALOG, 0);
        const DUMMY2: u32 = RETRO_DEVICE_SUBCLASS!(RETRO_DEVICE_ANALOG, 1);
        const RETRO_GUN: u32 = RETRO_DEVICE_LIGHTGUN;
        const AUGMENTED: u32 = RETRO_DEVICE_JOYPAD;

        const CONTROLLERS: [retro_controller_description; 4] = [
            retro_controller_description {
                desc: b"Dummy Controller #1\0".as_ptr() as *const c_char,
                id: DUMMY1,
            },
            retro_controller_description {
                desc: b"Dummy Controller #2\0".as_ptr() as *const c_char,
                id: DUMMY2,
            },
            retro_controller_description {
                desc: b"Lightgun\0".as_ptr() as *const c_char,
                id: RETRO_GUN,
            },
            retro_controller_description {
                desc: b"Augmented Joypad\0".as_ptr() as *const c_char,
                id: AUGMENTED,
            },
        ];

        const PORTS: [retro_controller_info; 3] = [
            retro_controller_info {
                types: CONTROLLERS.as_ptr(),
                num_types: 4,
            },
            retro_controller_info {
                types: CONTROLLERS.as_ptr(),
                num_types: 4,
            },
            retro_controller_info {
                types: std::ptr::null(),
                num_types: 0,
            },
        ];

        let gctx: GenericContext = ctx.into();
        gctx.set_controller_info(&PORTS);
    }
}

impl Core for TestCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("TestCore").unwrap(),
            library_version: CString::new("0.1.0").unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_init(&mut self, ctx: &mut InitContext) {
        // TODO: Write a neater abstraction / macro
        const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &[
            retro_input_descriptor {
                port: 0,
                device: RETRO_DEVICE_JOYPAD,
                index: 0,
                id: RETRO_DEVICE_ID_JOYPAD_UP,
                description: b"Up\0".as_ptr() as *const _,
            },
            retro_input_descriptor {
                port: 0,
                device: RETRO_DEVICE_JOYPAD,
                index: 0,
                id: RETRO_DEVICE_ID_JOYPAD_DOWN,
                description: b"Down\0".as_ptr() as *const _,
            },
            retro_input_descriptor {
                port: 0,
                device: RETRO_DEVICE_JOYPAD,
                index: 0,
                id: RETRO_DEVICE_ID_JOYPAD_LEFT,
                description: b"Left\0".as_ptr() as *const _,
            },
            retro_input_descriptor {
                port: 0,
                device: RETRO_DEVICE_JOYPAD,
                index: 0,
                id: RETRO_DEVICE_ID_JOYPAD_RIGHT,
                description: b"Right\0".as_ptr() as *const _,
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

        let gctx: GenericContext = ctx.into();
        gctx.set_input_descriptors(INPUT_DESCRIPTORS);
    }

    fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
        if !initial {
            return;
        }

        ctx.set_support_no_game(true);

        self.set_sub_system_info(ctx);
        self.set_controller_info(ctx);
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        self.get_av_info()
    }

    fn on_load_game(
        &mut self,
        _info: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !ctx.set_pixel_format(PixelFormat::XRGB8888) {
            return Err("XRGB8888 is not supported".into());
        }

        ctx.set_performance_level(0);
        ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t);

        match ctx.enable_rumble_interface() {
            Ok(_) => log::info!("Rumble is supported"),
            Err(_) => log::info!("Rumble is unsupported"),
        }

        let gctx: GenericContext = ctx.into();
        gctx.enable_audio_callback();
        gctx.enable_keyboard_callback();

        Ok(())
    }

    fn on_load_game_special(
        &mut self,
        game_type: std::os::raw::c_uint,
        _info: *const retro_game_info,
        num_info: size_t,
        ctx: &mut LoadGameSpecialContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Loading special content!");

        if game_type != 0x200 {
            return Err(format!("Unknown game type: 0x{game_type:03X}").into());
        }

        if num_info != 2 {
            return Err(format!("Invalid number of info objects: {num_info}").into());
        }

        self.on_load_game(None, &mut ctx.into())
    }

    fn on_unload_game(&mut self, _ctx: &mut UnloadGameContext) {
        self.last_aspect = 0.0;
        self.last_samplerate = 0.0;
    }

    fn on_reset(&mut self, _ctx: &mut ResetContext) {
        self.x_coord = 0;
        self.y_coord = 0;
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("test_aspect") {
            Some("4:3") => self.aspect = 4.0 / 3.0,
            Some("16:9") => self.aspect = 16.0 / 9.0,
            _ => (),
        }

        if let Some(value) = ctx.get_variable("test_samplerate") {
            self.sample_rate = value.parse().unwrap()
        }

        match ctx.get_variable("test_analog_mouse") {
            Some("true") => self.analog_mouse = true,
            Some("false") => self.analog_mouse = false,
            _ => (),
        }

        match ctx.get_variable("test_analog_mouse_relative") {
            Some("true") => self.analog_mouse_relative = true,
            Some("false") => self.analog_mouse_relative = false,
            _ => (),
        }

        match ctx.get_variable("test_audio_enable") {
            Some("true") => self.audio_enable = true,
            Some("false") => self.audio_enable = false,
            _ => (),
        }
    }

    fn on_set_controller_port_device(&mut self, port: u32, device: u32, ctx: &mut GenericContext) {
        let mut descriptors: [retro_input_descriptor; 6 + 1] =
            unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        match device {
            RETRO_DEVICE_NONE => (),
            RETRO_DEVICE_LIGHTGUN => {
                descriptors[0].port = port;
                descriptors[0].device = RETRO_DEVICE_JOYPAD;
                descriptors[0].index = 0;
                descriptors[0].id = RETRO_DEVICE_ID_LIGHTGUN_TRIGGER;
                descriptors[0].description = b"Gun Trigger\0".as_ptr() as *const _;

                descriptors[1].port = port;
                descriptors[1].device = RETRO_DEVICE_JOYPAD;
                descriptors[1].index = 0;
                descriptors[1].id = RETRO_DEVICE_ID_LIGHTGUN_RELOAD;
                descriptors[1].description = b"Gun Reload\0".as_ptr() as *const _;

                descriptors[2].port = port;
                descriptors[2].device = RETRO_DEVICE_JOYPAD;
                descriptors[2].index = 0;
                descriptors[2].id = RETRO_DEVICE_ID_LIGHTGUN_START;
                descriptors[2].description = b"Gun Start\0".as_ptr() as *const _;

                descriptors[3].port = port;
                descriptors[3].device = RETRO_DEVICE_JOYPAD;
                descriptors[3].index = 0;
                descriptors[3].id = RETRO_DEVICE_ID_LIGHTGUN_SELECT;
                descriptors[3].description = b"Gun Select\0".as_ptr() as *const _;
            }
            RETRO_DEVICE_JOYPAD => {
                descriptors[0].port = port;
                descriptors[0].device = RETRO_DEVICE_JOYPAD;
                descriptors[0].index = 0;
                descriptors[0].id = RETRO_DEVICE_ID_JOYPAD_UP;
                descriptors[0].description = b"Up\0".as_ptr() as *const _;

                descriptors[1].port = port;
                descriptors[1].device = RETRO_DEVICE_JOYPAD;
                descriptors[1].index = 0;
                descriptors[1].id = RETRO_DEVICE_ID_JOYPAD_DOWN;
                descriptors[1].description = b"Down\0".as_ptr() as *const _;

                descriptors[2].port = port;
                descriptors[2].device = RETRO_DEVICE_JOYPAD;
                descriptors[2].index = 0;
                descriptors[2].id = RETRO_DEVICE_ID_JOYPAD_LEFT;
                descriptors[2].description = b"Left\0".as_ptr() as *const _;

                descriptors[3].port = port;
                descriptors[3].device = RETRO_DEVICE_JOYPAD;
                descriptors[3].index = 0;
                descriptors[3].id = RETRO_DEVICE_ID_JOYPAD_RIGHT;
                descriptors[3].description = b"Right\0".as_ptr() as *const _;
            }
            _ => {
                descriptors[0].port = port;
                descriptors[0].device = RETRO_DEVICE_JOYPAD;
                descriptors[0].index = 0;
                descriptors[0].id = RETRO_DEVICE_ID_JOYPAD_UP;
                descriptors[0].description = b"Up\0".as_ptr() as *const _;

                descriptors[1].port = port;
                descriptors[1].device = RETRO_DEVICE_JOYPAD;
                descriptors[1].index = 0;
                descriptors[1].id = RETRO_DEVICE_ID_JOYPAD_DOWN;
                descriptors[1].description = b"Down\0".as_ptr() as *const _;

                descriptors[2].port = port;
                descriptors[2].device = RETRO_DEVICE_JOYPAD;
                descriptors[2].index = 0;
                descriptors[2].id = RETRO_DEVICE_ID_JOYPAD_LEFT;
                descriptors[2].description = b"Left\0".as_ptr() as *const _;

                descriptors[3].port = port;
                descriptors[3].device = RETRO_DEVICE_JOYPAD;
                descriptors[3].index = 0;
                descriptors[3].id = RETRO_DEVICE_ID_JOYPAD_RIGHT;
                descriptors[3].description = b"Right\0".as_ptr() as *const _;

                descriptors[4].port = port;
                descriptors[4].device = RETRO_DEVICE_JOYPAD;
                descriptors[4].index = 0;
                descriptors[4].id = RETRO_DEVICE_ID_JOYPAD_START;
                descriptors[4].description = b"Digital Start\0".as_ptr() as *const _;

                descriptors[5].port = port;
                descriptors[5].device = RETRO_DEVICE_JOYPAD;
                descriptors[5].index = 0;
                descriptors[5].id = RETRO_DEVICE_ID_JOYPAD_SELECT;
                descriptors[5].description = b"Digital Select\0".as_ptr() as *const _;
            }
        }

        ctx.set_input_descriptors(&descriptors);
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
        if self.last_samplerate != self.sample_rate {
            log::info!("Changing sample rate to {}", self.sample_rate);
            ctx.set_system_av_info(self.get_av_info());
        } else if self.last_aspect != self.aspect {
            log::info!("Changing aspect ratio to {}", self.aspect);
            ctx.set_game_geometry(self.get_av_info().geometry);
        }

        self.update_input(ctx);
        self.render(ctx);
    }

    fn on_write_audio(&mut self, ctx: &mut AudioContext) {
        if !self.audio_enable {
            return ctx.queue_audio_sample(0, 0);
        }

        let mut samples = Vec::with_capacity(self.sample_rate as usize * 2);
        let d = self.sample_rate as f32;

        for _ in 0..self.sample_rate as u64 / 60 {
            let value = ((0x800 as f32)
                * (2.0 * std::f32::consts::PI * (self.phase as f32) * 300.0 / d).sin())
                as i16;

            samples.push(value);
            samples.push(value);

            self.phase += 1;
        }

        self.phase %= 100;

        ctx.batch_audio_samples(&samples);
    }

    fn on_keyboard_event(
        &mut self,
        down: bool,
        keycode: retro_key,
        character: u32,
        key_modifiers: retro_mod,
    ) {
        log::info!("Keyboard:\n\tDown: {down}\n\tCode: {keycode:?}\n\tChar: {character}\n\tMod: {key_modifiers:?}");
    }

    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> size_t {
        (std::mem::size_of_val(&self.x_coord) + std::mem::size_of_val(&self.y_coord)) as size_t
    }

    fn on_serialize(&mut self, slice: &mut [u8], _ctx: &mut SerializeContext) -> bool {
        slice[0..std::mem::size_of_val(&self.x_coord)].copy_from_slice(&self.x_coord.to_le_bytes());
        slice[std::mem::size_of_val(&self.x_coord)..].copy_from_slice(&self.y_coord.to_le_bytes());

        true
    }

    fn on_unserialize(&mut self, slice: &mut [u8], _ctx: &mut UnserializeContext) -> bool {
        use byterepr::ByteRepr;

        self.x_coord = ByteRepr::from_le_bytes(&slice[0..std::mem::size_of_val(&self.x_coord)]);
        self.y_coord = ByteRepr::from_le_bytes(&slice[std::mem::size_of_val(&self.x_coord)..]);

        true
    }
}
