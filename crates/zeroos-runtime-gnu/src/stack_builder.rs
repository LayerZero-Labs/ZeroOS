/// # Safety
/// `stack_top` must be a valid stack top address.
pub unsafe fn build_gnu_stack(
    stack_top: usize,
    _ehdr_start: usize,
    _program_name: &'static [u8],
) -> usize {
    // TODO: Implement actual glibc stack building
    // For now, just return stack_top - this will not work!
    stack_top
}
