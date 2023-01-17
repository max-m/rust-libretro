//! Port of <https://github.com/libretro/libretro-samples/tree/7418a585efd24c6506ca5f09f90c36268f0074ed/tests/test_advanced>
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

use bytemuck::Pod;
use dasp_sample::Sample;
use dasp_signal::{self as signal, ConstHz, IntoInterleavedSamples, ScaleAmp, Signal, Sine};
use num::Integer;
use num_traits::{cast::AsPrimitive, int::PrimInt};
use rust_libretro::{
    contexts::*, core::Core, env_version, proc::CoreOptions, retro_core, sys::*, types::*,
};
use serde::{Deserialize, Serialize, Serializer};
use std::{char, ffi::CString, fmt::Display};

const FRAMERATE: f64 = 60.0;
const SAMPLE_RATE: f64 = 30720.0;
const FREQUENCY: f64 = 440.0;
const AMPLITUDE: f64 = 0.1;

const NUM_GROUPS: u8 = 4;
const GROUP_SIZES: [u8; NUM_GROUPS as usize] = [6, 2, 1, 1];

const INIT_GRP: u8 = 1;
const INIT_SUB: char = 'a';

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

pub fn serialize_array<S, T>(array: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    array.serialize(serializer)
}

#[macro_export]
macro_rules! serde_array {
    ($m:ident, $n:expr) => {
        pub mod $m {
            use serde::{de, Deserialize, Deserializer};
            use std::{mem, ptr};
            pub use $crate::serialize_array as serialize;

            pub fn deserialize<'de, D, T>(deserializer: D) -> Result<[T; $n], D::Error>
            where
                D: Deserializer<'de>,
                T: Deserialize<'de> + 'de,
            {
                let slice: Vec<T> = Deserialize::deserialize(deserializer)?;
                if slice.len() != $n {
                    return Err(de::Error::custom("input slice has wrong length"));
                }
                unsafe {
                    let mut result: [T; $n] = mem::MaybeUninit::uninit().assume_init();
                    for (src, dst) in slice.into_iter().zip(&mut result[..]) {
                        ptr::write(dst, src);
                    }
                    Ok(result)
                }
            }
        }
    };
}

serde_array!(test4a, 28 * 3);

#[repr(C)]
#[derive(Serialize, Deserialize)]
struct State {
    test_group: u8,
    test_sub: char,
    can_change: bool,

    frame: u32,

    test3a_activate: u8,
    test3a_last: u64,

    #[serde(with = "test4a")]
    test4a: [u16; 28 * 3],
}

impl Default for State {
    fn default() -> Self {
        Self {
            test_group: INIT_GRP,
            test_sub: INIT_SUB,
            can_change: false,

            frame: 0,

            test3a_activate: 0,
            test3a_last: 0,
            test4a: [0; 28 * 3],
        }
    }
}

#[derive(CoreOptions)]
#[categories({
    "video_settings",
    "Video",
    "Options related to video output."
})]
#[options({
    "test_advanced_pixel_format",
    "Video > Pixel Format (needs hard reset)",
    "Pixel Format (needs hard reset)",
    "Setting 'Video > Pixel Format' instructs the core which framebuffer pixel format to use.",
    "Setting 'Pixel Format' instructs the core which framebuffer pixel format to use.",
    "video_settings",
    {
        { "0RGB1555" },
        { "XRGB8888" },
        { "RGB565" },
    }
})]
struct AdvancedTestCore {
    pixel_format: PixelFormat,
    active_pixel_format: PixelFormat,

    state: State,

    inp_state: [JoypadState; 2],
    sound_enable: bool,

    has_perf: bool,

    sine: IntoInterleavedSamples<ScaleAmp<Sine<ConstHz>>>,
}

