pub struct DebugWriter;

impl core::fmt::Write for DebugWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        debug_write(s.as_bytes());
        Ok(())
    }
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        crate::putchar(c as u8);
        Ok(())
    }
}

fn debug_write(buf: &[u8]) {
    for &b in buf {
        crate::putchar(b);
    }
}
