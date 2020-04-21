run:
	cargo build --release --quiet
	./target/release/nes -3 super_mario.nes
