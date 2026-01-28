# ZeroOS FAQ

## General Questions

### What exactly is ZeroOS?

ZeroOS is a modular library OS for zkVM environments. It implements the Linux
userspace syscall interface (syscall ABI + calling convention for the target
ISA) at the syscall trap boundary (e.g., `ecall` on RISC-V), enabling standard
toolchains and `std`-based programs to run without toolchain forks or runtime
patches.

### Why can't I just use a forked Rust standard library?

You can, but toolchain forks are a long-term maintenance burden (frequent
rebases), expand the TCB (bespoke runtime code), and fragment the ecosystem.
ZeroOS avoids this by operating at the syscall boundary instead.

### What zkVMs does ZeroOS support?

Currently, ZeroOS has been integrated with Jolt (see the signature recovery
example). The architecture is designed to be zkVM-agnostic: any zkVM that can
expose a syscall/trap boundary (e.g., trap on RISC-V `ecall`) can integrate
ZeroOS by implementing the syscall contract.

## Technical Questions

### What does "compatible" mean in practice?

"Compatible" means a binary compiled for a standard target (like
`riscv64imac-unknown-linux-musl`) can execute correctly inside the zkVM with
ZeroOS. Specifically:

1. **ISA compatibility**: The zkVM executes the RISC-V instructions correctly
2. **ABI compatibility**: The binary's calling conventions and data layouts
   match what libc and the syscall interface expect

### What ISA / ABI does ZeroOS target?

ZeroOS has a modular architecture designed to support multiple ISAs and OS ABIs.
Currently, RISC-V (ISA) and Linux (OS ABI) are supported, targeting binaries
compiled for `riscv64imac-unknown-linux-musl`. In principle, other ISAs (like
x86-64, ARM) and OS ABIs could be added by implementing the corresponding
architecture-specific and OS-specific modules.

### What's the performance overhead of using ZeroOS?

In the current integration, each syscall adds ~128 instructions to the trace: 57
to save registers, 33 in the trap handler, and 38 to restore registers. This is
largely a fixed per-syscall cost, but the exact count depends on the zkVM
integration/configuration.

### Does ZeroOS support multithreading?

Yes. ZeroOS includes a cooperative scheduler with threading primitives (mutexes,
condition variables, thread spawning). Rayon can run on top of this (execution
remains single-core in the zkVM, but threads are deterministically interleaved).

### How does determinism work in a multithreaded environment?

ZeroOS maintains determinism through single-core execution with no interrupts
and no preemption. Thread scheduling is cooperative and deterministic, so all
behavior is captured in the zkVM trace.

### What syscalls does ZeroOS currently support?

ZeroOS supports core syscalls across three main categories: memory management,
scheduler (threading and synchronization), and I/O. The signature recovery trace
shows these syscalls in action during execution.

### Does ZeroOS support processes (`fork`, `execve`) and full Unix semantics?

ZeroOS's modular design architecturally allows for supporting multi-process
semantics like `fork`, `execve`, and `wait4`. However, these are not currently
implemented, and there may be implementation challenges in adapting full process
semantics to the deterministic, single-core zkVM environment. The current focus
is on an in-process runtime model (memory, threads, basic I/O), which covers
most zkVM workloads today.

### How are time and randomness handled in a deterministic zkVM?

zkVM execution must be deterministic, so wall-clock time and entropy must be
virtualized or provided as explicit inputs. In the current design, the developer
supplies a seed when initializing the randomness subsystem during
`__platform_bootstrap`, and all randomness is derived deterministically from
that seed. If/when APIs like `getrandom` and `clock_gettime` are supported, they
must be backed by deterministic, trace-committed sources rather than the host
OS.

### Can I use file I/O in ZeroOS?

Yes, through a virtual filesystem (VFS). ZeroOS provides device abstractions
like console output, and you only link what you need.

### What filesystem semantics does the VFS provide?

The VFS is an abstraction layer, so semantics depend on which filesystem/device
modules you link (e.g., console, in-memory, or host-provided). For proofs, any
file content that influences execution should be treated as committed input.

### What happens if my program makes an unsupported syscall?

ZeroOS follows a fail-fast principle: unsupported syscalls are rejected
immediately with a clear error, rather than silently stubbing or returning fake
success. This ensures your trace only contains intentional, fully-supported
operations.

### How do I debug syscall failures or missing functionality?

