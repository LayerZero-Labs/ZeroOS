## spike-platform

`spike-platform` is the Spike (RISC‑V) platform integration crate for ZeroOS. It
provides a small ABI surface (symbols) that the arch/runtime/debug layers expect
to link against, plus the early platform bootstrap.

## Callstack diagrams

### Boot callstack

```
crates/zeroos-arch-riscv::_start
    ↳ crates/zeroos-arch-riscv::__bootstrap
        ↳ platforms/spike-platform::__platform_bootstrap()
            ↳ zeroos::initialize()
            ↳ foundation::kfn::memory::kinit(...)         (if memory)
            ↳ install trap vector (mtvec = _trap_handler) (if os-linux)
            ↳ foundation::kfn::scheduler::kinit()         (if thread; returns anchor address)
            ↳ foundation::kfn::vfs::kinit()               (if vfs)
            ↳ foundation::kfn::random::kinit(0)           (if random)
        ↳ crates/zeroos-runtime-*/__runtime_bootstrap()
            ↳ foundation::__main_entry(...)
```

### Trap / syscall callstack (RISC‑V)

When user/kernel code executes `ecall`, the CPU traps into the arch trap vector.

```
crates/zeroos-arch-riscv::_trap_handler
    ↳ crates/zeroos-arch-riscv::save_regs() -> *mut TrapFrame
        ↳ platforms/spike-platform::trap_handler(regs)
            ↳ arch-riscv::decode_trap(mcause)
            ↳ (ecall) mepc += 4
            ↳ os-linux::dispatch_syscall(regs)                    (if os-linux)
            ↳ foundation::kfn::scheduler::{update_frame,finish_trap} (if thread)
    ↳ crates/zeroos-arch-riscv::restore_regs(regs) -> !
```

## ABI surface: what spike-platform must provide

| Symbol                                                         | ABI | Required when          | Used by                    | Purpose                             |
| -------------------------------------------------------------- | --- | ---------------------- | -------------------------- | ----------------------------------- |
| `__platform_bootstrap() -> ()`                                 | C   | always (real platform) | `crates/zeroos-arch-riscv` | Early platform init hook            |
| `platform_exit(code: i32) -> !`                                | C   | always                 | `foundation::kfn::kexit`   | Terminate execution (HTIF on Spike) |
| `trap_handler(regs: *mut zeroos::arch_riscv::TrapFrame) -> ()` | C   | `target_arch=riscv*`   | `crates/zeroos-arch-riscv` | Exception/interrupt handler         |
| `__debug_write(msg: *const u8, len: usize) -> ()`              | C   | when `debug` is linked | `crates/zeroos-debug`      | Debug output sink                   |

## Symbols provided elsewhere (must still link)

| Symbol                         | Provider                                         | Selected by                     |
| ------------------------------ | ------------------------------------------------ | ------------------------------- |
| `__runtime_bootstrap() -> !`   | `crates/zeroos-runtime-nostd` / `-musl` / `-gnu` | `zeroos` / platform features    |
| `_trap_handler` (vector entry) | `crates/zeroos-arch-riscv` (weak default)        | platform may override if needed |

## Linker-provided symbols (addresses)

`__platform_bootstrap` expects these symbols from the linker script:

- `__heap_start`, `__heap_end`
- `__stack_top`, `__stack_bottom`
