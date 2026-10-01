// SPDX-License-Identifier: MIT OR Apache-2.0

//! Kernel logging.
//!
//! For now, output goes only to the QEMU debug console (I/O port `0xE9`).
//! The framebuffer console and the in-memory log ring come later in v0.1.

use core::fmt::{self, Write};

use crate::arch;

/// Writes formatted kernel output to every log sink.
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::log::print(format_args!($($arg)*))
    };
}

/// Like [`kprint!`], followed by a newline.
macro_rules! kprintln {
    () => {
        kprint!("\n")
    };
    ($($arg:tt)*) => {
        kprint!("{}\n", format_args!($($arg)*))
    };
}

struct DebugCon;

impl Write for DebugCon {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            arch::debugcon_write(byte);
        }
        Ok(())
    }
}

/// Backend of [`kprint!`]; use the macros instead.
pub fn print(args: fmt::Arguments) {
    // Writing to the debug console cannot fail.
    let _ = DebugCon.write_fmt(args);
}