impl Default for AdvancedTestCore {
    fn default() -> Self {
        Self {
            pixel_format: PixelFormat::XRGB8888,
            active_pixel_format: PixelFormat::XRGB8888,

            sine: signal::rate(SAMPLE_RATE)
                .const_hz(FREQUENCY)
                .sine()
                .scale_amp(AMPLITUDE)
                .into_interleaved_samples(),

            inp_state: [JoypadState::empty(); 2],
            sound_enable: false,

            has_perf: false,

            state: State::default(),
        }
    }
}

retro_core!(AdvancedTestCore::default());

struct Pixel;
impl Pixel {
    #[inline]
    pub fn rgb<T>(r: u8, g: u8, b: u8, format: PixelFormat) -> T
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        use PixelFormat::*;

        let r: u32 = r.as_();
        let g: u32 = g.as_();
        let b: u32 = b.as_();

        let value: T = match format {
            XRGB1555 => ((r & 0b11111) << 10) | ((g & 0b11111) << 5) | (b & 0b11111),
            XRGB8888 => (r << 16) | (g << 8) | b,
            RGB565 => ((r & 0b11111) << 11) | ((g & 0b111111) << 5) | (b & 0b11111),
            _ => r | g | b,
        }
        .as_();

        value
    }
}

macro_rules! impl_pixfmt {
    ($name:ident $(, $($opt:ident: $ty:ty),*)?) => {
        #[allow(clippy::too_many_arguments)]
        fn $name(&mut self, fb: &Framebuffer $(, $($opt: $ty),*)?) {
            use PixelFormat::*;

            ::paste::paste! {
                match fb.format {
                    XRGB8888 => self.[<$name _inner>]::<u32>(fb $(, $($opt),*)?),
                    XRGB1555 | RGB565 => {
                        self.[<$name _inner>]::<u16>(fb $(, $($opt),*)?)
                    }
                    _ => (),
                }
            }
        }
    };
}

