#!/bin/sh
set -eu
runtime_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec i686-elf-gcc -nostdlib -static -T "$runtime_dir/linker.ld" \
    "$runtime_dir/lib/runtime.o" "$runtime_dir/lib/start.o" -L"$runtime_dir/lib" "$@"
