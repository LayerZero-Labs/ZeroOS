extern crate alloc;

#[allow(unused_imports)]
pub use htif::{fromhost, tohost};

extern "C" {
    static __heap_start: u8;
    static __heap_end: u8;
    static __stack_top: u8;
    static __stack_bottom: u8;
}

#[inline(always)]
#[cfg(feature = "os-linux")]
fn install_trap_vector() {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        core::arch::asm!("la      t0, _trap_handler", "csrw    mtvec, t0",);
    }
}

#[no_mangle]
pub extern "C" fn __platform_bootstrap() {
    debug::writeln!("[BOOT] __platform_bootstrap");

    zeroos::initialize();

    #[cfg(feature = "memory")]
    {
        let heap_start = core::ptr::addr_of!(__heap_start) as usize;
        let heap_end = core::ptr::addr_of!(__heap_end) as usize;
        debug::writeln!("[BOOT] Heap start=0x{:x}, end=0x{:x}", heap_start, heap_end);
        let heap_size = heap_end - heap_start;
        foundation::kfn::memory::kinit(heap_start, heap_size);

        let _stack_top = core::ptr::addr_of!(__stack_top) as usize;
        let _stack_bottom = core::ptr::addr_of!(__stack_bottom) as usize;
        debug::writeln!(
            "[BOOT] Stack top=0x{:x}, bottom=0x{:x}",
            _stack_top,
            _stack_bottom
        );
    }

    cfg_if::cfg_if! {
        if #[cfg(not(target_os = "none"))] {
            #[cfg(feature = "os-linux")]
            {
                install_trap_vector();
                debug::writeln!("[BOOT] Trap handler installed");
            }

            #[cfg(feature = "thread")]
            let boot_thread_anchor: usize = {
                let anchor = foundation::kfn::scheduler::kinit();

                // Trap entry swaps tp <-> mscratch. In kernel, keep tp=anchor and mscratch=0 so
                // traps are treated as kernel traps and the kernel can find the current anchor.
                #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
                unsafe {
                    core::arch::asm!("mv tp, {0}", in(reg) anchor);
                    core::arch::asm!("csrw mscratch, x0");
                }

                anchor
            };

            #[cfg(feature = "vfs")]
            {
                foundation::kfn::vfs::kinit();

                #[cfg(feature = "vfs-device-console")]
                {
                    debug::writeln!("[BOOT] Registering console file descriptors");
                    register_console_fd(1, &STDOUT_FOPS);
                    register_console_fd(2, &STDERR_FOPS);
                }
            }

            #[cfg(feature = "random")]
            {
                // SECURITY: RNG seed is fixed (0) for deterministic runs (e.g. sims/tests).
                // Please Replace with a proper seed source for production/real entropy use.
                foundation::kfn::random::kinit(0);
            }

            // Before entering libc: leave tp for TLS (musl owns it) and park anchor in mscratch,
            // so a user trap swaps the anchor into tp on entry.
            #[cfg(feature = "thread")]
            {
                #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
                unsafe {
                    core::arch::asm!("csrw mscratch, {0}", in(reg) boot_thread_anchor);
                    core::arch::asm!("mv tp, x0");
                }
            }
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "vfs-device-console")] {
        use zeroos::vfs::{self};

        fn htif_console_write(_file: *mut u8, buf: *const u8, count: usize) -> isize {
            unsafe {
                let slice = core::slice::from_raw_parts(buf, count);
                for &byte in slice {
                    htif::putchar(byte);
                }
            }
            count as isize
        }

        fn register_console_fd(fd: i32, ops: &'static vfs::FileOps) {
            debug::writeln!("[HTIF] register_console_fd fd={}", fd);
            let _ = vfs::register_fd(
                fd,
                vfs::FdEntry {
                    ops,
                    private_data: core::ptr::null_mut(),
                },
            );
        }

        static STDOUT_FOPS: vfs::FileOps = vfs::devices::console::stdout_fops(htif_console_write);
        static STDERR_FOPS: vfs::FileOps = vfs::devices::console::stderr_fops(htif_console_write);
    }
}
