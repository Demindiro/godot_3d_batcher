GODOT?=godot
TARGET_LINUX?=x86_64-unknown-linux-gnu
TARGET_OSX?=x86_64-apple-darwin
TARGET_WINDOWS?=x86_64-pc-windows-gnu
CARGO_FLAGS?=--quiet


default: build-release


build-release: build-release-windows build-release-osx build-release-linux


build-release-windows: lib/
	cargo build $(CARGO_FLAGS) --release --target $(TARGET_WINDOWS)
	cp target/$(TARGET_WINDOWS)/release/gd_3d_batcher.dll lib/gd_3d_batcher.dll
	strip lib/gd_3d_batcher.dll


build-release-osx: lib/
	cargo build $(CARGO_FLAGS) --release --target $(TARGET_OSX)
	cp target/$(TARGET_OSX)/release/libgd_3d_batcher.dylib lib/libgd_3d_batcher.dylib
	#strip lib/libgd_3d_batcher.dylib


build-release-linux: lib/
	cargo build $(CARGO_FLAGS) --release --target $(TARGET_LINUX)
	cp target/$(TARGET_LINUX)/release/libgd_3d_batcher.so lib/libgd_3d_batcher.so
	strip lib/libgd_3d_batcher.so


debug: lib/
	cargo build
	mkdir -p lib/
	cp target/debug/*gd_3d_batcher.* lib/


lib/:
	mkdir -p $@


clean:
	cargo clean
