# Syscall Cycles Example

This example measures the fixed instruction-count overhead of the system call path in ZeroOS on RISC-V. 

To isolate the cost of the trap handler itself (entry/exit) from the cost of actual kernel logic (like VFS or memory management), this demo intentionally performs an invalid/unknown system call (`SYS_UNKNOWN = 0x1fff`). This ensures that the time spent inside the kernel is minimized to just the dispatch logic and an immediate error return, providing a "clean" measurement of the architectural fixed cost.

## How to Run

Execute the provided script from the root of the repository:

```bash
./build-syscall-cycles.sh
```

This script performs the following steps:
1. Build: Compiles the ZeroOS kernel and the `syscall-cycles` example for RISC-V 64-bit (`riscv64imac-zero-linux-musl`).
2. Execute: Runs the binary on the Spike simulator with full instruction logging enabled (`-l`).
3. Analyze: Parses the Spike trace log to extract the specific instruction sequence triggered by the `ecall` instruction.
4. Report: Generates a detailed instruction trace in `target/syscall-cycles-logs/syscall_unknown.exec.log`.

## Fixed Cost Analysis

A system call involves entering the kernel through the trap handler, saving the CPU state, performing the system call logic, and restoring the state before returning to userspace.

### Call Diagram

```text
Userspace (ecall)
    ↳ _default_trap_handler (Assembly)
        ↳ Save Context (~52 instructions)
        ↳ trap_handler (C/Rust)
            ↳ ksyscall (Dispatch)
                ↳ [Syscall Logic]
        ↳ ret_from_exception (Assembly)
            ↳ Restore Context (~41 instructions)
                ↳ Userspace (mret)
```

### Instruction Count Breakdown

The instruction counts below are derived from the RISC-V 64-bit implementation in `crates/zeroos-arch-riscv/src/trap.rs`:

*   Save Registers (~52 instructions):
    *   Context Setup (7 instructions): Swaps `tp` with `mscratch` to locate the `ThreadAnchor`, switches to the kernel stack, and allocates the `TrapFrame`.
    *   GPR Save (30 instructions): Saves all general-purpose registers (x1-x31, excluding x0 and including the stashed `tp`) to the `TrapFrame`.
    *   CSR Save (12 instructions): Reads and saves critical Control and Status Registers (`mstatus`, `mepc`, `mcause`, `mtval`, and the original `mscratch`).
    *   Finalize (3 instructions): Clears `mscratch`, sets up the first argument (`a0`) with the `TrapFrame` pointer, and calls the high-level handler.

*   Restore Registers (~41 instructions):
    *   Kernel State Sync (5 instructions): Updates the `ThreadAnchor` with the next kernel stack pointer and primes `mscratch` for the next trap.
    *   CSR Restore (4 instructions): Restores `mstatus` and `mepc`.
    *   GPR Restore (31 instructions): Loads all general-purpose registers (x1-x31) back from the `TrapFrame`.
    *   Exit (1 instruction): Executes `mret` to return to the original privilege level and program counter.

Total fixed overhead per syscall: ~93 instructions.


