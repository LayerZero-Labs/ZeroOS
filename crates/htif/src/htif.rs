// HTIF implementation for Spike/fesvr-style hosts.
// Packet format (by convention; must match host):
//   - bits 63..56: device (8 bits)
//   - bits 55..48: command (8 bits)
//   - bits 47..0 : payload (48 bits)
//
// Assumptions (contract with host):
//   - Only one outstanding packet in `tohost` at a time.
//   - Host clears `tohost` to 0 after processing a command (ack).
//   - Host writes responses to `fromhost`; we clear `fromhost` to 0 after consuming.

use core::arch::asm;

#[repr(align(64))]
pub struct Aligned64(u64);

#[link_section = ".tohost"]
#[used]
#[no_mangle]
pub static mut tohost: Aligned64 = Aligned64(0);

#[link_section = ".fromhost"]
#[used]
#[no_mangle]
pub static mut fromhost: Aligned64 = Aligned64(0);

pub const DEV_SYSCALL: u64 = 0x00;
pub const DEV_CONSOLE: u64 = 0x01;

pub const CMD_SYSCALL: u64 = 0x00;
pub const CMD_GETCHAR: u64 = 0x00;
pub const CMD_PUTCHAR: u64 = 0x01;

// Packet fields
const DEV_SHIFT: u64 = 56;
const CMD_SHIFT: u64 = 48;
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

#[inline(always)]
fn encode_packet(device: u64, cmd: u64, payload: u64) -> u64 {
    ((device & 0xFF) << DEV_SHIFT) | ((cmd & 0xFF) << CMD_SHIFT) | (payload & PAYLOAD_MASK)
}

#[inline(always)]
fn decode_packet(word: u64) -> (u64, u64, u64) {
    let device = (word >> DEV_SHIFT) & 0xFF;
    let cmd = (word >> CMD_SHIFT) & 0xFF;
    let payload = word & PAYLOAD_MASK;
    (device, cmd, payload)
}

#[inline(always)]
fn read_tohost() -> u64 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(tohost.0)) }
}

#[inline(always)]
fn write_tohost(word: u64) {
    unsafe { core::ptr::write_volatile(core::ptr::addr_of_mut!(tohost.0), word) }
}

#[inline(always)]
fn read_fromhost() -> u64 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(fromhost.0)) }
}

#[inline(always)]
fn write_fromhost(word: u64) {
    unsafe { core::ptr::write_volatile(core::ptr::addr_of_mut!(fromhost.0), word) }
}

#[inline(always)]
fn wait_tohost_idle() {
    while read_tohost() != 0 {
        core::hint::spin_loop();
    }
}

#[inline(always)]
fn wait_tohost_ack() {
    while read_tohost() != 0 {
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn send_packet(device: u64, cmd: u64, payload: u64) {
    wait_tohost_idle();
    write_tohost(encode_packet(device, cmd, payload));
    wait_tohost_ack();
}

#[inline(always)]
pub fn recv_packet() -> (u64, u64, u64) {
    loop {
        let word = read_fromhost();
        if word != 0 {
            let pkt = decode_packet(word);
            write_fromhost(0);
            return pkt;
        }
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn try_recv_packet() -> Option<(u64, u64, u64)> {
    let word = read_fromhost();
    if word == 0 {
        return None;
    }
    let pkt = decode_packet(word);
    write_fromhost(0);
    Some(pkt)
}

pub fn exit(code: u32) -> ! {
    let payload = ((code as u64) << 1) | 1;
    send_packet(DEV_SYSCALL, CMD_SYSCALL, payload);
    loop {
        // Host will terminate the simulation; either spin or use wfi.
        unsafe { asm!("wfi") };
    }
}

pub fn putchar(ch: u8) {
    send_packet(DEV_CONSOLE, CMD_PUTCHAR, ch as u64);
}

pub fn getchar() -> u8 {
    send_packet(DEV_CONSOLE, CMD_GETCHAR, 0);

    loop {
        let (dev, cmd, payload) = recv_packet();
        if dev == DEV_CONSOLE && cmd == CMD_GETCHAR {
            return (payload & 0xFF) as u8;
        }
        // Ignore unrelated responses
    }
}

pub fn syscall(payload: u64) -> u64 {
    send_packet(DEV_SYSCALL, CMD_SYSCALL, payload);

    loop {
        let (dev, cmd, resp) = recv_packet();
        if dev == DEV_SYSCALL && cmd == CMD_SYSCALL {
            return resp;
        }
        // Ignore unrelated responses
    }
}
