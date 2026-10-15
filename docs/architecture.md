# Kernel and userspace boundaries

The selective runtime port keeps the device registry and driver abstractions from
`main`. It does not require the Orbis shell, Rust std migration, or SMP bootstrap.

* `interrupts/interrupt_frame.rs` describes the saved i386 machine state.
* `interrupts/syscall/register.rs` is the sole adapter from registers to typed
  syscall arguments. EAX selects the call; EBX, ECX, EDX, ESI, EDI, EBP carry its
  arguments. Results are returned in EAX, with failures encoded as negative errno.
* `interrupts/syscall/*` validates ABI values and copies user memory before
  invoking the relevant subsystem. Subsystems must not fetch syscall arguments
  from task stacks or depend on the interrupt-frame layout.
* `schedule` owns process and task lifetime, runnable state, and wait queues.
* `fs` owns filesystem and descriptor operations; device implementations stay in
  `device`, with devfs discovering them through the device-node registry.
* `net` owns protocols and socket state. Drivers exchange Ethernet frames through
  `NetworkDevice`; user-address translation belongs to the syscall boundary.
* `programs/stdlib` owns C calling conventions, errno translation and userspace
  conveniences. The assembly syscall entry preserves cdecl callee-saved registers.

The ABI migration requires rebuilding all userspace executables together with the
kernel. Socket operations use socketcall argument blocks. Private network syscalls
are replaced by standard sockets and ioctls on the registered `/dev/net0` device.

## Threads

A process owns its address space, descriptors and signal dispositions. A task owns
its registers, scheduling state and optional `clear_child_tid` address. Process and
thread IDs use one allocator, so the initial thread has TID == PID without ID
collisions. `schedule/thread.rs` implements exit policy; the task manager owns task
creation, removal, and futex wait queues. `exec` removes siblings and restores the
process leader identity. Group exit removes every task before freeing process
memory. Exiting only the leader leaves surviving threads runnable.

Libc `pthread.c` owns join/detach state, stacks and thread-specific values. A stack
is reclaimed only after the kernel clears its completion word on thread exit.
Detached stacks are collected by the next create/detach operation, bounded by the
64-entry registry. The shared heap is locked, errno uses per-thread storage, and
fork snapshots the heap and thread registry under locks before resetting the
child's registry to the calling thread. TLS destructors run outside registry locks.

This port targets the existing single-CPU scheduler. It implements shared-address-
space clone with PARENT_SETTID and CHILD_CLEARTID, plus private futex wait/wake.
Compiler TLS setup, nonzero stack guards and timed futex waits are rejected rather
than silently accepted. Thread-specific keys and destructors are supported.

The thread tests also exposed prerequisites fixed here: partial ELF reads, the
segment-register inline-assembly clobber, and preserving user state across kernel
interrupts and page-fault frames containing an error code.

## Networking

`net/socket.rs` owns socket binding, ephemeral ports, peer filtering, UDP delivery
and payload transmission. The stack retains the `NetworkDevice` driver interface.
`device/network_control.rs` exposes configuration and DHCP through `/dev/net0`.
`memory/user.rs` validates mapped user pages and handles user copies for syscall
and ioctl adapters. Device code does not decode syscall registers.

Libc separates socket wrappers (`network.c`), configuration and compatibility
helpers (`network_control.c`), and DNS encoding/parsing (`resolver.c`). DNS uses
connected UDP sockets; receive deadlines are a userspace convenience. UDP is
limited to unfragmented Ethernet-sized packets; TCP is not implemented. Emulator
tests verify DHCP, ICMP and actual UDP payload round trips against a local server.

## I/O and local IPC

`fs/descriptor.rs` owns open-file operations, reference counts, readiness and local
socket endpoints. The process module owns the descriptor table and per-descriptor
close-on-exec bits. File status flags are shared across dup/fork. Local stream
socket pairs use two pipes, with the same blocking, EOF, wakeup and SIGPIPE rules.
Writes up to the pipe capacity are atomic.

`interrupts/syscall/ipc.rs` validates pipe2, socketpair, writev and poll arguments.
Poll sleeps the calling task between readiness scans; console readiness does not
consume input. Libc implements select over poll. Vectored writes gather at most
64 KiB per call and may return a short write. Local send/recv support flags=0;
unsupported flags are rejected.