Start from the syscall log/trace: identify the syscall and arguments, then
confirm (1) it is implemented and (2) the relevant module/device is linked and
configured. For missing syscalls, the most direct fix is usually to implement
the syscall (or a compatibility shim) rather than patch application code.

## Integration Questions

### How do I integrate ZeroOS into my zkVM?

See the
[zkVM integration guide](https://github.com/LayerZero-Labs/ZeroOS/blob/main/docs/zkvm-integration.md)
for detailed instructions, and the Jolt integration serves as a reference
implementation.

### What do I need to provide at the integration boundary?

You need to:

1. **Memory layout**: Declare the guest memory layout (heap, stack regions,
   etc.) via linker scripts
2. **Platform bootstrap**: Implement `__platform_bootstrap` and a few other
   platform-specific initialization functions

See the
[zkVM integration guide](https://github.com/LayerZero-Labs/ZeroOS/blob/main/docs/zkvm-integration.md)
for complete details on these requirements.

### How hard is it to add support for another syscall?

Each syscall is an independent compatibility unit: define the Linux-visible
semantics, implement them using deterministic primitives, and wire it into the
dispatcher. Some are mostly bookkeeping; others require additional
devices/subsystems (filesystem, clocks, randomness).

### Do I need to modify my application code to use ZeroOS?

No. If your code compiles with standard targets like
`riscv64imac-unknown-linux-musl`, it can run on ZeroOS without application-level
modifications (see compatibility notes above). The signature recovery example
uses upstream Reth and Rayon crates without maintaining forks.

### Can I use existing Rust crates that depend on `std`?

Yes. That’s the key advantage: you use standard toolchains and `std`-based
crates, and you inherit upstream security fixes and updates. Compatibility
depends on whether the crate’s runtime stays within the syscalls/devices you’ve
enabled.

### What about C/C++ programs?

ZeroOS is language-agnostic. Any language that compiles to RISC-V and links
against a libc can use ZeroOS, as long as the resulting program stays within the
supported syscall/device surface.

### Does ZeroOS support Go programs?

Yes— with more engineering work planned in the roadmap.

Go has its own runtime (at a similar layer to libc), and that runtime still
requests kernel services via the Linux userspace syscall interface at the trap
boundary (e.g., RISC-V `ecall`). With ZeroOS providing that syscall layer, Go
programs can run without forking the Go toolchain.

In practice, this works to the extent that the Go runtime’s required syscalls
and devices are implemented and enabled in your ZeroOS configuration.

## Architecture Questions

### What does "modular" mean in practice?

You link only the subsystems your program needs. For example, if you don't spawn
threads, you don't link the scheduler. This minimizes both TCB and trace length.
The dependency tree slide shows how Jolt features map to ZeroOS modules.

### How does ZeroOS handle memory allocation?

ZeroOS includes a memory management subsystem that handles `brk` and `mmap`
syscalls. The allocator implementation is pluggable - the example uses a
linked-list allocator.

### What's the TCB size of ZeroOS?

The TCB is modular and depends on which subsystems you link. The fail-fast
design and shared codebase across zkVMs help consolidate security audits
compared to maintaining separate toolchain forks.

## Practical Questions

### Is ZeroOS production-ready?

ZeroOS successfully runs the Jolt signature recovery workload and produces
verifiable proofs. The project demonstrates feasibility. Check the repository
for current status and limitations.

### What kinds of workloads is ZeroOS a good fit for today?

Compute-heavy workloads that fit a single-process model but want standard
toolchains/`std` (e.g., parsers, cryptography, signature verification). If you
need extensive OS services (networking, complex filesystems, multi-process
orchestration), expect additional integration work or a larger syscall/device
surface.

### Where can I find the code?

- Integration ZeroOS in zkVM:
  `github.com/LayerZero-Research/jolt/tree/gx/integrate-zeroos`
- Example application: `github.com/zouguangxian/jolt-on-zeroos`

### How can zkVM projects collaborate on ZeroOS?

Multiple zkVM projects can share one OS implementation instead of each
maintaining separate toolchain forks. This consolidates effort and reduces the
total TCB across the ecosystem. Reach out to discuss integration.

### What's the ask for zkVM developers?

Implement the syscall contract once in your zkVM, integrate ZeroOS, and stop
carrying bespoke runtime forks. This shifts the burden from "every language ×
every zkVM needs patches" to "each zkVM integrates once, all languages work."
