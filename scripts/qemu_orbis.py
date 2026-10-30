#!/usr/bin/env python3
"""Boot real std applications and exercise the vendored Orbis shell in QEMU."""
import os
from pathlib import Path
import re
import select
import shutil
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]

def main():
    output = bytearray()
    with tempfile.TemporaryDirectory(prefix="polyos-orbis-") as directory:
        monitor_path = str(Path(directory) / "monitor.sock")
        disk = Path(directory) / "os.bin"
        shutil.copyfile(ROOT / "bin/os.bin", disk)
        process = subprocess.Popen([
            shutil.which("qemu-system-i386") or "qemu-system-x86_64",
            "-snapshot", "-drive", f"format=raw,file={disk}", "-serial", "stdio",
            "-display", "none", "-monitor", f"unix:{monitor_path},server=on,wait=off",
        ], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        monitor = None
        def wait(predicate, start=0, timeout=30):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                data = bytes(output[start:])
                if b"KERNEL PANIC" in data:
                    raise AssertionError("guest kernel panic")
                if predicate(data):
                    return data
                if process.poll() is not None:
                    break
                if select.select([process.stdout], [], [], .05)[0]:
                    chunk = os.read(process.stdout.fileno(), 65536)
                    if not chunk:
                        break
                    output.extend(chunk)
            raise AssertionError("guest did not reach expected state: " + repr(bytes(output[-1000:])))
        def send(data):
            process.stdin.write(data)
            process.stdin.flush()
        def monitor_reply():
            data = bytearray()
            while b"(qemu) " not in data:
                chunk = monitor.recv(65536)
                if not chunk:
                    raise AssertionError("monitor disconnected")
                data.extend(chunk)
            return bytes(data)
        def command(text):
            monitor.sendall(text.encode() + b"\n")
            return monitor_reply()
        def prompt(start):
            return wait(lambda s: b"\x1b[?2004h" in s and b"orbis" in s and b"$" in s
                        and re.search(rb"\r\x1b\[[0-9]+C", s) is not None, start)
        def line(text, expected=None):
            start = len(output)
            send(text.encode() + b"\n")
            data = prompt(start)
            if expected is not None:
                assert expected in data, repr(data[-1000:])
            return data
        try:
            wait(lambda s: b"PolyOS v2.0.0" in s and b"> " in s)
            start = len(output)
            send(b"/bin/std-smoke.elf\n")
            wait(lambda s: b"rust-std: all checks passed" in s and b"Process exited with status 0" in s and b"> " in s, start)
            print("[ok] upstream std: filesystem, hashing, guarded threads, timed sync, spawn/exec", flush=True)
            start = len(output)
            send(b"/bin/orbis.elf --version\n")
            wait(lambda s: b"orbis 0.1.0" in s and b"Process exited with status 0" in s and b"> " in s, start)
            print("[ok] vendored Orbis version command", flush=True)
            start = len(output)
            send(b"/bin/orbis.elf\n")
            prompt(start)
            monitor = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            monitor.settimeout(5)
            monitor.connect(monitor_path)
            monitor_reply()
            start = len(output)
            send(b"echo editX")
            wait(lambda s: b"editX" in s, start)
            command("sendkey backspace")
            deadline = time.monotonic() + 3
            while True:
                dump = command("xp /5400hx 0xb8000")
                words = re.findall(rb"0x[0-9a-fA-F]{4}(?![0-9a-fA-F])", dump)
                cells = bytes(int(word, 16) & 255 for word in words)
                if b"orbis:/$ echo edit " in cells or time.monotonic() >= deadline:
                    break
                time.sleep(.02)
            assert b"orbis:/$ echo edit " in cells, "VGA ANSI editing left stale characters"
            send(b"\n")
            data = prompt(start)
            assert re.search(rb"\r?\nedit\r?\n", data), repr(data[-500:])
            print("[ok] Rustyline PS/2 Backspace and VGA ANSI redraw", flush=True)
            line("orbisbox echo PIPE_OK > /tmp/orbis-output")
            line("cat /tmp/orbis-output", b"\nPIPE_OK\r\n")
            line("cat /tmp/orbis-output | wc -c", b"       8\r\n")
            print("[ok] Orbis redirection, utility fallback, pipes and terminal handoff", flush=True)
            data = line("ps -e")
            assert b"/bin/shell-v2.elf" in data and b"/bin/orbis.elf" in data, repr(data[-1000:])
            print("[ok] Orbisbox ps reads numeric procfs views", flush=True)
            start = len(output)
            send(b"/bin/selftest.elf --shell-job-child\n")
            wait(lambda s: b"shell-job-ready" in s, start)
            start = len(output)
            send(b"\x1a")
            data = prompt(start)
            assert b"Stopped" in data
            line("jobs", b"Stopped")
            start = len(output)
            send(b"fg\n")
            wait(lambda s: b"\x1b[?2004l\r\n" in s, start)
            time.sleep(.2)
            send(b"done\n")
            data = prompt(start)
            assert b"shell-job-done" in data
            print("[ok] Orbis Ctrl-Z/jobs/fg stop and resume a foreground command", flush=True)
            start = len(output)
            send(b"\x03")
            prompt(start)
            start = len(output)
            send(b"exit\n")
            wait(lambda s: b"Process exited with status 0" in s and b"> " in s, start)
            print("[ok] Orbis Ctrl-C and clean exit restore the parent shell", flush=True)
        finally:
            if monitor:
                monitor.close()
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            (ROOT / "log").mkdir(exist_ok=True)
            (ROOT / "log/orbis_review.txt").write_bytes(output)

if __name__ == "__main__":
    main()
