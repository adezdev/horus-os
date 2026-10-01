// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal access to the Limine-provided framebuffer.
//!
//! Text rendering and the boot logo come later in v0.1; for now the kernel
//! can only fill the screen with a color.

use limine::framebuffer::{FRAMEBUFFER_RGB, Framebuffer};

use crate::boot;

/// A 24-bit color as `0xRRGGBB`.
#[derive(Clone, Copy)]
pub struct Rgb(pub u32);

fn primary() -> Option<&'static Framebuffer> {
    boot::FRAMEBUFFER
        .response()
        .and_then(|r| r.framebuffers().first().copied())
}

/// Converts a color to the framebuffer's pixel format.
fn encode(fb: &Framebuffer, Rgb(rgb): Rgb) -> u32 {
    let channel = |value: u32, size: u8, shift: u8| {
        let scaled = if size >= 8 {
            value << (size - 8)
        } else {
            value >> (8 - size)
        };
        scaled << shift
    };
    channel((rgb >> 16) & 0xff, fb.red_mask_size, fb.red_mask_shift)
        | channel((rgb >> 8) & 0xff, fb.green_mask_size, fb.green_mask_shift)
        | channel(rgb & 0xff, fb.blue_mask_size, fb.blue_mask_shift)
}

/// Fills the whole screen with `color`. Does nothing without a supported
/// (32-bit RGB) framebuffer.
pub fn fill(color: Rgb) {
    let Some(fb) = primary() else { return };
    if fb.memory_model != FRAMEBUFFER_RGB || fb.bpp != 32 {
        return;
    }
    let pixel = encode(fb, color);
    let base = fb.address().cast::<u8>();
    for y in 0..fb.height as usize {
        // SAFETY: Limine maps the framebuffer for us; each row starts at
        // `y * pitch` and holds `width` 32-bit pixels, so every write stays
        // within the `pitch * height` bytes of the framebuffer.
        let row = unsafe { base.add(y * fb.pitch as usize).cast::<u32>() };
        for x in 0..fb.width as usize {
            // SAFETY: `x < width`, see above. Volatile because this is
            // device memory.
            unsafe { row.add(x).write_volatile(pixel) };
        }
    }
}
