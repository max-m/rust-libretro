rust-libretro-sys
=================

Raw bindings to the libretro API (generated with bindgen).

[![libretro.h](https://img.shields.io/badge/libretro.h-b89692c-informational?logo=RetroArch)](https://github.com/libretro/RetroArch/blob/b89692c/libretro-common/include/libretro.h)

[![Build status](https://img.shields.io/github/actions/workflow/status/max-m/rust-libretro/main.yaml?branch=master)](https://github.com/max-m/rust-libretro/actions)
[![Latest version](https://img.shields.io/crates/v/rust-libretro-sys.svg)](https://crates.io/crates/rust-libretro-sys)
[![Documentation](https://docs.rs/rust-libretro-sys/badge.svg)](https://docs.rs/rust-libretro-sys)
![License](https://img.shields.io/crates/l/rust-libretro-sys.svg)

Vulkan Support
==============

This package provides optional support for the Vulkan bindings
provided in [`libretro_vulkan.h`](https://github.com/libretro/RetroArch/blob/master/libretro-common/include/libretro_vulkan.h).
Vulkan supported is gated behind the `vulkan` feature.
