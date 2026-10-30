all:
	python3 ../../scripts/build_rust_std.py --manifest-path ../../vendor/orbis/Cargo.toml
	cp ../../vendor/orbis/target/i686-polyos-std/release/orbis orbis.elf
	cp ../../vendor/orbis/target/i686-polyos-std/release/orbisbox ../../file/bin/orbisbox.elf

clean:
	rm -f orbis.elf ../../file/bin/orbisbox.elf
