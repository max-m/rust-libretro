use image::{DynamicImage, ImageFormat, ImageResult};
use rust_libretro::{
    contexts::*,
    core::{Core, CoreOptions},
    env_version, retro_core,
    sys::*,
    types::*,
};
use std::ffi::CString;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 400;

const PASSIVE_COLOR: Option<(u8, u8, u8)> = None;
const ACTIVE_COLOR: Option<(u8, u8, u8)> = Some((0, 255, 0));

struct Images {
    body: DynamicImage,
    bumper_l: DynamicImage,
    bumper_r: DynamicImage,
    button: DynamicImage,
    dpad_down: DynamicImage,
    dpad_left: DynamicImage,
    dpad_right: DynamicImage,
    dpad_up: DynamicImage,
    home: DynamicImage,
    joystick: DynamicImage,
    start: DynamicImage,
    trigger_l: DynamicImage,
    trigger_r: DynamicImage,
}

struct InputTestCore {
    images: Option<Images>,
}

retro_core!(InputTestCore { images: None });

impl CoreOptions for InputTestCore {}
impl Core for InputTestCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("InputTestCore").unwrap(),
            library_version: CString::new(env_version!("CARGO_PKG_VERSION").to_string()).unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

    fn on_set_environment(&mut self, ctx: &mut SetEnvironmentContext) {
        // This function gets called multiple times by RetroArch,
        // but the supplied environment callback supports different sets of
        // functions, so let's ignore errors here.
        let _ = ctx.set_support_no_game(true);
    }

    fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
        retro_system_av_info {
            geometry: retro_game_geometry {
                base_width: WIDTH,
                base_height: HEIGHT,
                max_width: WIDTH,
                max_height: HEIGHT,
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
        use image::imageops::{flip_horizontal, flip_vertical, rotate90};
        use DynamicImage::ImageRgba8;

        ctx.set_pixel_format(PixelFormat::XRGB8888).map_err(|_| {
            rust_libretro::anyhow::anyhow!("Required pixel format “XRGB8888” is not supported")
        })?;

        let _ = ctx.set_performance_level(0);

        fn load(buf: &[u8]) -> ImageResult<DynamicImage> {
            image::load_from_memory_with_format(buf, ImageFormat::Png)
        }

        let body = load(include_bytes!("img/body.png"))?;
        let bumper_l = load(include_bytes!("img/bumper.png"))?;
        let button = load(include_bytes!("img/button.png"))?;
        let dpad_up = load(include_bytes!("img/dpad.png"))?;
        let home = load(include_bytes!("img/home.png"))?;
        let joystick = load(include_bytes!("img/joystick.png"))?;
        let start = load(include_bytes!("img/start.png"))?;
        let trigger_l = load(include_bytes!("img/trigger.png"))?;

        let bumper_r = ImageRgba8(flip_horizontal(&bumper_l));
        let trigger_r = ImageRgba8(flip_horizontal(&trigger_l));
        let dpad_down = ImageRgba8(flip_vertical(&dpad_up));
        let dpad_right = ImageRgba8(rotate90(&dpad_up));
        let dpad_left = ImageRgba8(flip_horizontal(&dpad_right));

        self.images = Some(Images {
            body,
            bumper_l,
            bumper_r,
            button,
            dpad_down,
            dpad_left,
            dpad_right,
            dpad_up,
            home,
            joystick,
            start,
            trigger_l,
            trigger_r,
        });

        Ok(())
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
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

        Self::fill(data, 0x62, 0x62, 0x62, 0xFF);

        let lx = ctx.get_input_state(
            0,
            RETRO_DEVICE_ANALOG,
            RETRO_DEVICE_INDEX_ANALOG_LEFT,
            RETRO_DEVICE_ID_ANALOG_X,
        ) as f32
            / 32767.0;
        let ly = ctx.get_input_state(
            0,
            RETRO_DEVICE_ANALOG,
            RETRO_DEVICE_INDEX_ANALOG_LEFT,
            RETRO_DEVICE_ID_ANALOG_Y,
        ) as f32
            / 32767.0;
        let rx = ctx.get_input_state(
            0,
            RETRO_DEVICE_ANALOG,
            RETRO_DEVICE_INDEX_ANALOG_RIGHT,
            RETRO_DEVICE_ID_ANALOG_X,
        ) as f32
            / 32767.0;
        let ry = ctx.get_input_state(
            0,
            RETRO_DEVICE_ANALOG,
            RETRO_DEVICE_INDEX_ANALOG_RIGHT,
            RETRO_DEVICE_ID_ANALOG_Y,
        ) as f32
            / 32767.0;

        let input = unsafe { ctx.get_joypad_bitmask(0, 0) };
        self.draw_controller(&fb, input, (lx, ly), (rx, ry));

        let width = fb.width;
        let height = fb.height;
        let pitch = fb.pitch;
        ctx.draw_frame(data, width, height, pitch);
    }
}

impl InputTestCore {
    fn fill(dst: &mut [u8], r: u8, g: u8, b: u8, a: u8) {
        // This should not fail, but just to be safe
        if let Ok(data) = bytemuck::try_cast_slice_mut(dst) {
            let value = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
            data.fill(value);
        } else {
            for chunk in dst.chunks_exact_mut(4) {
                chunk[0] = b;
                chunk[1] = g;
                chunk[2] = r;
                chunk[3] = a;
            }
        }
    }

