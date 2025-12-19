extern crate zeroos;

use foundation::kfn::arch as karch;

use riscv::register::mcause::Exception;

#[inline(always)]
fn mcause_is_interrupt(mcause: usize) -> bool {
    mcause >> (usize::BITS as usize - 1) != 0
}

#[inline(always)]
fn mcause_code(mcause: usize) -> usize {
    // RISC-V encodes interrupts by setting the top bit of mcause; the rest is the code.
    mcause & ((1usize << (usize::BITS as usize - 1)) - 1)
}

#[inline(always)]
fn advance_mepc_for_breakpoint(regs: *mut u8) {
    unsafe {
        let pc = karch::ktrap_frame_get_pc(regs);
        karch::ktrap_frame_set_pc(regs, pc.wrapping_add(instr_len(pc)));
    }
}

#[inline(always)]
fn instr_len(addr: usize) -> usize {
    let halfword = unsafe { core::ptr::read_unaligned(addr as *const u16) };
    if (halfword & 0b11) == 0b11 {
        4
    } else {
        2
    }
}

/// # Safety
/// `regs` must be a non-null pointer to a valid `TrapFrame` for the current CPU trap context.
#[no_mangle]
pub unsafe extern "C" fn trap_handler(regs: *mut u8) {
    let mcause = unsafe { karch::ktrap_frame_get_cause(regs) };
    if mcause_is_interrupt(mcause) {
        // Interrupt handling is disabled
        return;
    }

    match mcause_code(mcause) {
        // Handle envcalls (syscalls) from any privilege mode.
        code if code == (Exception::UserEnvCall as usize)
            || code == (Exception::SupervisorEnvCall as usize)
            || code == (Exception::MachineEnvCall as usize) =>
        unsafe {
            let pc = karch::ktrap_frame_get_pc(regs);
            karch::ktrap_frame_set_pc(regs, pc + 4);

            let ret = foundation::kfn::trap::ksyscall(
                karch::ktrap_frame_get_arg(regs, 0),
                karch::ktrap_frame_get_arg(regs, 1),
                karch::ktrap_frame_get_arg(regs, 2),
                karch::ktrap_frame_get_arg(regs, 3),
                karch::ktrap_frame_get_arg(regs, 4),
                karch::ktrap_frame_get_arg(regs, 5),
                karch::ktrap_frame_get_nr(regs),
            );
            karch::ktrap_frame_set_retval(regs, ret as usize);
        },
        code if code == (Exception::Breakpoint as usize) => {
            advance_mepc_for_breakpoint(regs);
        }
        code => {
            htif::exit(code as u32);
        }
    }
}
