// SPDX-License-Identifier: MIT OR Apache-2.0

//! Boot stage colors.
//!
//! The laptop has no serial port, so each boot stage paints the whole screen
//! a distinct color. If boot hangs on real hardware, the color shows how far
//! it got. The table is mirrored in `docs/architecture/boot.md`.

use crate::framebuffer::{self, Rgb};

/// A boot stage, in the order they're reached.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// Limine refused the requested protocol base revision.
    Unsupported,
    /// The kernel entry point is running.
    Entry,
    /// Boot information from Limine has been read.
    BootInfo,
    /// Early boot finished.
    Ready,
    /// The kernel panicked.
    Panic,
}

impl Stage {
    const fn color(self) -> Rgb {
        match self {
            Stage::Unsupported => Rgb(0x8e44ad), // purple
            Stage::Entry => Rgb(0x1d3557),       // dark blue
            Stage::BootInfo => Rgb(0x2a9d8f),    // teal
            Stage::Ready => Rgb(0xd4a63a),       // gold
            Stage::Panic => Rgb(0xc0392b),       // red
        }
    }
}

/// Marks the start of `stage` on screen.
pub fn enter(stage: Stage) {
    framebuffer::fill(stage.color());
}
