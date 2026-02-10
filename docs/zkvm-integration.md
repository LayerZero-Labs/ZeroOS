# zkVM Integration with ZeroOS

This document describes how zero-knowledge virtual machines (zkVMs) can build on
and integrate with ZeroOS, using [Jolt](https://github.com/a16z/jolt) as a
reference implementation.

## Overview

ZeroOS provides a modular operating system foundation that zkVMs can use to:

- Execute guest programs with standard library support
- Handle Linux syscalls deterministically
- Manage memory allocation and virtual file systems
- Provide threading and random number generation

The integration is feature-driven, allowing zkVMs to include only the subsystems
they need.

## Reference Implementation: Jolt

- **Repository**: https://github.com/LayerZero-Research/jolt.git (fork of
  https://github.com/a16z/jolt.git)
- **Branch**: `gx/wip_csr`

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Guest Program                         │
│            (provable code / application logic)            │
├─────────────────────────────────────────────────────────┤
│                      jolt-sdk                            │
│  ┌────────────┐ ┌────────┐ ┌────────┐ ┌──────────┐      │
│  │ Cargo.toml │ │ boot.rs│ │ trap.rs│ │ ecall.rs │      │
│  └────────────┘ └────────┘ └────────┘ └──────────┘      │
├─────────────────────────────────────────────────────────┤
│             Language + dependency libraries              │
│    Rust core/alloc/std + third-party crates (e.g. rayon) │
├───────────────────────────┬─────────────────────────────┤
│         Runtime           │          Runtime             │
│  ┌─────────────────────┐  │  ┌───────────────────────┐  │
│  │   runtime-nostd     │  │  │     runtime-musl      │  │
│  │     (no libc)       │  │  │  (musl libc, Linux    │  │
│  │                     │  │  │   syscall ABI)        │  │
│  └─────────────────────┘  │  └───────────────────────┘  │
├───────────────────────────┼─────────────────────────────┤
│                           │       ZeroOS kernel          │
│                           │  ┌───────────────────────┐  │
│                           │  │      os-linux         │  │
│                           │  │  (Linux syscall tbl)  │  │
│  ┌─────────────────────┐  │  ├───────────────────────┤  │
│  │ optional subsystems │  │  │  optional subsystems  │  │
│  │   (vfs/random)      │  │  │  (vfs/sched/random)   │  │
│  │  + devices          │  │  │  + devices (console)  │  │
│  ├─────────────────────┤  │  ├───────────────────────┤  │
│  │     foundation      │  │  │      foundation       │  │
│  │ - memory,vfs,random │  │  │ - memory,vfs,sched,   │  │
│  │                     │  │  │   random              │  │
│  └─────────────────────┘  │  └───────────────────────┘  │
├───────────────────────────┴─────────────────────────────┤
│                 Hardware / Emulator (zkVM)               │
└─────────────────────────────────────────────────────────┘
```

## Execution Flow

### Boot Sequence (nostd Mode)

```
_start (linker entry)
    → __platform_bootstrap()
        → zeroos::initialize()
        → memory::kinit(__heap_start, __heap_end - __heap_start)
    → __runtime_bootstrap()
        → __main_entry()
            → main()
```

### Boot Sequence (std Mode / musl)

```
_start (linker entry)
    → __platform_bootstrap()
        → zeroos::initialize()
        → memory::kinit(__heap_start, __heap_end - __heap_start)
        → install_trap_vector()
        → scheduler::kinit()  (if thread feature)
        → vfs::kinit()        (if vfs feature)
        → random::kinit(0)    (if random feature)
    → __runtime_bootstrap()
        → __libc_start_main(__main_entry, ...)
            → __main_entry()
                → main()
```

### Syscall Flow (std Mode)

Guest programs make Linux syscalls via the RISC-V `ecall` instruction:

```
ecall (a7 = syscall number, a0-a5 = args)
    → trap_handler()
        → zeroos::foundation::kfn::trap::ksyscall()
            → zeroos::os::linux::linux_handle()
                → syscall handler (read, write, mmap, etc.)
    → return to guest
```

## Integration Points

### 1. Linker Script

The linker script defines memory layout and exports symbols used by the runtime
and SDK.

#### Memory Layout

```
┌─────────────────────┐ ← __heap_end / __memory_end
│                     │
│       Heap          │   (grows upward)
│                     │
├─────────────────────┤ ← __heap_start
├─────────────────────┤ ← __stack_top (initial SP)
│                     │
│       Stack         │   (grows downward)
│                     │
├─────────────────────┤ ← __stack_bottom
│   Stack Canary      │   (128 bytes)
├─────────────────────┤
│       .bss          │
├─────────────────────┤
│   .tbss / .tdata    │   (TLS sections)
├─────────────────────┤
│     .data/.sdata    │
├─────────────────────┤
│      .rodata        │
├─────────────────────┤
│       .text         │
├─────────────────────┤
│    .text.boot       │
└─────────────────────┘ ← ORIGIN
```

#### Memory Region Symbols

| Symbol              | Used by             | Purpose                                          |
| ------------------- | ------------------- | ------------------------------------------------ |
| `__heap_start`      | `memory::kinit()`   | Start of heap region                             |
| `__heap_end`        | `memory::kinit()`   | End of heap region                               |
| `__stack_top`       | `_start` (assembly) | Initial stack pointer (`lla sp, __stack_top`)    |
| `__stack_bottom`    | boot.rs (debug)     | Bottom of stack (for overflow detection)         |
| `__global_pointer$` | `_start` (assembly) | RISC-V gp register (`lla gp, __global_pointer$`) |

#### Section Symbols (optional)

| Symbol                                   | Purpose                                    |
| ---------------------------------------- | ------------------------------------------ |
| `__bss_start`, `__bss_end`               | BSS zeroing at startup                     |
| `__tdata_start`, `__tdata_end`           | Thread-local data template                 |
| `__tbss_start`, `__tbss_end`             | Thread-local BSS                           |
| `__init_array_start`, `__init_array_end` | Constructor functions (called before main) |
| `__fini_array_start`, `__fini_array_end` | Destructor functions (called after main)   |

### 2. Platform Functions

The SDK must implement platform-specific functions to wire ZeroOS into the zkVM.

#### Required Symbols

| Symbol                 | Provided by        | Required  | Purpose                                                      |
| ---------------------- | ------------------ | --------- | ------------------------------------------------------------ |
| `__platform_bootstrap` | boot.rs            | Yes       | Called by runtime before `main()`                            |
| `trap_handler`         | trap.rs            | Yes       | Called by `_default_trap_handler` to handle traps            |
| `__platform_exit`      | exit.rs            | Yes       | Called by `foundation::kfn::kexit()` for program termination |
| `_start`               | runtime (assembly) | No (weak) | Program entry point, can override default                    |
| `_trap_handler`        | runtime (assembly) | No (weak) | Trap vector address, can override default                    |

#### Required: `__platform_bootstrap()` (boot.rs)

Always required. Called before `main()`:

```rust
extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
    static __stack_top: u8;
    static __stack_bottom: u8;
}