impl AdvancedTestCore {
    impl_pixfmt!(
        render_character,
        r: u8,
        g: u8,
        b: u8,
        chr: char,
        x: u16,
        y: u16
    );
    #[allow(clippy::too_many_arguments)]
    fn render_character_inner<T>(
        &self,
        fb: &Framebuffer,
        r: u8,
        g: u8,
        b: u8,
        chr: char,
        x: u16,
        y: u16,
    ) where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        // Taken from ZSNES
        const Z_FONT: [u8; 390] = [
            0, 0, 0, 0, 0, 0x70, 0x98, 0xA8, 0xC8, 0x70, 0x20, 0x60, 0x20, 0x20, 0x70, 0x70, 0x88,
            0x30, 0x40, 0xF8, 0x70, 0x88, 0x30, 0x88, 0x70, 0x50, 0x90, 0xF8, 0x10, 0x10, 0xF8,
            0x80, 0xF0, 0x08, 0xF0, 0x70, 0x80, 0xF0, 0x88, 0x70, 0xF8, 0x08, 0x10, 0x10, 0x10,
            0x70, 0x88, 0x70, 0x88, 0x70, 0x70, 0x88, 0x78, 0x08, 0x70, 0x70, 0x88, 0xF8, 0x88,
            0x88, 0xF0, 0x88, 0xF0, 0x88, 0xF0, 0x70, 0x88, 0x80, 0x88, 0x70, 0xF0, 0x88, 0x88,
            0x88, 0xF0, 0xF8, 0x80, 0xF0, 0x80, 0xF8, 0xF8, 0x80, 0xF0, 0x80, 0x80, 0x78, 0x80,
            0x98, 0x88, 0x70, 0x88, 0x88, 0xF8, 0x88, 0x88, 0xF8, 0x20, 0x20, 0x20, 0xF8, 0x78,
            0x10, 0x10, 0x90, 0x60, 0x90, 0xA0, 0xE0, 0x90, 0x88, 0x80, 0x80, 0x80, 0x80, 0xF8,
            0xD8, 0xA8, 0xA8, 0xA8, 0x88, 0xC8, 0xA8, 0xA8, 0xA8, 0x98, 0x70, 0x88, 0x88, 0x88,
            0x70, 0xF0, 0x88, 0xF0, 0x80, 0x80, 0x70, 0x88, 0xA8, 0x90, 0x68, 0xF0, 0x88, 0xF0,
            0x90, 0x88, 0x78, 0x80, 0x70, 0x08, 0xF0, 0xF8, 0x20, 0x20, 0x20, 0x20, 0x88, 0x88,
            0x88, 0x88, 0x70, 0x88, 0x88, 0x50, 0x50, 0x20, 0x88, 0xA8, 0xA8, 0xA8, 0x50, 0x88,
            0x50, 0x20, 0x50, 0x88, 0x88, 0x50, 0x20, 0x20, 0x20, 0xF8, 0x10, 0x20, 0x40, 0xF8,
            0x00, 0x00, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x68, 0x90, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x08, 0x10, 0x20, 0x40, 0x80, 0x10, 0x20, 0x40,
            0x20, 0x10, 0x40, 0x20, 0x10, 0x20, 0x40, 0x70, 0x40, 0x40, 0x40, 0x70, 0x70, 0x10,
            0x10, 0x10, 0x70, 0x00, 0x20, 0x00, 0x20, 0x00, 0x60, 0x98, 0x70, 0x98, 0x68, 0x20,
            0x20, 0xA8, 0x70, 0x20, 0x50, 0xF8, 0x50, 0xF8, 0x50, 0x00, 0xF8, 0x00, 0xF8, 0x00,
            0x48, 0x90, 0x00, 0x00, 0x00, 0x80, 0x40, 0x20, 0x10, 0x08, 0xA8, 0x70, 0xF8, 0x70,
            0xA8, 0x70, 0x88, 0x30, 0x00, 0x20, 0x88, 0x10, 0x20, 0x40, 0x88, 0x20, 0x20, 0xF8,
            0x20, 0x20, 0x00, 0x00, 0x00, 0x20, 0x40, 0x30, 0x40, 0x40, 0x40, 0x30, 0x60, 0x10,
            0x10, 0x10, 0x60, 0x70, 0x98, 0xB8, 0x80, 0x70, 0x20, 0x40, 0x00, 0x00, 0x00, 0x20,
            0x20, 0x20, 0x00, 0x20, 0x78, 0xA0, 0x70, 0x28, 0xF0, 0x00, 0x20, 0x00, 0x20, 0x40,
            0x40, 0x20, 0x00, 0x00, 0x00, 0x20, 0x50, 0x00, 0x00, 0x00, 0x30, 0x40, 0xC0, 0x40,
            0x30, 0x60, 0x10, 0x18, 0x10, 0x60, 0x20, 0x20, 0x70, 0x70, 0xF8, 0xF8, 0x70, 0x70,
            0x20, 0x20, 0x08, 0x38, 0xF8, 0x38, 0x08, 0x80, 0xE0, 0xF8, 0xE0, 0x80, 0x20, 0x60,
            0xF8, 0x60, 0x20, 0x38, 0x20, 0x30, 0x08, 0xB0, 0xFC, 0x84, 0xFC, 0x00, 0x00, 0x00,
            0xFC, 0x00, 0x00, 0x00, 0xF8, 0x88, 0x88, 0x88, 0xF8,
        ];

