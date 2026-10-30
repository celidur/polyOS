#!/usr/bin/env python3
"""Build upstream Rust std for PolyOS using the isolated GNU/i386 ABI adapter."""
import os
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
runtime = ROOT / 'programs/rust-std'
subprocess.run(['make', '-C', str(ROOT / 'programs/stdlib')], check=True)
subprocess.run(['make', '-C', str(runtime)], check=True)
env = dict(os.environ, CARGO_TARGET_I686_POLYOS_STD_LINKER=str(runtime / 'linker.sh'))
subprocess.run(['cargo', '+nightly', 'build', '-Zjson-target-spec', '-Zbuild-std=std,panic_abort',
                '--target', str(runtime / 'i686-polyos-std.json'), '--release', '--locked', *(['--offline'] if os.environ.get('POLYOS_CARGO_OFFLINE') == '1' else []), *sys.argv[1:]],
               env=env, check=True)
