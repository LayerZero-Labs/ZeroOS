const AT_NULL: usize = 0;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_PAGESZ: usize = 6;
const AT_ENTRY: usize = 9;
const AT_UID: usize = 11;
const AT_EUID: usize = 12;
const AT_GID: usize = 13;
const AT_EGID: usize = 14;
const AT_CLKTCK: usize = 17;
const AT_SECURE: usize = 23;
const AT_RANDOM: usize = 25;
const AT_HWCAP: usize = 16;

struct DownwardStack<T> {
    sp: usize,
    #[cfg(feature = "bounds-checks")]
    buffer_bottom: usize,
    #[cfg(feature = "bounds-checks")]
    buffer_top: usize,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> DownwardStack<T> {
    #[inline]
    fn new(stack_top: usize, _buffer_bottom: usize) -> Self {
        Self {
            sp: stack_top,
            #[cfg(feature = "bounds-checks")]
            buffer_top: stack_top,
            #[cfg(feature = "bounds-checks")]
            buffer_bottom: _buffer_bottom,
            _phantom: core::marker::PhantomData,
        }
    }

    #[inline(always)]
    fn push(&mut self, val: T)
    where
        T: Copy,
    {
        self.sp -= core::mem::size_of::<T>();

        #[cfg(feature = "bounds-checks")]
        {
            if self.sp < self.buffer_bottom {
                #[cfg(feature = "debug")]
                debug::writeln!(
                    "Stack overflow! SP=0x{:x} below stack bottom=0x{:x}, top=0x{:x}",
                    self.sp,
                    self.buffer_bottom,
                    self.buffer_top
                );

                panic!(
                    "Stack overflow! SP=0x{:x} below stack bottom=0x{:x}, top=0x{:x}",
                    self.sp, self.buffer_bottom, self.buffer_top
                );
            }
        }

        unsafe {
            core::ptr::write(self.sp as *mut T, val);
        }
    }

    #[inline]
    fn sp(&self) -> usize {
        self.sp
    }
}

impl DownwardStack<usize> {
    /// Push raw bytes onto the stack, rounded up to `align` bytes.
    /// Returns a pointer (address) to the start of the bytes.
    #[inline(always)]
    #[cfg(feature = "backtrace")]
    fn push_bytes_aligned(&mut self, bytes: &[u8], align: usize) -> usize {
        debug_assert!(align.is_power_of_two());
        let len = bytes.len();
        let rounded = (len + (align - 1)) & !(align - 1);
        self.sp -= rounded;

        #[cfg(feature = "bounds-checks")]
        {
            if self.sp < self.buffer_bottom {
                #[cfg(feature = "debug")]
                debug::writeln!(
                    "Stack overflow! SP=0x{:x} below stack bottom=0x{:x}, top=0x{:x}",
                    self.sp,
                    self.buffer_bottom,
                    self.buffer_top
                );

                panic!(
                    "Stack overflow! SP=0x{:x} below stack bottom=0x{:x}, top=0x{:x}",
                    self.sp, self.buffer_bottom, self.buffer_top
                );
            }
        }

        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.sp as *mut u8, len);
            // Zero any padding bytes so the stack contents are deterministic.
            for i in len..rounded {
                core::ptr::write((self.sp + i) as *mut u8, 0);
            }
        }

        self.sp
    }
}

#[inline]
fn generate_random_bytes(entropy: &[u64]) -> (u64, u64) {
    let mut state = 0x123456789abcdef0u64;
    for &e in entropy {
        state ^= e;
        state = state.wrapping_mul(0x5851f42d4c957f2d);
        state ^= state >> 33;
    }

    let random_low = state;
    state = state.wrapping_mul(0x5851f42d4c957f2d);
    state ^= state >> 33;
    let random_high = state;

    (random_low, random_high)
}

