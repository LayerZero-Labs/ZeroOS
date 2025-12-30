#![no_std]

zeroos_macros::require_at_most_one_feature!("alloc-linked-list", "alloc-buddy", "alloc-bump");
zeroos_macros::require_at_most_one_feature!("scheduler-cooperative");

pub use foundation;

pub use zeroos_macros as macros;

#[cfg(feature = "arch-riscv")]
pub extern crate arch_riscv;

#[cfg(feature = "arch-riscv")]
pub use arch_riscv::TrapFrame;

#[cfg(feature = "os-linux")]
extern crate os_linux;

#[cfg(target_os = "none")]
pub extern crate runtime_nostd;

#[cfg(feature = "runtime-musl")]
pub extern crate runtime_musl;

#[cfg(feature = "runtime-gnu")]
pub extern crate runtime_gnu;

#[cfg(feature = "memory")]
pub use foundation::register_memory;

#[cfg(feature = "vfs")]
pub use foundation::register_vfs;

#[cfg(feature = "scheduler")]
pub use foundation::register_scheduler;

#[cfg(feature = "random")]
pub use foundation::register_random;

pub mod arch {
    #[cfg(all(
        feature = "arch-riscv",
        any(target_arch = "riscv32", target_arch = "riscv64")
    ))]
    pub mod riscv {
        pub use arch_riscv::{boot, trap};

        pub use arch_riscv::{Exception, Trap, __bootstrap, _default_trap_handler, _start};

        pub use arch_riscv::TrapFrame;
    }
}

pub mod os {
    #[cfg(feature = "os-linux")]
    pub mod linux {
        pub use os_linux::*;
    }
}

pub mod runtime {
    #[cfg(feature = "runtime-nostd")]
    pub use runtime_nostd as nostd;

    pub mod libc {
        #[cfg(feature = "runtime-musl")]
        pub use runtime_musl as musl;

        #[cfg(feature = "runtime-gnu")]
        pub use runtime_gnu as gnu;
    }
}

#[cfg(target_os = "none")]
pub use runtime_nostd::alloc;

#[cfg(feature = "vfs")]
pub mod vfs {
    pub use vfs_core::*;

    pub mod devices {
        #[cfg(feature = "vfs-device-console")]
        pub use device_console as console;

        #[cfg(feature = "vfs-device-null")]
        pub use device_null as null;

        #[cfg(feature = "vfs-device-urandom")]
        pub use device_urandom as urandom;

        #[cfg(feature = "vfs-device-zero")]
        pub use device_zero as zero;
    }
}

#[cfg(feature = "scheduler")]
pub mod scheduler {
    #[cfg(feature = "scheduler-cooperative")]
    pub use scheduler_cooperative::*;
}

#[cfg(any(feature = "rng-lcg", feature = "rng-chacha"))]
pub mod rng {
    pub use rng::*;
}

pub fn initialize() {
    #[cfg(feature = "arch-riscv")]
    foundation::register_arch(arch_riscv::ARCH_OPS);

    #[cfg(feature = "os-linux")]
    foundation::register_trap(os_linux::TRAP_OPS);

    #[cfg(feature = "alloc-linked-list")]
    foundation::register_memory(allocator_linked_list::LINKED_LIST_ALLOCATOR_OPS);

    #[cfg(feature = "alloc-buddy")]
    foundation::register_memory(allocator_buddy::BUDDY_ALLOCATOR_OPS);

    #[cfg(feature = "alloc-bump")]
    foundation::register_memory(allocator_bump::BUMP_ALLOCATOR_OPS);

    #[cfg(feature = "vfs")]
    foundation::register_vfs(vfs_core::VFS_OPS);

    #[cfg(feature = "scheduler-cooperative")]
    foundation::register_scheduler(scheduler_cooperative::SCHEDULER_OPS);

    #[cfg(feature = "random")]
    foundation::register_random(rng::RNG_OPS);
}