#[inline(always)]
#[cfg(all(target_arch ="riscv64", target_os = "linux"))]
fn install_trap_vector() {
    unsafe {
        core::arch::asm!("la t0, _trap_handler", "csrw mtvec, t0");
    }
}

#[no_mangle]
pub extern "C" fn __platform_bootstrap() {
    zeroos::initialize();

    // Initialize heap allocator
    {
        let heap_start = core::ptr::addr_of!(__heap_start) as usize;
        let heap_end = core::ptr::addr_of!(__heap_end) as usize;
        let heap_size = heap_end - heap_start;
        zeroos::foundation::kfn::memory::kinit(heap_start, heap_size);
    }

    #[cfg(not(target_os = "none"))]
    {
        install_trap_vector();

        #[cfg(feature = "zeroos-thread")]
        let boot_thread_anchor: usize = {
            let anchor = zeroos::foundation::kfn::scheduler::kinit();
            unsafe {
                core::arch::asm!("mv tp, {0}", in(reg) anchor);
                core::arch::asm!("csrw mscratch, x0");
            }
            anchor
        };

        #[cfg(feature = "zeroos-vfs")]
        {
            zeroos::foundation::kfn::vfs::kinit();

            #[cfg(feature = "zeroos-vfs-device-console")]
            {
                register_console_fd(1, &STDOUT_FOPS);
                register_console_fd(2, &STDERR_FOPS);
            }
        }

        #[cfg(feature = "zeroos-random")]
        {
            // RNG seed is fixed (0) for deterministic ZK proofs
            zeroos::foundation::kfn::random::kinit(0);
        }

        // Before entering libc: park anchor in mscratch for user trap handling
        #[cfg(feature = "zeroos-thread")]
        unsafe {
            core::arch::asm!("csrw mscratch, {0}", in(reg) boot_thread_anchor);
            core::arch::asm!("mv tp, x0");
        }
    }
}
```

#### Required for std mode: `trap_handler()` (trap.rs)

Routes CPU traps to ZeroOS syscall handling:

```rust
use zeroos::arch::riscv::TrapFrame;
use riscv::register::mcause::Exception;

#[inline(always)]
fn mcause_is_interrupt(mcause: usize) -> bool {
    mcause >> (usize::BITS as usize - 1) != 0
}

#[inline(always)]
fn mcause_code(mcause: usize) -> usize {
    mcause & ((1usize << (usize::BITS as usize - 1)) - 1)
}

