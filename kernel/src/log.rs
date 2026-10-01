// SPDX-License-Identifier: MIT OR Apache-2.0

//! Kernel logging.
//!
//! Output goes to the QEMU debug console (I/O port `0xE9`) and, once it's
//! active, the framebuffer console. An in-memory log ring comes later.

use core::fmt::{self, Write};

use crate::{arch, console};

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

/// Like [`kprintln!`], but only to the debug console. For the panic path,
/// which must not depend on the framebuffer console.
macro_rules! kprintln_debugcon {
    ($($arg:tt)*) => {
        $crate::log::print_debugcon(format_args!("{}\n", format_args!($($arg)*)))
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
    print_debugcon(args);
    console::print(args);
}

/// Backend of [`kprintln_debugcon!`]; use the macro instead.
pub fn print_debugcon(args: fmt::Arguments) {
    // Writing to the debug console cannot fail.
    let _ = DebugCon.write_fmt(args);
}
