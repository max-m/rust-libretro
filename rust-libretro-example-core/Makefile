release:
	cargo build --release

debug:
	cargo build

native:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	strip ../target/release/rust-libretro-example-core.so

clean:
	cargo clean

.PHONY: debug release native clean
