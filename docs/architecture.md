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
Joining/detaching the initial thread, compiler TLS setup, nonzero stack guards
are rejected rather
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

## Filesystem APIs

`fs/path.rs` resolves cwd and directory-relative paths. `fs/open.rs` shares open
policy between open and openat, including permissions, exclusive creation,
close-on-exec and directory-only opens. A zero creation mode means no permissions;
libc fopen and shell creation paths supply their intended default mode explicitly.
`FileOps` owns descriptor chmod, truncation and position. Memfs preserves open
objects after unlink. FAT maintains a logical offset, zero-fills gaps/extensions,
and preserves the offset across truncate; FAT chmod remains unsupported.

`interrupts/syscall/filesystem.rs` marshals access, readlink, ftruncate, fchmod,
openat, fstatat, stat64 and getdents64. The 96-byte i386 stat64 and variable-length
dirent64 records have explicit APIs; the existing PolyOS stat/dirent ABI remains
available for existing programs. Inode/device IDs and timestamps are not yet backed
by filesystem metadata and are reported as zero. Directory entries are snapshots
at open time. The read-only procfs exposes `/proc/self/exe` through the VFS link
interface, with stat/open following the link and lstat/readlink preserving it.

## System inspection

`inspection.rs` supplies snapshots to sysinfo and read-only procfs files (uptime,
meminfo, stat and self/status). Memory totals describe the managed kernel/page
allocation pool, not a firmware RAM inventory. Swap and load averages are not
implemented and report zero. The scheduler currently reports one CPU. The loader
copies ELF headers and auxiliary-vector entries into user memory; libc implements
getauxval, sysconf and dl_iterate_phdr from these values. Proc files retain a stable
snapshot for the lifetime of an open descriptor.

## Virtual memory

`memory/address_space.rs` owns mmap pages, address selection, replacement and
permissions. The process retains mappings and copied pages across fork; exec drops
the old address space. mprotect validates the entire range before changing it,
including mappings temporarily marked PROT_NONE. Newly writable shared pages are
made copy-on-write even when they were read-only at fork.

Supported mappings are private anonymous pages and private file snapshots within
a dedicated user arena. File mappings preserve descriptor offsets, zero-fill the
last page and reject ranges beyond the file's final page. Shared mappings and
file writeback are unsupported. Non-PAE i386 cannot enforce execute-disable or
write-only pages; PROT_NONE and write protection are enforced. Mappings are limited
to 32 MiB per call and kernel allocation failures return ENOMEM.

## Memory allocation

Libc `allocation.c` owns the locked small-allocation free list, 16-byte alignment,
checked calloc/realloc sizes, aligned allocation and large mmap-backed blocks.
Failed realloc leaves the original block intact. Tail trimming verifies the actual
program break before releasing memory. brk/sbrk share a separate lock and query the
current kernel break; fork snapshots both locks and resets them in the child.
Kernel page allocation checks multiplication and alignment overflow before allocating.

## Synchronization

`schedule/ipc.rs` owns keyed System V semaphore sets and atomic semop transactions.
The syscall adapter copies operations and marshals the i386 IPC argument block;
blocking transactions restart under the same scheduler lock that tests the values.
Removal wakes waiters with EIDRM. SETALL validates all values before publishing.
SEM_UNDO and IPC_STAT/IPC_SET are explicitly unsupported. Legacy sem_init calls
retain their compatibility backend, now also owned by the scheduler.

Timed futex waits have a scheduler deadline and compete with explicit wake through
the same task state. Libc `pthread_sync.c` implements private mutexes, conditions
and once initialization on these queues. Mutexes support normal, recursive and
error-checking modes; the default checks ownership. Conditions reacquire the mutex
on timeout. Process-shared and robust pthread synchronization are not implemented.

## Signals

`schedule/signal.rs` owns pending delivery and trusted active-frame state. Masks,
alternate stacks and active handlers belong to each task; dispositions belong to
the process. Process-directed signals select an eligible task or stay pending.
Delivery occurs at the return-to-user boundary. Signal return requires the active
frame address, validates user instruction/stack addresses and forces user selectors
and safe flags. A user frame cannot grant kernel privilege.

Caught signals interrupt blocked syscalls with EINTR; nanosleep reports remaining
time. STOP/CONT preserve and restore scheduling state for every process thread.
User page faults signal SIGSEGV; unexpected supervisor faults panic the kernel.
Libc errno lookup uses stable per-thread slots so signal handlers do not acquire
the pthread registry lock. Semaphore and child-exit wakeups reject stale waiters.

The ABI supports signals 1–31 with a 32-bit mask, SA_ONSTACK, SA_NODEFER and
SA_RESETHAND. SA_RESTART and realtime signal queues are not implemented; unsupported
action flags return ENOTSUP. Alternate stacks and pending state reset on exec,
while the calling thread's mask is preserved.

## Process management

`schedule/control.rs` owns synchronized credentials, saved IDs, supplementary
groups, session/group identity and the process root. Identity changes are checked
before publication. Fork inherits control state; exec preserves credentials and
refreshes the loader's auxiliary IDs. PR_SET_NAME names the calling task; the
no-new-privileges flag is inherited and cannot be cleared. Set-ID executable bits
are not implemented. The existing mode-bit policy still applies to UID 0 file
access; UID 0 authorizes credential and ownership changes.

The process manager applies group/session and signal permission policy. Child exit,
stop and continue events remain available until a matching wait consumes them.
Waitpid supports exact, any-child and group selectors, WNOHANG, WUNTRACED and
WCONTINUED. Blocking waiters restart and recheck events; no user status pointer is
stored in a kernel wait queue. Waitid marshals these events with the child's actual UID and requires WEXITED.

Path resolution carries the process root through absolute symlink targets. Chroot
requires privilege and resets cwd to the new root. Directory descriptors outside
that root cannot anchor relative path lookup; existing non-directory descriptors
remain usable. Libc process.c owns PATH search; spawn.c owns file actions, signal
attributes and a close-on-exec error pipe that reports child setup/exec failures.
Spawn actions are bounded to 32. Current-thread pthread naming is supported;
naming another thread through pthread_setname_np returns ENOTSUP.

The initial userspace process has PID 1. The scheduler's ID allocator reserves
PID 0 for selector semantics. Adopted orphans are auto-reaped separately from
children created by init, which remain waitable. Init receives explicitly installed
signal handlers and ignores default fatal/stop dispositions.

## Terminals

`device/terminal.rs` owns the shared console line discipline, termios, input queues,
window size and controlling session/foreground group. Console aliases share this
state. The timer pumps keyboard and UART input, so control characters signal the
foreground group even when it is not reading. Background reads generate SIGTTIN;
TOSTOP writes and background terminal changes generate SIGTTOU.

Canonical input supports erase, kill, line boundaries and EOF. Raw input preserves
NUL and supports VMIN, including immediate reads with VMIN=0. Input CR/NL mapping,
echo, output NL processing, flushes and window-size notifications are implemented.
Output is synchronous, so TCSADRAIN has no software output queue to drain. Nonzero
VTIME, hardware baud/parity changes and unsupported termios flags return ENOTSUP.
The console is one terminal; pseudo-terminals are not implemented. Rust and C
readline share libc's character-mode setup and restoration.

The QEMU runner injects canonical editing, EOF, raw bytes and control characters;
it also verifies a stopped background reader resumes after foreground transfer.

Interrupt and syscall entry clear the direction flag before running Rust, while
iret restores the user's saved flags. The regression suite calls a syscall with
DF set to verify that user flags cannot reverse kernel memory copies.
