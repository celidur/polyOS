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
kernel. The legacy private seven-argument receive call passes an explicit argument
block in EBX; it does not extend the register ABI with hidden stack arguments.
