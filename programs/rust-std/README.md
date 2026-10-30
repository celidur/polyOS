# Static Rust std profile for PolyOS

This adapter builds upstream Rust std without editing the Rust source tree. The
custom target uses its Linux/GNU Unix backend, with vendor `polyos`, static i386
ELF, SSE2, panic=abort and pthread-key TLS. It is a constrained compatibility ABI,
not full Linux/glibc or a native upstream PolyOS target.

The tested compiler is `rustc 1.101.0-nightly (12433c307 2026-10-24)` with rust-src.
The regular kernel's existing nightly/toolchain requirements still apply. Install
rust-src for that nightly and provide the project's i686-elf GCC/binutils and NASM.
Fetch the locked Cargo dependencies once (network required):

```
cargo +nightly fetch -Zjson-target-spec --manifest-path vendor/orbis/Cargo.toml --target programs/rust-std/i686-polyos-std.json
cargo +nightly fetch -Zjson-target-spec --manifest-path programs/std-smoke/Cargo.toml --target programs/rust-std/i686-polyos-std.json
make all
python3 scripts/qemu_orbis.py
```

`build_rust_std.py` builds the native libc, the adapter and upstream std.
Set `POLYOS_CARGO_OFFLINE=1` after dependencies (including build-std dependencies)
are cached to disable Cargo network access.
It selects the absolute linker wrapper; linker inputs are tracked by Cargo build
scripts. Orbis and std-smoke are optional separate programs. `make all` installs
`/bin/orbis.elf`, `/bin/orbisbox.elf` and `/bin/std-smoke.elf` in the disk image.

The adapter owns GNU sigaction/sigset/termios layouts, directory records, 64-bit
file offsets with checked 32-bit bounds, vectored reads, pthread defaults and
startup stack alignment. It translates through the existing libc and syscall
boundary; it does not duplicate scheduler or VFS policy. Link-role archives
satisfy upstream Unix std native-library grouping; they do not provide complete
libm, libdl, libpthread or glibc implementations.

Validated surfaces include allocation and HashMap seeding, file creation/read/
write/unlink and directory traversal, guarded threads, timed mutex/condvar waits,
Command fork/exec success and missing-program errors, terminal input, pipelines
and foreground job control. See std-smoke and qemu_orbis.py for executable checks.

Current limits:

* Static linking only, no dynamic symbol lookup or compiler TLS. Panic aborts.
* 32-bit file sizes/offsets and the kernel's existing descriptor/process limits.
* statvfs, hard links, symlink creation, rename, utimensat and ancillary
  sendmsg/recvmsg return ENOTSUP. sendfile/splice return ENOSYS for std fallbacks.
* getpwuid_r reports no account entry. realpath follows the VFS's supported final
  links and lexical normalization; this is not general symlink traversal support.
* Secure entropy requires RDRAND. Hash seeding may explicitly use GRND_INSECURE.
* No TCP, pseudo-terminals, SMP, dynamic libraries, or general Linux syscall set.
* Existing signal, filesystem metadata and terminal limits remain in effect.

Unsupported Orbis utilities may consequently return errors. Vendoring Orbis does
not imply that every utility or option is implemented by the OS.
