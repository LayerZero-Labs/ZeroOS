use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "memory")] {
        pub mod memory;
    } else {
        pub(crate) mod memory;
    }
}
pub use memory::MemoryOps;

cfg_if! {
    if #[cfg(feature = "scheduler")] {
        pub mod scheduler;
    } else {
        pub(crate) mod scheduler;
    }
}
pub use scheduler::SchedulerOps;

cfg_if! {
    if #[cfg(feature = "vfs")] {
        pub mod vfs;
    } else {
        pub(crate) mod vfs;
    }
}
pub use vfs::VfsOps;

cfg_if! {
    if #[cfg(feature = "random")] {
        pub mod random;
    } else {
        pub(crate) mod random;
    }
}
pub use random::RandomOps;

cfg_if! {
    if #[cfg(feature = "arch")] {
        pub mod arch;
    } else {
        pub(crate) mod arch;
    }
}
pub use arch::ArchOps;

cfg_if! {
    if #[cfg(feature = "trap")] {
        pub mod trap;
    } else {
        pub(crate) mod trap;
    }
}
pub use trap::TrapOps;