    fn blend(dst: &mut [u8], src: &image::Rgba<u8>) {
        let f_r = src[0] as f32 / 255.0;
        let f_g = src[1] as f32 / 255.0;
        let f_b = src[2] as f32 / 255.0;
        let f_a = src[3] as f32 / 255.0;

        dst[2] = (((f_r * f_a) + ((dst[2] as f32 / 255.0) * (1.0 - f_a))) * 255.0) as u8;
        dst[1] = (((f_g * f_a) + ((dst[1] as f32 / 255.0) * (1.0 - f_a))) * 255.0) as u8;
        dst[0] = (((f_b * f_a) + ((dst[0] as f32 / 255.0) * (1.0 - f_a))) * 255.0) as u8;
    }

    fn soft_light(a: u8, b: u8) -> u8 {
        let a = a as f32 / 255.0;
        let b = b as f32 / 255.0;

        fn g(a: f32) -> f32 {
            if a <= 0.25 {
                ((16.0 * a - 12.0) * a + 4.0) * a
            } else {
                a.sqrt()
            }
        }

        if b <= 0.5 {
            ((a - (1.0 - 2.0 * b) * a * (1.0 - a)) * 255.0) as u8
        } else {
            ((a + (2.0 * b - 1.0) * g(a) - a) * 255.0) as u8
        }
    }

    fn blit(
        dst: &Framebuffer,
        src: &DynamicImage,
        x_offset: u32,
        y_offset: u32,
        color: Option<(u8, u8, u8)>,
    ) {
        use image::GenericImageView;

        let data = unsafe { std::slice::from_raw_parts_mut(dst.data, dst.data_len) };

        let bpp = dst.pitch / dst.width as usize;

        for y in y_offset..(src.height() + y_offset).min(HEIGHT) {
            for x in x_offset..(src.width() + x_offset).min(WIDTH) {
                let i = y as usize * dst.pitch + x as usize * bpp;

                let top = if let Some(color) = color {
                    let mut top = src.get_pixel(x - x_offset, y - y_offset);
                    if top[3] != 0 {
                        top[0] = Self::soft_light(top[0], color.0);
                        top[1] = Self::soft_light(top[1], color.1);
                        top[2] = Self::soft_light(top[2], color.2);
                    }
                    top
                } else {
                    src.get_pixel(x - x_offset, y - y_offset)
                };

                Self::blend(&mut data[i..i + 3], &top);
            }
        }
    }

    fn draw_controller(
        &self,
        fb: &Framebuffer,
        input: JoypadState,
        analog_l: (f32, f32),
        analog_r: (f32, f32),
    ) {
        let body = &self.images.as_ref().unwrap().body;
        let body_x = (WIDTH - body.width()) / 2;
        let body_y = (HEIGHT - body.height()) / 2;

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().trigger_l,
            body_x + 46,
            body_y - 5,
            if input.contains(JoypadState::L2) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().bumper_l,
            body_x + 56,
            body_y - 1,
            if input.contains(JoypadState::L) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().trigger_r,
            body_x + 277,
            body_y - 5,
            if input.contains(JoypadState::R2) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().bumper_r,
            body_x + 283,
            body_y - 1,
            if input.contains(JoypadState::R) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(fb, body, body_x, body_y, None);

        self.draw_dpad(fb, body_x, body_y, input);

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().start,
            body_x + 158,
            body_y + 75,
            if input.contains(JoypadState::SELECT) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().home,
            body_x + 196,
            body_y + 63,
            None,
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().start,
            body_x + 227,
            body_y + 75,
            if input.contains(JoypadState::START) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().joystick,
            body_x + (115.0 + 10.0 * analog_l.0) as u32,
            body_y + (122.0 + 10.0 * analog_l.1) as u32,
            if input.contains(JoypadState::L3) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().joystick,
            body_x + (247.0 + 10.0 * analog_r.0) as u32,
            body_y + (122.0 + 10.0 * analog_r.1) as u32,
            if input.contains(JoypadState::R3) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().button,
            body_x + 349,
            body_y + 67,
            if input.contains(JoypadState::A) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().button,
            body_x + 321,
            body_y + 95,
            if input.contains(JoypadState::B) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().button,
            body_x + 321,
            body_y + 39,
            if input.contains(JoypadState::X) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );

        Self::blit(
            fb,
            &self.images.as_ref().unwrap().button,
            body_x + 293,
            body_y + 67,
            if input.contains(JoypadState::Y) {
                ACTIVE_COLOR
            } else {
                PASSIVE_COLOR
            },
        );
    }

    fn draw_dpad(&self, fb: &Framebuffer, body_x: u32, body_y: u32, input: JoypadState) {
        for active in [false, true] {
            let color = if active { ACTIVE_COLOR } else { None };

            if !active || input.contains(JoypadState::UP) {
                let up = &self.images.as_ref().unwrap().dpad_up;
                Self::blit(fb, up, body_x + 83, body_y + 53, color);
            }

            if !active || input.contains(JoypadState::DOWN) {
                let down = &self.images.as_ref().unwrap().dpad_down;
                Self::blit(fb, down, body_x + 83, body_y + 82, color);
            }

            if !active || input.contains(JoypadState::LEFT) {
                let left = &self.images.as_ref().unwrap().dpad_left;
                Self::blit(fb, left, body_x + 61, body_y + 75, color);
            }

            if !active || input.contains(JoypadState::RIGHT) {
                let right = &self.images.as_ref().unwrap().dpad_right;
                Self::blit(fb, right, body_x + 90, body_y + 75, color);
            }
        }
    }
}
