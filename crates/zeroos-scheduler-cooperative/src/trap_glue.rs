use crate::context::TrapFrame;
use crate::scheduler::Scheduler;
use crate::thread::ThreadControlBlock;
use crate::thread::{apply_thread_ctx_to_frame, sync_thread_ctx_from_frame};
use core::ptr::NonNull;
use foundation::utils::GlobalCell;

static LAST_TRAP_THREAD: GlobalCell<Option<NonNull<ThreadControlBlock>>> = GlobalCell::new(None);

#[inline(always)]
unsafe fn read_trap_frame_from_ptr(ptr: *const TrapFrame) -> TrapFrame {
    core::ptr::read(ptr)
}

#[inline(always)]
unsafe fn write_trap_frame_to_ptr(frame: &TrapFrame, ptr: *mut TrapFrame) {
    core::ptr::write(ptr, *frame)
}

impl Scheduler {
    /// # Safety
    /// If `frame_ptr` is non-null, it must point to a valid `TrapFrame` for reads.
    pub unsafe fn update_current_from_frame(&mut self, frame_ptr: *const TrapFrame, mepc: usize) {
        if frame_ptr.is_null() {
            return;
        }
        if let Some(tcb) = self.current_thread() {
            unsafe {
                (*tcb.as_ptr()).trap_frame = read_trap_frame_from_ptr(frame_ptr);
                sync_thread_ctx_from_frame(
                    &mut (*tcb.as_ptr()).thread_ctx,
                    &(*tcb.as_ptr()).trap_frame,
                );
                (*tcb.as_ptr()).saved_pc = mepc;
            }
        }
    }

    /// # Safety
    /// - If `frame_ptr` is non-null, it must be valid for reads and writes of a `TrapFrame`.
    /// - If `mepc_ptr` is non-null, it must be valid for reads and writes of a `usize`.
    pub unsafe fn finish_trap(
        &mut self,
        frame_ptr: *mut TrapFrame,
        mepc_ptr: *mut usize,
        _mepc: usize,
    ) {
        if frame_ptr.is_null() {
            return;
        }

        let entry = LAST_TRAP_THREAD.with(|t| *t);
        let current = self.current_thread();
        let switched = entry.is_some() && current != entry;

        if let Some(tcb) = entry {
            (*tcb.as_ptr()).trap_frame = read_trap_frame_from_ptr(frame_ptr);
            sync_thread_ctx_from_frame(
                &mut (*tcb.as_ptr()).thread_ctx,
                &(*tcb.as_ptr()).trap_frame,
            );
            if !mepc_ptr.is_null() {
                (*tcb.as_ptr()).saved_pc = mepc_ptr.read();
            }
        }

        if switched {
            if let Some(tcb) = current {
                apply_thread_ctx_to_frame(
                    &mut (*tcb.as_ptr()).trap_frame,
                    &(*tcb.as_ptr()).thread_ctx,
                );
                write_trap_frame_to_ptr(&(*tcb.as_ptr()).trap_frame, frame_ptr);

                if !mepc_ptr.is_null() {
                    mepc_ptr.write((*tcb.as_ptr()).saved_pc);
                }
            }
        }

        LAST_TRAP_THREAD.with_mut(|t| *t = None);
    }
}

pub fn update_frame(frame_ptr: usize, mepc: usize) {
    Scheduler::with_mut(|scheduler| {
        if scheduler.thread_count != 0 {
            unsafe { scheduler.update_current_from_frame(frame_ptr as *const TrapFrame, mepc) };
            LAST_TRAP_THREAD.with_mut(|last| *last = scheduler.current_thread());
        }
    });
}

pub fn finish_trap(frame_ptr: usize, mepc_ptr: usize, mepc: usize) {
    Scheduler::with_mut(|scheduler| unsafe {
        scheduler.finish_trap(frame_ptr as *mut TrapFrame, mepc_ptr as *mut usize, mepc);
    });
}
