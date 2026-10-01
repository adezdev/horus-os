// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drawing on the framebuffer that Limine set up through UEFI GOP.

use limine::framebuffer::FRAMEBUFFER_RGB;

use crate::boot;
use crate::font::{self, Glyph};

/// A 24-bit color as `0xRRGGBB`.
#[derive(Clone, Copy)]
pub struct Rgb(pub u32);

/// A color already converted to a surface's pixel format.
#[derive(Clone, Copy)]
pub struct Pixel(u32);

impl Pixel {
    /// All bits clear; black in every RGB format.
    pub const ZERO: Pixel = Pixel(0);
}

/// A framebuffer the kernel can draw on: 32 bits per pixel, RGB.
#[derive(Clone, Copy)]
pub struct Surface {
    /// Virtual address of the top-left pixel.
    base: usize,
    pub width: usize,
    pub height: usize,
    /// Bytes per row.
    pitch: usize,
    /// `(size, shift)` of the red, green, and blue channels.
    channels: [(u8, u8); 3],
}

impl Surface {
    /// The first framebuffer from Limine, if it uses a supported format.
    pub fn primary() -> Option<Surface> {
        let fb = boot::FRAMEBUFFER.response()?.framebuffers().first()?;
        if fb.memory_model != FRAMEBUFFER_RGB || fb.bpp != 32 {
            return None;
        }
        Some(Surface {
            base: fb.address() as usize,
            width: usize::try_from(fb.width).ok()?,
            height: usize::try_from(fb.height).ok()?,
            pitch: usize::try_from(fb.pitch).ok()?,
            channels: [
                (fb.red_mask_size, fb.red_mask_shift),
                (fb.green_mask_size, fb.green_mask_shift),
                (fb.blue_mask_size, fb.blue_mask_shift),
            ],
        })
    }

    /// Converts `color` to this surface's pixel format.
    pub fn encode(&self, Rgb(rgb): Rgb) -> Pixel {
        let [red, green, blue] = self.channels;
        let channel = |value: u32, (size, shift): (u8, u8)| {
            let scaled = if size >= 8 {
                value << (size - 8)
            } else {
                value >> (8 - size)
            };
            scaled << shift
        };
        Pixel(
            channel((rgb >> 16) & 0xff, red)
                | channel((rgb >> 8) & 0xff, green)
                | channel(rgb & 0xff, blue),
        )
    }

    /// Writes one pixel. The caller ensures `x < width` and `y < height`.
    fn put(&self, x: usize, y: usize, Pixel(value): Pixel) {
        debug_assert!(x < self.width && y < self.height);
        let address = self.base + y * self.pitch + x * 4;
        // SAFETY: Limine maps `pitch * height` bytes of framebuffer at `base`.
        // With `x < width` and `y < height`, the 4-byte pixel lies inside
        // that range and is 4-byte aligned. Volatile: this is device memory.
        unsafe { (address as *mut u32).write_volatile(value) }
    }

    /// Fills a rectangle, clipped to the surface.
    pub fn fill_rect(&self, x: usize, y: usize, width: usize, height: usize, pixel: Pixel) {
        let x_end = x.saturating_add(width).min(self.width);
        let y_end = y.saturating_add(height).min(self.height);
        for py in y.min(y_end)..y_end {
            for px in x.min(x_end)..x_end {
                self.put(px, py, pixel);
            }
        }
    }

    /// Fills the whole surface.
    pub fn fill(&self, pixel: Pixel) {
        self.fill_rect(0, 0, self.width, self.height, pixel);
    }

    /// Draws a glyph cell with its top-left corner at `(x, y)`, clipped to
    /// the surface.
    pub fn draw_glyph(&self, x: usize, y: usize, glyph: &Glyph, fg: Pixel, bg: Pixel) {
        for (dy, &bits) in glyph.iter().enumerate() {
            let py = y + dy;
            if py >= self.height {
                break;
            }
            for dx in 0..font::WIDTH {
                let px = x + dx;
                if px >= self.width {
                    break;
                }
                let lit = bits & (0x80 >> dx) != 0;
                self.put(px, py, if lit { fg } else { bg });
            }
        }
    }
}
