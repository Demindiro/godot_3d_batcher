default:
	cargo build --release
	mkdir -p lib/
	cp target/release/libgd_3d_batcher.so lib/libgd_3d_batcher.so
	strip lib/libgd_3d_batcher.so


debug:
	cargo build
	mkdir -p lib/
	cp target/debug/libgd_3d_batcher.so lib/libgd_3d_batcher.so
	strip lib/libgd_3d_batcher.so
