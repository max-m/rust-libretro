rust-libretro-example-core
==========================

Very simple example core using the [rust-libretro](../rust-libretro) API abstractions.

This crate demonstrates how a minimal setup could look like.

[![Build status](https://img.shields.io/github/workflow/status/max-m/rust-libretro/CI/master)](https://github.com/max-m/rust-libretro/actions)
[![Latest version](https://img.shields.io/crates/v/rust-libretro-example-core.svg)](https://crates.io/crates/rust-libretro-example-core)
[![Documentation](https://docs.rs/rust-libretro-example-core/badge.svg)](https://docs.rs/rust-libretro-example-core)
![License](https://img.shields.io/crates/l/rust-libretro-example-core.svg)

How to run:
-----------

Simply run `cargo build` or `make debug` to compile a debug build.
The produced shared library will follow the standard naming scheme of Rust, so Linux builds for example will be saved in `../target/debug/librust_libretro_example_core.so`.

Release builds can be compiled with `cargo build --release` or `make release`. As usual, the produced library will reside in `../target/debug/`.
The [Makefile](Makefile) also provides a `native` target that instructs `rustc` to optimize the produced code for your host CPU only.

One easy way to test your compiled core is to use RetroArch’s CLI: `retroarch -L <path to your built library>`

Note for Windows users:
-----------------------

In an initial test it seemed like only release builds were working.
When loading a debug build RetroArch closed immediately.
