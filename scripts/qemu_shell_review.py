#!/usr/bin/env python3
"""Exercise the interactive shell through serial input and the PS/2 keyboard."""
import os
import re
import select
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def main():
    output = bytearray()
    monitor = None
    with tempfile.TemporaryDirectory(prefix="polyos-shell-") as temporary:
        path = str(Path(temporary) / "monitor.sock")
        command = [shutil.which("qemu-system-i386") or "qemu-system-x86_64",
                   "-snapshot", "-drive", "format=raw,file=./bin/os.bin",
                   "-serial", "stdio", "-display", "none",
                   "-monitor", f"unix:{path},server=on,wait=off"]
        process = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        def read_until(predicate, timeout=15):
            end = time.monotonic() + timeout
            while time.monotonic() < end:
                if predicate(bytes(output)):
                    return
                if process.poll() is not None:
                    break
                readable, _, _ = select.select([process.stdout], [], [], .05)
                if readable:
                    data = os.read(process.stdout.fileno(), 4096)
                    if not data:
                        break
                    output.extend(data)
            raise AssertionError("shell output did not reach expected state: " + repr(bytes(output[-300:])))
        def send(data):
            process.stdin.write(data)
            process.stdin.flush()
        def monitor_reply():
            data = bytearray()
            while b"(qemu) " not in data:
                chunk = monitor.recv(65536)
                if not chunk:
                    raise AssertionError("QEMU monitor disconnected")
                data.extend(chunk)
            return bytes(data)
        def monitor_command(command):
            monitor.sendall(command.encode() + b"\n")
            return monitor_reply()
        try:
            read_until(lambda s: b"PolyOS v2.0.0" in s and b"> " in s, 30)
            monitor = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            monitor.settimeout(5)
            monitor.connect(path)
            monitor_reply()
            # The terminal transitions to character mode after printing the prompt.
            send(b"echo ax")
            read_until(lambda s: b"echo ax" in s)
            start = len(output)
            monitor_command("sendkey backspace")
            read_until(lambda s: b"\x08 \x08" in s[start:] or b"\x7f" in s[start:])
            assert b"\x08 \x08" in output[start:], "PS/2 Backspace did not visually erase the character"
            # Serial output precedes the VGA write; wait for the redraw too.
            deadline = time.monotonic() + 2
            while True:
                screen = monitor_command("xp /2000hx 0xb8000")
                words = re.findall(rb"0x[0-9a-fA-F]{4}(?![0-9a-fA-F])", screen)
                cells = bytes(int(word, 16) & 255 for word in words)
                if b"echo a " in cells or time.monotonic() >= deadline:
                    break
                time.sleep(.02)
            (ROOT / "log/shell_review_vga.txt").write_bytes(cells)
            assert b"echo a " in cells, "VGA still displays the erased character"
            send(b"b\n")
            read_until(lambda s: re.search(rb"\r?\nab\r?\n> ", s[start:]) is not None)
            print("[ok] physical PS/2 Backspace edits the command and VGA screen")
            for key, label in [(b"\x08", "serial BS"), (b"\x7f", "serial DEL")]:
                start = len(output)
                send(b"echo ax" + key + b"b\n")
                read_until(lambda s: re.search(rb"\r?\nab\r?\n> ", s[start:]) is not None)
                assert b"\x08 \x08" in output[start:], label + " did not emit an erase sequence"
                print("[ok] " + label + " edits and redraws the command")
            for command, marker, label in [
                (b"/bin/selftest.elf --shell-job-child\n", b"shell-job-ready", "command"),
                (b"/bin/selftest.elf --shell-job-child | /bin/selftest.elf --shell-pipe-child\n",
                 b"shell-pipeline-ready", "pipeline"),
            ]:
                start = len(output)
                send(command)
                read_until(lambda s: marker in s[start:])
                start = len(output)
                send(b"\x1a")
                read_until(lambda s: b"] Stopped " in s[start:] and b"> " in s[start:])
                start = len(output)
                send(b"jobs\n")
                read_until(lambda s: b"] Stopped " in s[start:] and b"> " in s[start:])
                start = len(output)
                send(b"fg\n")
                read_until(lambda s: b"/bin/selftest.elf --shell-job-child" in s[start:])
                send(b"done\n")
                read_until(lambda s: b"shell-job-done" in s[start:] and b"> " in s[start:])
                print("[ok] Ctrl-Z, jobs, and fg restore " + label + " and terminal ownership")
            start = len(output)
            send(b"echo job-control-ok\n")
            read_until(lambda s: re.search(rb"\r?\njob-control-ok\r?\n> ", s[start:]) is not None)
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
            (ROOT / "log/shell_review.txt").write_bytes(output)

if __name__ == "__main__":
    main()
