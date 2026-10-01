// SPDX-License-Identifier: MIT OR Apache-2.0

//! x86_64 primitives.

pub mod cpu;

use core::arch::asm;

/// QEMU's `debugcon` device (`-debugcon`), also used by Bochs.
const DEBUGCON_PORT: u16 = 0xE9;

/// Writes `value` to an I/O port.
///
/// # Safety
///
/// Writing to an I/O port can reconfigure hardware. The caller must ensure
/// that `port` belongs to a device for which `value` is a valid write.
unsafe fn outb(port: u16, value: u8) {
    // SAFETY: the caller guarantees the write is valid for this port; `out`
    // touches no memory and leaves the stack and flags alone.
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

/// Writes one byte to the QEMU debug console.
pub fn debugcon_write(byte: u8) {
    // SAFETY: port 0xE9 is the emulator debug console. On the target laptop
    // nothing decodes this port, so the write is discarded.
    unsafe { outb(DEBUGCON_PORT, byte) }
}

/// Stops this CPU for good: interrupts off, then halt.
pub fn halt_forever() -> ! {
    loop {
        // SAFETY: `cli; hlt` only stops this CPU; with interrupts masked it
        // stays stopped (an NMI resumes the loop, which halts again).
        unsafe { asm!("cli", "hlt", options(nomem, nostack)) }
    }
}
