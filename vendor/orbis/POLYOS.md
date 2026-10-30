# Orbis source snapshot for PolyOS

Source: the local `programs/shell/Orbis` snapshot supplied in this workspace,
package version 0.1.0. No upstream URL or revision was present in the supplied
snapshot; none is inferred. `POLYOS-SOURCE.sha256` records SHA-256 checksums of
that original snapshot, before the changes below. Git metadata, Finder files and
build outputs are excluded. The original source directory remains untouched.
The upstream MIT license is retained in LICENSE.

PolyOS integration changes:

* Cargo lockfile updated; release binaries use aborting panics, LTO and size
  optimization. Orbisbox statvfs arithmetic explicitly widens 32-bit counts.
* Build scripts track external runtime and linker inputs.
* A close-on-exec pipe barrier holds pipeline children until process groups and
  foreground terminal ownership are established. Children restore job-control
  signal defaults; the shell ignores SIGTTIN/SIGTTOU during terminal handoff.
* Foreground builtins apply redirections in the parent with buffered output
  flushed and descriptor state restored on success or error.
* Child setup failures exit with status 126, and failed exec exits with status 127. On PolyOS, known utility names
  fall back to `/bin/orbisbox.elf` without filesystem symlink aliases. Explicit
  `orbisbox <utility>` uses the same installed binary.

From the repository root run `make all`, then `/bin/orbis.elf` in the PolyOS
shell. See `programs/rust-std/README.md` for dependency preparation and ABI limits.
`python3 scripts/qemu_orbis.py` checks std startup, physical Backspace/VGA redraw,
redirection, pipelines, procfs ps, stop/resume and exit.

The source is vendored; Cargo registry dependencies are locked but not copied
into this directory. The build uses the repository helper's nightly compiler,
not this snapshot's upstream rust-toolchain.toml. Utilities requiring unsupported
filesystem or networking facilities return errors; see the runtime documentation.
