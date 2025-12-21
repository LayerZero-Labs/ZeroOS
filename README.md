# ZeroOS

_A universal modular Library OS (libOS) for zkVM-powered verifiable
applications._

ZeroOS is a modular library operating system that transforms normal applications
into verifiable applications (vApps) simply by linking against a lightweight,
platform-agnostic OS layer. Unlike prior zkVM stacks that rely on patched
toolchains and modified language runtimes, ZeroOS requires no changes to libc or
language standard libraries. Developers use off-the-shelf Rust/C toolchains,
link against ZeroOS, and obtain a deterministic, zkVM-compatible unikernel.

ZeroOS enables zkVMs to standardize around a shared OS layer—radically reducing
fragmentation, audit burden, and maintenance cost across the ecosystem. Every
zkVM platform integrating ZeroOS implements only one function:
__platform_bootstrap.

ZeroOS is open-source, built for extensibility, and designed to encourage
community contributions.

## Key Features

### Language-agnostic syscall shim

ZeroOS intercepts real syscall instructions and implements a stable subset of
the Linux syscall ABI inside the kernel.

- Works with unmodified musl libc and Rust `std`
- Eliminates version hell associated with patched toolchains
- Ensures long-term ABI stability (kernel ABIs are more stable than language
  runtimes)

### Modular architecture

ZeroOS is composed of small, swappable kernel modules:

- Memory allocators (freelist, buddy, bump, etc.)
- Syscall wrappers toggled via compile-time flags
- Swappable schedulers
- Optional I/O and device modules

Developers include _only_ what their vApp requires—minimizing trusted computing
base and zkVM trace length.

### Deterministic, zkVM-friendly design

Every subsystem is engineered to preserve determinism:

- No external nondeterminism (time, randomness, signals are stubbed or
  controlled)
- Single-address-space unikernel model
- All guest behavior is fully captured by the zkVM trace

### Minimal platform integration surface

To port ZeroOS to a zkVM, implement only:

```c
void __platform_bootstrap();
```

This function sets up:

- Trap handler registration
- Memory-mapped I/O regions
- Platform-specific device initialization

Everything else is shared across all platforms.

### Unikernel-style, static linking simplicity

ZeroOS builds a fully static ELF that runs directly on the zkVM’s bare-metal
execution model while preserving the familiar POSIX/Linux syscall interface.

## Design Principles

ZeroOS is guided by a set of foundational principles aimed at delivering maximum security, transparency, and trust for developers building verifiable applications.

### Fail-Fast Engineering
ZeroOS follows a fail-fast philosophy: rather than masking incomplete features behind silent stubs, the kernel immediately returns an error or halts when encountering unsupported requests (such as certain mmap flags or malloc options). This approach guarantees that every execution path in a zkVM trace is both intentional and fully supported.

### Toolchain Integrity
Developers can rely on standard toolchains without modification. ZeroOS works seamlessly with off-the-shelf musl-libc and Rust std, keeping the Trusted Computing Base (TCB) lean. By avoiding custom compiler patches or altered libraries, audit efforts remain focused on the ZeroOS kernel and application logic, not sprawling toolchain changes.

### Surgical Modularity
The kernel is built from modular, compile-time configurable components. This design lets developers include only the functionality they truly need, reducing binary size and zkVM trace length. With less code to review, audits become more straightforward and effective.

### Architectural Transparency
ZeroOS prioritizes clarity over abstraction. Core operations—such as thread creation, trap handling, and context switching—are implemented with explicit initialization and visible resource ownership. This transparency makes it easier to inspect critical paths for vulnerabilities and ensures deterministic behavior.

### Unikernel Simplicity
Operating as a statically linked unikernel in a single address space, ZeroOS avoids the complexity of multiple privilege levels and MMU management. In zkVM environments, these mechanisms often add unnecessary overhead and subtle risks. By stripping them away, ZeroOS achieves a simpler, more secure execution model.

## Getting Started

### Prepare

```bash
./bootstrap
```

If you're working in a devcontainer, run:

```bash
./bootstrap --install-shell-integration
```

### Examples

```bash
./build-fibonacci.sh
./build-std-smoke.sh
./build-c-smoke.sh
```

### Check/Lint/Format/Test

```bash
cargo matrix fix
cargo matrix fmt
cargo matrix check
cargo matrix test
```

or

```bash
cargo massage
```

## License

See `LICENSE-MIT` and `LICENSE-APACHE` for details.
