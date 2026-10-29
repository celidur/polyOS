#!/usr/bin/env python3
import argparse
import os
import re
import select
import signal
import shutil
import socket
import threading
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def run_checked(command):
    result = subprocess.run(command, cwd=ROOT)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def main():
    parser = argparse.ArgumentParser(description="Boot PolyOS in QEMU and run /bin/selftest.elf")
    parser.add_argument("--no-build", action="store_true", help="skip make all")
    parser.add_argument("--timeout", type=float, default=90.0, help="maximum seconds to wait")
    parser.add_argument("--network", action="store_true", help="test DHCP, ICMP and local UDP echo")
    args = parser.parse_args()

    if not args.no_build:
        run_checked(["make", "all"])

    log_dir = ROOT / "log"
    log_dir.mkdir(exist_ok=True)
    log_path = log_dir / time.strftime("selftest_%Y%m%d_%H%M%S.txt")

    command = [
        shutil.which("qemu-system-i386") or "qemu-system-x86_64",
        "-snapshot",
        "-drive",
        "format=raw,file=./bin/os.bin",
        "-netdev",
        "user,id=net0",
        "-device",
        "rtl8139,netdev=net0",
        "-serial",
        "stdio",
        "-display",
        "none",
    ]

    echo_socket = None
    if args.network:
        echo_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        echo_socket.bind(("127.0.0.1", 31337))
        def echo():
            while True:
                try:
                    data, peer = echo_socket.recvfrom(2048)
                    echo_socket.sendto(data, peer)
                except OSError:
                    return
        threading.Thread(target=echo, daemon=True).start()

    proc = subprocess.Popen(
        command,
        cwd=ROOT,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )

    output = bytearray()
    sent_selftest = False
    sent_terminal = set()
    passed = False
    failed = False
    deadline = time.monotonic() + args.timeout
    command_text = "/bin/selftest.elf"
    command_text += " net\n" if args.network else "\n"

    try:
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                break

            ready, _, _ = select.select([proc.stdout], [], [], 0.1)
            if not ready:
                continue

            chunk = os.read(proc.stdout.fileno(), 4096)
            if not chunk:
                break
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()
            output.extend(chunk)

            text = output.decode("utf-8", errors="replace")
            if not sent_selftest and "PolyOS v2.0.0" in text and "> " in text:
                proc.stdin.write(command_text.encode("ascii"))
                proc.stdin.flush()
                sent_selftest = True

            terminal_inputs = [
                ("tty-canonical-ready", b"ab\x7fc\r"),
                ("tty-eof-ready", b"\x04"),
                ("tty-raw-ready", b"\x00Z"),
                ("tty-nonblock-ready", b"N"),
                ("tty-signal-ready", b"\x03"),
                ("tty-background-ready", b"B"),
            ]
            for marker, payload in terminal_inputs:
                if marker in text and marker not in sent_terminal:
                    proc.stdin.write(payload)
                    proc.stdin.flush()
                    sent_terminal.add(marker)
            if "KERNEL PANIC" in text:
                failed = True
                break

            match = re.search(r"(?m)^selftest: passed=(\d+) failed=(\d+)", text)
            if match:
                passed = match.group(2) == "0"
                failed = not passed
                break
    finally:
        if echo_socket:
            echo_socket.close()
        log_path.write_bytes(output)
        if proc.poll() is None:
            try:
                if proc.stdin and not proc.stdin.closed:
                    proc.stdin.write(b"shutdown\n")
                    proc.stdin.flush()
                    time.sleep(0.5)
            except BrokenPipeError:
                pass
            if proc.poll() is None:
                proc.send_signal(signal.SIGTERM)
                try:
                    proc.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait()

    print(f"\nlog: {log_path}")
    if passed and not failed:
        return 0
    if not sent_selftest:
        print("selftest command was not sent before timeout", file=sys.stderr)
    elif not passed:
        print("selftest did not report failed=0", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
