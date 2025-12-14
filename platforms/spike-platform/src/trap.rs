extern crate zeroos;

use zeroos::arch_riscv::TrapFrame;

use zeroos::arch_riscv::{decode_trap, Exception, Trap};

#[cfg(feature = "thread")]
use foundation::kfn;

#[inline(always)]
fn advance_mepc_for_breakpoint(regs: &mut TrapFrame) {
    regs.mepc = regs.mepc.wrapping_add(instr_len(regs.mepc));
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
pub unsafe extern "C" fn trap_handler(regs: *mut TrapFrame) {
    let regs = unsafe { &mut *regs };

    match decode_trap(regs.mcause) {
        Trap::Exception(Exception::MachineEnvCall)
        | Trap::Exception(Exception::SupervisorEnvCall)
        | Trap::Exception(Exception::UserEnvCall) => {
            regs.mepc += 4;

            #[cfg(feature = "thread")]
            {
                let frame_ptr = regs as *mut TrapFrame as usize;
                kfn::scheduler::update_frame(frame_ptr, regs.mepc);
            }

            #[cfg(feature = "os-linux")]
            {
                unsafe { zeroos::os::linux::dispatch_syscall(regs as *mut TrapFrame) };
            }

            #[cfg(not(feature = "os-linux"))]
            {
                foundation::SyscallFrame::set_ret(regs, -38);
            }

            #[cfg(feature = "thread")]
            {
                let frame_ptr = regs as *mut TrapFrame as usize;
                kfn::scheduler::finish_trap(
                    frame_ptr,
                    (&mut regs.mepc as *mut usize) as usize,
                    regs.mepc,
                );
            }
        }
        Trap::Exception(Exception::Breakpoint) => {
            advance_mepc_for_breakpoint(regs);
        }
        Trap::Exception(code) => {
            htif::exit(code as u32);
        }
        Trap::Interrupt(_code) => {}
    }
}
