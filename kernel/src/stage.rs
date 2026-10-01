// SPDX-License-Identifier: MIT OR Apache-2.0

//! Boot stage colors.
//!
//! The laptop has no serial port, so each boot stage shows a distinct color:
//! the whole screen until the console starts, then a stripe at the top of
//! the console. If boot hangs on real hardware, the color shows how far it
//! got. The table is mirrored in `docs/architecture/boot.md`.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::console;
use crate::framebuffer::{Rgb, Surface};

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
    pub const fn color(self) -> Rgb {
        match self {
            Stage::Unsupported => Rgb(0x8e44ad), // purple
            Stage::Entry => Rgb(0x1d3557),       // dark blue
            Stage::BootInfo => Rgb(0x2a9d8f),    // teal
            Stage::Ready => Rgb(0xd4a63a),       // gold
            Stage::Panic => Rgb(0xc0392b),       // red
        }
    }
}

static CURRENT: AtomicU32 = AtomicU32::new(0);

/// Marks the start of `stage` on screen.
pub fn enter(stage: Stage) {
    CURRENT.store(stage.color().0, Ordering::Relaxed);
    redraw();
}

/// Shows the current stage again, e.g. after the console cleared the screen.
pub fn redraw() {
    let color = Rgb(CURRENT.load(Ordering::Relaxed));
    if console::draw_stripe(color) {
        return;
    }
    if let Some(surface) = Surface::primary() {
        surface.fill(surface.encode(color));
    }
}
