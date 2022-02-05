release:
	cargo build --release

debug:
	cargo build

native:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	strip ../target/release/librust_libretro_example_core.so

clean:
	cargo clean

.PHONY: debug release native clean