        const CONV_TABLE: [u8; 256] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x33, 0x31, 0x3F, 0x37, 0x2F, 0x3D, 0x3A, 0x3B,
            0x35, 0x38, 0x39, 0x25, 0x28, 0x29, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x2E, 0x40, 0x2A, 0x32, 0x2B, 0x36, 0x3C, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
            0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x2C, 0x34, 0x2D, 0x42, 0x26, 0x41, 0x0B,
            0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19,
            0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x43, 0x00, 0x44,
            0x27, 0x00, 0x0D, 0x1F, 0x0F, 0x0B, 0x0B, 0x0B, 0x0B, 0x0D, 0x0F, 0x0F, 0x0F, 0x13,
            0x13, 0x13, 0x0B, 0x0B, 0x0F, 0x0B, 0x0B, 0x19, 0x19, 0x19, 0x1F, 0x1F, 0x23, 0x19,
            0x1F, 0x0D, 0x10, 0x23, 0x1A, 0x10, 0x0B, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54,
            0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F, 0x60, 0x61, 0x62,
            0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F, 0x70,
            0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E,
            0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4D, 0x4C, 0x4B, 0x4A, 0x45,
            0x46, 0x47, 0x48, 0x49,
        ];

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let color = Pixel::rgb(r, g, b, fb.format);

        for iy in 0..5 {
            for ix in 0..8 {
                if (Z_FONT[CONV_TABLE[chr as usize] as usize * 5 + iy] >> ix) & 1 == 1 {
                    let index = (y as usize + iy) * pitch + x as usize + (ix ^ 7);
                    data[index] = color;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_text(&mut self, fb: &Framebuffer, r: u8, g: u8, b: u8, text: &str, x: u16, y: u16) {
        // does not handle grapheme clusters!
        for (i, mut chr) in text.chars().enumerate() {
            if !chr.is_ascii() {
                chr = ' ';
            }

            self.render_character(fb, r, g, b, chr, x + i as u16 * 8, y);
        }
    }

    fn render_outlined_text(
        &mut self,
        fb: &Framebuffer,
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
        text: &str,
        x: u16,
        y: u16,
    ) {
        // Top
        self.render_text(fb, bg.0, bg.1, bg.2, text, x - 1, y - 1);
        self.render_text(fb, bg.0, bg.1, bg.2, text, x, y - 1);
        self.render_text(fb, bg.0, bg.1, bg.2, text, x + 1, y - 1);
        // Middle
        self.render_text(fb, bg.0, bg.1, bg.2, text, x - 1, y);
        self.render_text(fb, bg.0, bg.1, bg.2, text, x + 1, y);
        // Bottom
        self.render_text(fb, bg.0, bg.1, bg.2, text, x - 1, y + 1);
        self.render_text(fb, bg.0, bg.1, bg.2, text, x, y + 1);
        self.render_text(fb, bg.0, bg.1, bg.2, text, x + 1, y + 1);
        // Foreground
        self.render_text(fb, fg.0, fg.1, fg.2, text, x, y);
    }

    impl_pixfmt!(test_1a);
    fn test_1a_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let red = Pixel::rgb(255, 0, 0, fb.format);
        let green = Pixel::rgb(0, 255, 0, fb.format);
        let blue = Pixel::rgb(0, 0, 255, fb.format);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = white;
            }
        }

        for y in HEIGHT / 3..(HEIGHT as f32 / 1.5) as u32 {
            for x in 0..WIDTH / 4 {
                let index_r = (y as usize * pitch) + x as usize + (WIDTH / 8) as usize;
                let index_g = (y as usize * pitch) + x as usize + (WIDTH as f32 / 2.6666) as usize;
                let index_b = (y as usize * pitch) + x as usize + (WIDTH as f32 / 1.6) as usize;

                data[index_r] = red;
                data[index_g] = green;
                data[index_b] = blue;
            }
        }
    }

    impl_pixfmt!(test_1b);
    fn test_1b_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let black = Pixel::rgb(0, 0, 0, fb.format);
        let mod_val = WIDTH as usize / 8;
        let cmp_val = WIDTH as usize / 16;

        for (x, pixel) in data.iter_mut().enumerate().take(WIDTH as usize) {
            if (x + self.state.frame as usize) % mod_val > cmp_val {
                *pixel = white;
            } else {
                *pixel = black;
            }
        }

        for y in 1..HEIGHT as usize {
            data.copy_within(0..pitch, y * pitch);
        }
    }

    impl_pixfmt!(test_1c);
    fn test_1c_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let black = Pixel::rgb(0, 0, 0, fb.format);
        let mod_val = HEIGHT as usize / 8;
        let cmp_val = HEIGHT as usize / 16;

        for y in 0..HEIGHT as usize {
            for x in 0..WIDTH as usize {
                let index = (y * pitch) + x;

                if ((HEIGHT as usize - y) + self.state.frame as usize) % mod_val > cmp_val {
                    data[index] = white;
                } else {
                    data[index] = black;
                }
            }
        }
    }

    impl_pixfmt!(test_1d);
    fn test_1d_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let color = if self.state.frame % 2 == 1 {
            Pixel::rgb(255, 255, 255, fb.format)
        } else {
            Pixel::rgb(0, 0, 0, fb.format)
        };

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = color;
            }
        }
    }

    impl_pixfmt!(test_1e);
    fn test_1e_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let black = Pixel::rgb(0, 0, 0, fb.format);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = if (x ^ y) & 1 == 1 { white } else { black };
            }
        }
    }

    impl_pixfmt!(test_1f);
    fn test_1f_inner<T>(&self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let red = Pixel::rgb(255, 0, 0, fb.format);
        let yellow = Pixel::rgb(255, 255, 0, fb.format);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = white;
            }
        }

        for x in 0..WIDTH {
            let top = x as usize;
            let bot = ((HEIGHT as usize - 1) * pitch) + x as usize;

            data[top] = if x & 1 == 1 { red } else { yellow };
            data[bot] = if x & 1 == 1 { yellow } else { red };
        }

        for y in 0..HEIGHT {
            let l = y as usize * pitch;
            let r = (y as usize * pitch) + WIDTH as usize - 1;

            data[l] = if y & 1 == 1 { red } else { yellow };
            data[r] = if y & 1 == 1 { yellow } else { red };
        }
    }

    impl_pixfmt!(test_2a);
    fn test_2a_inner<T>(&mut self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let mod_val = HEIGHT;
        let cmp_val = HEIGHT / 2;

        let white = Pixel::rgb(255, 255, 255, fb.format);
        let black = Pixel::rgb(0, 0, 0, fb.format);
        let color_a;
        let color_b;

        if self.state.frame % mod_val >= cmp_val {
            color_a = white;
            color_b = black;
            self.sound_enable = true;
        } else {
            color_a = black;
            color_b = white;
            self.sound_enable = false;
        }

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = color_a;
            }
        }

        let mod_val = HEIGHT as usize / 2;
        let index = pitch * (self.state.frame as usize % mod_val) * 2;

        data[index..index + 8].fill(color_b);
    }

    impl_pixfmt!(test_2b);
    fn test_2b_inner<T>(&mut self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let color;

        if self.inp_state[0].intersects(
            JoypadState::B
                | JoypadState::Y
                | JoypadState::SELECT
                | JoypadState::START
                | JoypadState::A
                | JoypadState::X
                | JoypadState::L
                | JoypadState::R,
        ) {
            color = Pixel::rgb(0, 0, 0, fb.format);
            self.sound_enable = true;
        } else {
            color = Pixel::rgb(255, 255, 255, fb.format);
            self.sound_enable = false;
        }

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = color;
            }
        }
    }

    impl_pixfmt!(test_3a, ctx: &RunContext);
    fn test_3a_inner<T>(&mut self, fb: &Framebuffer, ctx: &RunContext)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        let white = Pixel::rgb(255, 255, 255, fb.format);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = white;
            }
        }

        if self.has_perf {
            let gctx: GenericContext = ctx.into();

            if self.state.test3a_activate == 1 {
                let mut calls: u64 = 0;
                let mut iter_len = 32;
                let mut now;
                let start = match gctx.perf_get_time_usec() {
                    Ok(v) => v,
                    Err(err) => {
                        log::error!("perf_get_time_usec() failed: {err}");
                        return;
                    }
                };

                loop {
                    now = match gctx.perf_get_time_usec() {
                        Ok(v) => v,
                        Err(err) => {
                            log::error!("perf_get_time_usec() failed: {err}");
                            return;
                        }
                    };

                    if now < start + 1000 && iter_len < 0x10000000 {
                        iter_len *= 2;
                    }

                    if now > start + 2000000 {
                        break;
                    }

                    for i in 0..iter_len {
                        let port = (((now ^ i) >> 4) % 2) as u32;
                        let id = ((now ^ i) % 16) as u32;
                        ctx.get_input_state(port, RETRO_DEVICE_JOYPAD, 0, id);
                        calls = calls.wrapping_add(1);
                    }
                }

                let seconds = (now - start) as u64 / 1000000;
                self.state.test3a_last = calls / seconds;
                self.state.test3a_activate = 2;
            }

            if self.state.test3a_activate == 0
                && self.inp_state[0].intersects(
                    !(JoypadState::UP | JoypadState::DOWN | JoypadState::LEFT | JoypadState::RIGHT),
                )
            {
                self.state.test3a_activate = 1;
            }

            if self.state.test3a_activate == 2 && self.inp_state[0].is_empty() {
                self.state.test3a_activate = 0;
            }

            if self.state.test3a_activate == 1 {
                self.render_text(fb, 0, 0, 0, "Running ...", 8, 24);
            } else if self.state.test3a_last == 0 {
                self.render_text(fb, 0, 0, 0, "Ready", 8, 24);
            } else {
                let text = format!("{} calls per second", format_number(self.state.test3a_last));
                self.render_text(fb, 0, 0, 0, &text, 8, 24);

                let text = format!(
                    "{} calls per frame",
                    format_number(self.state.test3a_last / 60)
                );
                self.render_text(fb, 0, 0, 0, &text, 8, 40);
            }
        } else {
            self.render_text(fb, 0, 0, 0, "Performance Interface", 8, 24);
            self.render_text(fb, 0, 0, 0, "   not available", 8, 32);
        }
    }

    impl_pixfmt!(test_4a);
    fn test_4a_inner<T>(&mut self, fb: &Framebuffer)
    where
        T: PrimInt,
        T: AsPrimitive<u32>,
        T: Pod,
        u32: AsPrimitive<T>,
    {
        if fb.data.is_null() {
            return;
        }

        let data = unsafe { fb.as_slice_mut() };
        let data: &mut [T] = bytemuck::cast_slice_mut(data);
        let pitch = fb.pitch / fb.format.bit_per_pixel();

        if self.inp_state[0].bits() != self.state.test4a[27 * 3 + 1]
            || self.inp_state[1].bits() != self.state.test4a[27 * 3 + 2]
        {
            for i in 0..27 * 3 {
                self.state.test4a[i] = self.state.test4a[i + 3];
            }

            self.state.test4a[27 * 3] = (self.state.frame as u16).wrapping_add(1);
            self.state.test4a[27 * 3 + 1] = self.inp_state[0].bits();
            self.state.test4a[27 * 3 + 2] = self.inp_state[1].bits();
        }

        let crc = crc32(bytemuck::cast_slice(&self.state.test4a), !0);
        let color = !crc & 0x7F7F7F;
        let r = (color >> 16) as u8;
        let g = (color >> 8) as u8;
        let b = color as u8;
        let color = Pixel::rgb(r, g, b, fb.format);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let index = (y as usize * pitch) + x as usize;
                data[index] = color;
            }
        }

        for i in 0..28 {
            if self.state.test4a[i * 3] > 0 {
                let text = format!(
                    "{}: {:04X} {:04X}",
                    self.state.test4a[i * 3],
                    self.state.test4a[i * 3 + 1],
                    self.state.test4a[i * 3 + 2]
                );
                self.render_text(fb, 255, 255, 255, &text, 8, 8 + i as u16 * 8);
            }
        }
    }
}

