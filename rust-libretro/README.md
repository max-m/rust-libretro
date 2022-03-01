rust-libretro
=============

A Rust library providing abstractions over the [libretro API](../rust-libretro-sys).

[![Build status](https://img.shields.io/github/workflow/status/max-m/rust-libretro/CI/master)](https://github.com/max-m/rust-libretro/actions)
[![Latest version](https://img.shields.io/crates/v/rust-libretro.svg)](https://crates.io/crates/rust-libretro)
[![Documentation](https://docs.rs/rust-libretro/badge.svg)](https://docs.rs/rust-libretro)
![License](https://img.shields.io/crates/l/rust-libretro.svg)

Many of the abstractions lack documentation right now.
PRs are welcome!

Examples
========

The following examples are available:
- input: A simple core that visualizes the input of the first joypad.
- test: A port of [libretro-samples/test](https://github.com/libretro/libretro-samples/tree/7418a585efd24c6506ca5f09f90c36268f0074ed/tests/test).

To build and run an example you can use the following commands:
```sh
# You might also want to add `--release`
cargo build --example <NAME> --features="unstable-env-commands log"

# adapt according to your target
retroarch -v -L ../target/debug/examples/lib<NAME>.so
```
