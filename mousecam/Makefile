all: target/release/rusttest ang/firmware


target/release/rusttest: src/main.rs Cargo.toml
	cargo rustc --release -- -C target-cpu=native

ang/firmware: ang/canon_buffman.a51 ang/huff_table.a51 ang/loader.a51 ang/main.a51
	cd ang && make firmware

run: all
	sudo ./target/release/rusttest

