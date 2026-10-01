// SPDX-License-Identifier: MIT OR Apache-2.0

//! The kernel console font: Spleen 8×16 (BSD-2-Clause), stored as PSF1.
//!
//! The font file is parsed at compile time, so a malformed or unsuitable
//! font fails the build instead of the boot. See `kernel/assets/fonts/`.

/// Glyph width in pixels.
pub const WIDTH: usize = 8;
/// Glyph height in pixels.
pub const HEIGHT: usize = 16;

/// One glyph: a row per byte, most significant bit leftmost.
pub type Glyph = [u8; HEIGHT];

const PSF1_MAGIC: [u8; 2] = [0x36, 0x04];
const PSF1_MODE_512: u8 = 0x01;
const PSF1_MODE_HAS_TABLE: u8 = 0x02;
const PSF1_SEPARATOR: u16 = 0xFFFF;
const PSF1_START_SEQUENCE: u16 = 0xFFFE;
const REPLACEMENT_CHARACTER: u16 = 0xFFFD;
const NO_GLYPH: u16 = u16::MAX;

/// A bitmap font with glyphs for Latin-1 and a replacement glyph for the rest.
pub struct Font {
    glyphs: &'static [Glyph],
    latin1: [u16; 256],
    replacement: u16,
}

/// The console font.
pub static FONT: Font = Font::from_psf1(include_bytes!("../assets/fonts/spleen-8x16.psfu"));

impl Font {
    const fn from_psf1(data: &'static [u8]) -> Font {
        assert!(
            data.len() >= 4 && data[0] == PSF1_MAGIC[0] && data[1] == PSF1_MAGIC[1],
            "console font is not a PSF1 file"
        );
        let mode = data[2];
        assert!(
            data[3] as usize == HEIGHT,
            "console font must be 16 pixels tall"
        );
        assert!(
            mode & PSF1_MODE_HAS_TABLE != 0,
            "console font needs a Unicode table"
        );
        let count = if mode & PSF1_MODE_512 != 0 { 512 } else { 256 };

        let (_, body) = data.split_at(4);
        assert!(body.len() >= count * HEIGHT, "console font is truncated");
        let (bitmaps, table) = body.split_at(count * HEIGHT);
        let (glyphs, _) = bitmaps.as_chunks::<HEIGHT>();

        // The table lists the code points of each glyph in turn, separated by
        // 0xFFFF. Code point sequences (after 0xFFFE) are skipped.
        let mut latin1 = [NO_GLYPH; 256];
        let mut replacement = NO_GLYPH;
        let mut glyph = 0;
        let mut in_sequence = false;
        let mut i = 0;
        while i + 1 < table.len() && glyph < count {
            let code_point = u16::from_le_bytes([table[i], table[i + 1]]);
            i += 2;
            match code_point {
                PSF1_SEPARATOR => {
                    glyph += 1;
                    in_sequence = false;
                }
                PSF1_START_SEQUENCE => in_sequence = true,
                _ if in_sequence => {}
                REPLACEMENT_CHARACTER if replacement == NO_GLYPH => replacement = glyph as u16,
                cp if cp < 256 && latin1[cp as usize] == NO_GLYPH => {
                    latin1[cp as usize] = glyph as u16;
                }
                _ => {}
            }
        }
        assert!(
            replacement != NO_GLYPH,
            "console font has no U+FFFD replacement glyph"
        );
        Font {
            glyphs,
            latin1,
            replacement,
        }
    }

    /// The glyph for `c`, or the replacement glyph if the font lacks it.
    pub fn glyph(&self, c: char) -> &Glyph {
        let index = match self.latin1.get(c as usize) {
            Some(&index) if index != NO_GLYPH => index,
            _ => self.replacement,
        };
        &self.glyphs[usize::from(index)]
    }
}