#[inline(always)]
fn advance_mepc_for_breakpoint(regs: *mut TrapFrame) {
    unsafe {
        let pc = (*regs).mepc;
        (*regs).mepc = pc.wrapping_add(instr_len(pc));
    }
}

#[inline(always)]
fn instr_len(addr: usize) -> usize {
    let halfword = unsafe { core::ptr::read_unaligned(addr as *const u16) };
    if (halfword & 0b11) == 0b11 { 4 } else { 2 }
}

#[no_mangle]
pub unsafe extern "C" fn trap_handler(regs: *mut u8) {
    let regs = regs as *mut TrapFrame;
    let mcause = (*regs).mcause;
    if mcause_is_interrupt(mcause) {
        return;
    }

    match mcause_code(mcause) {
        // Handle envcalls (syscalls) from any privilege mode
        code if code == (Exception::UserEnvCall as usize)
            || code == (Exception::SupervisorEnvCall as usize)
            || code == (Exception::MachineEnvCall as usize) =>
        {
            let pc = (*regs).mepc;
            (*regs).mepc = pc + 4;

            let ret = zeroos::foundation::kfn::trap::ksyscall(
                (*regs).a0,
                (*regs).a1,
                (*regs).a2,
                (*regs).a3,
                (*regs).a4,
                (*regs).a5,
                (*regs).a7,
            );
            (*regs).a0 = ret as usize;
        }
        code if code == (Exception::Breakpoint as usize) => {
            advance_mepc_for_breakpoint(regs);
        }
        code => {
            zeroos::foundation::kfn::kexit(code as i32);
        }
    }
}
```

### 3. SDK Configuration (Cargo.toml)

The SDK crate serves multiple build contexts: guest programs (std and nostd
modes) and host/prover programs. Feature flags control which ZeroOS subsystems
are included.

#### Build Context Features

The SDK defines three primary features for different build contexts:

```toml
host = [
    "jolt-core/default",
    "jolt-platform/std",
    "dep:tracer",
    "dep:common",
    ...
]

guest-nostd = [
    "zeroos-nostd",
]

guest-std = [
    "postcard/use-std",
    "jolt-platform/std",
    "jolt-sdk-macros/guest-std",
    "serde/std",
    "zeroos-std",
]
```

- `host`: For prover/verifier programs running on the host machine
- `guest-nostd`: Minimal guest without libc (uses `zeroos-runtime-nostd`)
- `guest-std`: Full std support via musl libc (uses `zeroos-runtime-musl`)

#### Target-Conditional Dependencies

ZeroOS features are activated based on target architecture and OS. This ensures
the correct subsystems are included only when compiling for the zkVM target:

```toml
[target.'cfg(all(target_arch = "riscv64"))'.dependencies]
riscv = { workspace = true }
zeroos = { workspace = true, default-features = false, features = ["debug", "arch-riscv", "alloc-linked-list"]}

[target.'cfg(all(target_arch = "riscv64", target_os = "linux"))'.dependencies]
zeroos = { workspace = true, default-features = false, features = ["os-linux"]}
```

- `target_arch = "riscv64"`: Enables RISC-V architecture support and heap
  allocator
- `target_os = "linux"`: Adds Linux syscall emulation (required for std mode)

#### Subsystem Feature Aliases

The SDK defines `zeroos-*` features for internal use in `support/` code, with
user-friendly aliases:

```toml
# User-facing aliases
thread = ["zeroos-thread"]
random = ["zeroos-random"]
stdout = ["zeroos-vfs-device-console"]

# Internal features (used by boot.rs via cfg(feature = "zeroos-..."))
zeroos-runtime-nostd = ["zeroos?/runtime-nostd"]
zeroos-runtime-musl = ["zeroos?/runtime-musl"]
zeroos-thread = ["zeroos?/scheduler-cooperative"]
zeroos-vfs = ["zeroos?/vfs"]
zeroos-random = ["zeroos?/random", "zeroos?/rng-lcg"]
zeroos-vfs-device-console = ["zeroos-vfs", "zeroos?/vfs-device-console"]
```

This naming convention:

- Avoids conflicts with other crate-specific features
- Provides simple aliases (`thread`, `random`, `stdout`) that hide ZeroOS
  implementation details
- Enables conditional compilation in `support/` code via
  `#[cfg(feature = "zeroos-thread")]`

### 4. Build Infrastructure

The build crate provides a build command (similar syntax to `cargo build`) and
uses `zeroos-build` to standardize how guest programs are compiled and linked
for the zkVM:

```toml
[dependencies]
zeroos-build.workspace = true
```

Key responsibilities:

- Toolchain management (e.g. musl toolchain for `guest-std`)
- Target/ABI configuration (target spec / target triple selection)
- Compile and link options (rustflags, linker args, linker script generation)
- Reproducible guest builds (stable paths/layout, deterministic configuration)
