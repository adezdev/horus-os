// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reads text back from screenshots, using the kernel's own console font.
//!
//! The kernel draws text with exact pixels from `kernel/assets/fonts/`, so a
//! screenshot cell can be matched against the font's glyphs exactly. Tests
//! then compare whole strings instead of looking for colored pixels.

use std::collections::HashMap;
use std::fs;

use crate::{Result, bail, root};

pub const WIDTH: usize = 8;
pub const HEIGHT: usize = 16;

/// Marks a cell that isn't a glyph in the expected colors.
pub const UNREADABLE: char = '\u{FFFD}';

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];
const PSF1_MODE_512: u8 = 0x01;
const PSF1_MODE_HAS_TABLE: u8 = 0x02;
const PSF1_SEPARATOR: u16 = 0xFFFF;
const PSF1_START_SEQUENCE: u16 = 0xFFFE;

/// Glyph bitmaps of printable ASCII, keyed by bitmap.
pub struct Font {
    chars: HashMap<[u8; HEIGHT], char>,
}

impl Font {
    /// Loads the kernel console font (PSF1 with a Unicode table).
    pub fn load() -> Result<Font> {
        let path = root().join("kernel/assets/fonts/spleen-8x16.psfu");
        let data = fs::read(&path)?;
        if data.len() < 4 || data[..2] != PSF1_MAGIC || usize::from(data[3]) != HEIGHT {
            bail!("{} is not an 8x16 PSF1 font", path.display());
        }
        let mode = data[2];
        if mode & PSF1_MODE_HAS_TABLE == 0 {
            bail!("{} has no Unicode table", path.display());
        }
        let count = if mode & PSF1_MODE_512 != 0 { 512 } else { 256 };
        let Some(bitmaps) = data.get(4..4 + count * HEIGHT) else {
            bail!("{} is truncated", path.display());
        };
        let table = &data[4 + count * HEIGHT..];

        let mut chars = HashMap::new();
        let mut glyph = 0;
        let mut in_sequence = false;
        for &pair in table.as_chunks::<2>().0 {
            if glyph >= count {
                break;
            }
            match u16::from_le_bytes(pair) {
                PSF1_SEPARATOR => {
                    glyph += 1;
                    in_sequence = false;
                }
                PSF1_START_SEQUENCE => in_sequence = true,
                code_point if !in_sequence && (0x20..0x7F).contains(&code_point) => {
                    let mut bitmap = [0; HEIGHT];
                    bitmap.copy_from_slice(&bitmaps[glyph * HEIGHT..(glyph + 1) * HEIGHT]);
                    chars.entry(bitmap).or_insert(char::from(code_point as u8));
                }
                _ => {}
            }
        }
        if chars.len() != 0x7F - 0x20 {
            bail!("{} lacks glyphs for printable ASCII", path.display());
        }
        Ok(Font { chars })
    }

    /// Reads up to `cols` characters starting at pixel `(x, y)`, where
    /// `pixel(x, y)` returns `0xRRGGBB`. A cell whose pixels aren't all `fg`
    /// or `bg`, or that matches no glyph, reads as [`UNREADABLE`]. Trailing
    /// spaces are removed.
    pub fn read(
        &self,
        pixel: impl Fn(usize, usize) -> u32,
        (x, y): (usize, usize),
        cols: usize,
        (fg, bg): (u32, u32),
    ) -> String {
        let mut text = String::new();
        for col in 0..cols {
            let cell_x = x + col * WIDTH;
            let mut bitmap = [0; HEIGHT];
            let mut readable = true;
            for (dy, row) in bitmap.iter_mut().enumerate() {
                for dx in 0..WIDTH {
                    match pixel(cell_x + dx, y + dy) {
                        p if p == fg => *row |= 0x80 >> dx,
                        p if p == bg => {}
                        _ => readable = false,
                    }
                }
            }
            let c = match readable {
                true => self.chars.get(&bitmap).copied().unwrap_or(UNREADABLE),
                false => UNREADABLE,
            };
            text.push(c);
        }
        text.truncate(text.trim_end_matches(' ').len());
        text
    }
}