/// The stack layout follows the System V ABI and Linux kernel conventions.
/// # Safety
/// Caller must ensure:
#[inline]
pub unsafe fn build_musl_stack(
    stack_top: usize,
    stack_bottom: usize,
    program_name: &'static [u8],
) -> usize {
    let mut ds = DownwardStack::<usize>::new(stack_top, stack_bottom);

    // Optional environment variables for musl's `__libc_start_main`:
    // it computes envp = argv + argc + 1.
    #[cfg(feature = "backtrace")]
    let rust_backtrace_ptr =
        ds.push_bytes_aligned(b"RUST_BACKTRACE=full\0", core::mem::align_of::<usize>());

    // In ZeroOS we run as a single static image with no dynamic loader; musl startup does not
    // require AT_PHDR/AT_PHNUM/AT_PHENT/AT_ENTRY for correctness, so we set them to 0.
    let (at_phdr, at_phent, at_phnum, at_entry) = (0usize, 0usize, 0usize, 0usize);

    // Prepare auxiliary vector entries
    let auxv_entries = [
        (AT_PHDR, at_phdr),
        (AT_PHENT, at_phent),
        (AT_PHNUM, at_phnum),
        (AT_ENTRY, at_entry),
        (AT_PAGESZ, 4096),
        (AT_CLKTCK, 100),
        (AT_HWCAP, 0),
        (AT_UID, 0),
        (AT_EUID, 0),
        (AT_GID, 0),
        (AT_EGID, 0),
        (AT_SECURE, 0),
        (AT_RANDOM, 0), // Will be replaced with actual pointer below
        (AT_NULL, 0),
    ];

    // Generate 16 bytes for AT_RANDOM (Linux kernel standard)
    // Musl's __init_ssp uses first sizeof(uintptr_t) bytes for stack canary

    let entropy = [stack_top as u64, 0xdeadbeef_cafebabe_u64];
    let (random_low, random_high) = generate_random_bytes(&entropy);

    // Memory layout after pushes (stack grows downward, lower addresses at bottom):

    #[cfg(target_pointer_width = "32")]
    {
        ds.push((random_high >> 32) as usize);
        ds.push(random_high as usize);
        ds.push((random_low >> 32) as usize);
        ds.push(random_low as usize);
    }
    #[cfg(target_pointer_width = "64")]
    {
        ds.push(random_high as usize);
        ds.push(random_low as usize);
    }

    let at_random_ptr = ds.sp();

    for &(key, val) in auxv_entries.iter().rev() {
        let eff_val = if key == AT_RANDOM { at_random_ptr } else { val };
        ds.push(eff_val);
        ds.push(key);
    }

    // envp terminator (always present)
    ds.push(0);
    // envp[0] (optional)
    #[cfg(feature = "backtrace")]
    ds.push(rust_backtrace_ptr);

    // argv terminator
    ds.push(0);
    ds.push(program_name.as_ptr() as usize);

    ds.push(1);

    stack_top - ds.sp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_musl_stack_alignment() {
        let stack_buffer = vec![0u8; 4096];
        let stack_top = (stack_buffer.as_ptr() as usize) + stack_buffer.len();

        let program_name = b"test\0";

        unsafe {
            let new_sp = build_musl_stack(stack_top, stack_top - 4096, program_name);

            assert_eq!(new_sp % 16, 0, "Stack pointer must be 16-byte aligned");

            assert!(new_sp < stack_top, "Stack pointer must move downward");
        }
    }

    #[test]
    fn test_build_musl_stack_argc_argv() {
        let stack_buffer = vec![0u8; 4096];
        let stack_top = (stack_buffer.as_ptr() as usize) + stack_buffer.len();

        let program_name = b"myprogram\0";

        unsafe {
            let new_sp = build_musl_stack(stack_top, stack_top - 4096, program_name);

            let argc_ptr = new_sp as *const usize;
            let argc = *argc_ptr;
            assert_eq!(argc, 1, "argc must be 1");

            let argv_ptr = (new_sp + core::mem::size_of::<usize>()) as *const *const u8;
            let argv0 = *argv_ptr;
            assert_eq!(
                argv0,
                program_name.as_ptr(),
                "argv[0] must point to program name"
            );

            let argv1_ptr = (new_sp + 2 * core::mem::size_of::<usize>()) as *const usize;
            let argv1 = *argv1_ptr;
            assert_eq!(argv1, 0, "argv[1] must be NULL");
        }
    }

    #[test]
    fn test_generate_random_bytes() {
        let entropy1 = [0x1234567890abcdef_u64, 0xfedcba0987654321_u64];
        let entropy2 = [0x1234567890abcdef_u64, 0xfedcba0987654321_u64];
        let entropy3 = [0x0000000000000000_u64, 0x0000000000000000_u64];

        let (low1, high1) = generate_random_bytes(&entropy1);
        let (low2, high2) = generate_random_bytes(&entropy2);
        let (low3, high3) = generate_random_bytes(&entropy3);

        assert_eq!(low1, low2);
        assert_eq!(high1, high2);

        assert_ne!(low1, low3);
        assert_ne!(high1, high3);
    }
}
