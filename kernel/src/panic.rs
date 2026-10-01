// SPDX-License-Identifier: MIT OR Apache-2.0

//! Kernel panic handler and panic screen.

use core::fmt::{self, Write};
use core::panic::PanicInfo;

use crate::arch;
use crate::font::{self, FONT};
use crate::framebuffer::{Pixel, Rgb, Surface};
use crate::stage::Stage;

const MARGIN: usize = 24;
const TEXT: Rgb = Rgb(0xffffff);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // The debug console first: it can't fail, even if drawing does.
    match info.location() {
        Some(location) => kprintln_debugcon!("horus: PANIC at {location}: {}", info.message()),
        None => kprintln_debugcon!("horus: PANIC: {}", info.message()),
    }
    if let Some(surface) = Surface::primary() {
        show(&surface, info);
    }
    arch::halt_forever();
}

/// Draws the panic screen. Doesn't use the console, which may be the code
/// that panicked.
fn show(surface: &Surface, info: &PanicInfo) {
    let bg = surface.encode(Stage::Panic.color());
    surface.fill(bg);
    let mut out = PanicWriter {
        surface,
        fg: surface.encode(TEXT),
        bg,
        x: MARGIN,
        y: MARGIN,
    };
    // Writing to the screen cannot fail.
    let _ = writeln!(out, "HORUS KERNEL PANIC\n");
    let _ = writeln!(out, "{}\n", info.message());
    if let Some(location) = info.location() {
        let _ = writeln!(out, "at {location}\n");
    }
    let _ = writeln!(out, "kernel {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(
        out,
        "\nThe system has stopped. Take a photo of this screen, then reboot."
    );
}

/// Writes text onto the panic screen, wrapping at the right margin. Text
/// past the bottom of the screen is dropped.
struct PanicWriter<'a> {
    surface: &'a Surface,
    fg: Pixel,
    bg: Pixel,
    x: usize,
    y: usize,
}

impl PanicWriter<'_> {
    fn newline(&mut self) {
        self.x = MARGIN;
        self.y += font::HEIGHT;
    }
}

impl Write for PanicWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.newline();
                continue;
            }
            if self.x + font::WIDTH > self.surface.width.saturating_sub(MARGIN) {
                self.newline();
            }
            if self.y + font::HEIGHT > self.surface.height {
                break;
            }
            self.surface
                .draw_glyph(self.x, self.y, FONT.glyph(c), self.fg, self.bg);
            self.x += font::WIDTH;
        }
        Ok(())
    }
}