impl Core for AdvancedTestCore {
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("AdvancedTestCore").unwrap(),
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
                fps: FRAMERATE,
                sample_rate: SAMPLE_RATE,
            },
        }
    }

    fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
        match ctx.get_variable("test_advanced_pixel_format") {
            Ok("0RGB1555") => self.pixel_format = PixelFormat::XRGB1555,
            Ok("XRGB8888") => self.pixel_format = PixelFormat::XRGB8888,
            Ok("RGB565") => self.pixel_format = PixelFormat::RGB565,
            _ => (),
        }
    }

    fn on_load_game(
        &mut self,
        _info: Option<retro_game_info>,
        ctx: &mut LoadGameContext,
    ) -> rust_libretro::core::Result<()> {
        self.active_pixel_format = self.pixel_format;
        ctx.set_pixel_format(self.active_pixel_format)
            .map_err(|_| {
                rust_libretro::anyhow::anyhow!(
                    "Required pixel format “{:?}” is not supported",
                    self.active_pixel_format
                )
            })?;

        let _ = ctx.set_performance_level(0);

        if let Err(err) =
            ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t)
        {
            log::error!("Failed to enable frame time callback: {}", err);
        }

        self.has_perf = ctx.enable_perf_interface().is_ok();

        let gctx: GenericContext = ctx.into();

        if let Err(err) = gctx.enable_audio_callback() {
            log::error!("Failed to enable audio callback: {}", err);
        }

        Ok(())
    }

    fn on_reset(&mut self, _ctx: &mut ResetContext) {
        core::mem::take(self);
    }

    #[inline]
    fn on_run(&mut self, ctx: &mut RunContext, _delta_us: Option<i64>) {
        // try to get a software framebuffer from the frontend
        let fb = unsafe {
            ctx.get_current_framebuffer_or_fallback(
                WIDTH,
                HEIGHT,
                MemoryAccess::WRITE,
                self.active_pixel_format,
            )
        };
        let data = unsafe { fb.as_slice_mut() };

        self.inp_state[0] = ctx.get_joypad_state(0, 0);
        self.inp_state[1] = ctx.get_joypad_state(1, 0);
        self.sound_enable = false;

        if self.state.can_change {
            let mut changed = false;

            if self.inp_state[0].intersects(JoypadState::UP) {
                self.state.test_group -= 1;
                if self.state.test_group == 0 {
                    self.state.test_group = NUM_GROUPS;
                }
                self.state.test_sub = 'a';
                changed = true;
            }

            if self.inp_state[0].intersects(JoypadState::DOWN) {
                self.state.test_group += 1;
                if self.state.test_group - 1 == NUM_GROUPS {
                    self.state.test_group = 1;
                }
                self.state.test_sub = 'a';
                changed = true;
            }

            if self.inp_state[0].intersects(JoypadState::LEFT)
                && GROUP_SIZES[self.state.test_group as usize - 1] != 1
            {
                self.state.test_sub = (self.state.test_sub as u8 - 1) as char;
                if self.state.test_sub == (b'a' - 1) as char {
                    self.state.test_sub =
                        (GROUP_SIZES[self.state.test_group as usize - 1] + b'a' - 1) as char;
                }
                changed = true;
            }

            if self.inp_state[0].intersects(JoypadState::RIGHT)
                && GROUP_SIZES[self.state.test_group as usize - 1] != 1
            {
                self.state.test_sub = (self.state.test_sub as u8 + 1) as char;
                if self.state.test_sub as u8 - 1
                    == GROUP_SIZES[self.state.test_group as usize - 1] + b'a' - 1
                {
                    self.state.test_sub = 'a';
                }
                changed = true;
            }

            if changed {
                self.state.frame = 0;
            }
        }
        self.state.can_change = !self.inp_state[0].intersects(
            JoypadState::UP | JoypadState::DOWN | JoypadState::LEFT | JoypadState::RIGHT,
        );

        match self.state.test_group {
            1 => match self.state.test_sub {
                'a' => self.test_1a(&fb),
                'b' => self.test_1b(&fb),
                'c' => self.test_1c(&fb),
                'd' => self.test_1d(&fb),
                'e' => self.test_1e(&fb),
                'f' => self.test_1f(&fb),
                _ => (),
            },
            2 => match self.state.test_sub {
                'a' => self.test_2a(&fb),
                'b' => self.test_2b(&fb),
                _ => (),
            },
            3 if self.state.test_sub == 'a' => self.test_3a(&fb, ctx),
            4 if self.state.test_sub == 'a' => self.test_4a(&fb),
            _ => (),
        }

        let test_id = format!(
            "{}{}",
            (self.state.test_group + b'0') as char,
            self.state.test_sub
        );
        self.render_outlined_text(&fb, (0, 0, 0), (255, 255, 255), &test_id, 8, 8);

        let text = match fb.format {
            PixelFormat::XRGB1555 => "0RGB1555",
            PixelFormat::XRGB8888 => "XRGB8888",
            PixelFormat::RGB565 => "RGB565",
            _ => "Unknown",
        };
        self.render_outlined_text(
            &fb,
            (0, 0, 0),
            (255, 255, 255),
            text,
            WIDTH as u16 - 8 - text.len() as u16 * 8,
            8,
        );

        if self.sound_enable {
            let text = "Sound Enabled";
            self.render_outlined_text(
                &fb,
                (0, 0, 0),
                (255, 255, 255),
                text,
                (WIDTH as u16 - text.len() as u16 * 8) / 2,
                8,
            );
        }

        self.state.frame = self.state.frame.wrapping_add(1);

        let width = fb.width;
        let height = fb.height;
        let pitch = fb.pitch;
        ctx.draw_frame(data, width, height, pitch);
    }

    fn on_write_audio(&mut self, ctx: &mut AudioContext) {
        if !self.sound_enable {
            let samples = vec![0; SAMPLE_RATE as usize * 2 / FRAMERATE as usize];
            return ctx.batch_audio_samples(&samples);
        }

        let sample_count = SAMPLE_RATE as usize / FRAMERATE as usize * 2;
        let mut samples = Vec::with_capacity(sample_count);

        for _ in 0..sample_count {
            samples.push(i16::from_sample(self.sine.next_sample()));
        }

        ctx.batch_audio_samples(&samples);
    }

    fn get_serialize_size(&mut self, _ctx: &mut GetSerializeSizeContext) -> usize {
        std::mem::size_of::<State>()
    }

    fn on_serialize(
        &mut self,
        slice: &mut [u8],
        _ctx: &mut SerializeContext,
    ) -> rust_libretro::core::Result<()> {
        use bincode::Options;

        bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .serialize_into(slice, &self.state)
            .map_err(Into::into)
    }

    fn on_unserialize(
        &mut self,
        slice: &mut [u8],
        _ctx: &mut UnserializeContext,
    ) -> rust_libretro::core::Result<()> {
        use bincode::Options;

        let state = bincode::DefaultOptions::new()
            .allow_trailing_bytes()
            .deserialize(slice)?;

        let _ = std::mem::replace(&mut self.state, state);
        Ok(())
    }
}

fn format_number<T: Integer + Display>(n: T) -> String {
    let n = n.to_string();
    let mut s = String::with_capacity(n.len() + n.len() / 3);

    for (i, chr) in n.chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            s.insert(0, ',');
        }

        s.insert(0, chr);
    }

    s
}

/// Karl Malbrain's compact CRC-32.
/// See "A compact CCITT crc16 and crc32 C implementation that balances processor cache usage against speed":
/// http://www.geocities.ws/malbrain/
fn crc32(data: &[u8], mut crc: u32) -> u32 {
    const LUT: [u32; 16] = [
        0x00000000, 0x1db71064, 0x3b6e20c8, 0x26d930ac, 0x76dc4190, 0x6b6b51f4, 0x4db26158,
        0x5005713c, 0xedb88320, 0xf00f9344, 0xd6d6a3e8, 0xcb61b38c, 0x9b64c2b0, 0x86d3d2d4,
        0xa00ae278, 0xbdbdf21c,
    ];

    for byte in data {
        crc = (crc >> 4) ^ LUT[(crc as usize & 15) ^ (*byte as usize & 15)];
        crc = (crc >> 4) ^ LUT[(crc as usize & 15) ^ ((*byte >> 4) as usize)];
    }

    crc
}
