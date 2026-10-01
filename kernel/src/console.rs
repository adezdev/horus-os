// SPDX-License-Identifier: MIT OR Apache-2.0

//! Framebuffer text console: the kernel log on screen.
//!
//! Text is kept in a grid of cells, so scrolling redraws from memory instead
//! of reading back framebuffer memory, which is very slow. A thin stripe at
//! the top of the screen shows the boot stage (see [`crate::stage`]).

use core::fmt::{self, Write};

use crate::font::{self, FONT};
use crate::framebuffer::{Pixel, Rgb, Surface};
use crate::sync::SpinLock;

/// Height of the boot stage stripe at the top of the screen.
const STRIPE_HEIGHT: usize = 6;
/// Space around the text area.
const MARGIN: usize = 8;
/// First pixel row of text.
const TEXT_TOP: usize = STRIPE_HEIGHT + MARGIN;
const MAX_COLS: usize = 256;
const MAX_ROWS: usize = 128;
const TAB_WIDTH: usize = 4;
/// An empty cell, drawn as a space.
const BLANK: char = '\0';

/// Matches the default theme's background (`docs/architecture/desktop.md`).
pub const BACKGROUND: Rgb = Rgb(0x0f1115);
/// Matches the default theme's text color.
pub const FOREGROUND: Rgb = Rgb(0xe6e6e6);

static CONSOLE: SpinLock<Console> = SpinLock::new(Console::INACTIVE);

struct Console {
    surface: Option<Surface>,
    cols: usize,
    rows: usize,
    col: usize,
    row: usize,
    fg: Pixel,
    bg: Pixel,
    cells: [[char; MAX_COLS]; MAX_ROWS],
}

impl Console {
    const INACTIVE: Console = Console {
        surface: None,
        cols: 0,
        rows: 0,
        col: 0,
        row: 0,
        fg: Pixel::ZERO,
        bg: Pixel::ZERO,
        cells: [[BLANK; MAX_COLS]; MAX_ROWS],
    };

    fn activate(&mut self, surface: Surface) -> bool {
        let cols = (surface.width.saturating_sub(2 * MARGIN) / font::WIDTH).min(MAX_COLS);
        let rows = (surface.height.saturating_sub(TEXT_TOP + MARGIN) / font::HEIGHT).min(MAX_ROWS);
        if cols == 0 || rows == 0 {
            return false;
        }
        self.cols = cols;
        self.rows = rows;
        self.col = 0;
        self.row = 0;
        self.fg = surface.encode(FOREGROUND);
        self.bg = surface.encode(BACKGROUND);
        self.cells = [[BLANK; MAX_COLS]; MAX_ROWS];
        surface.fill(self.bg);
        self.surface = Some(surface);
        true
    }

    fn put_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.col = 0,
            '\t' => {
                let next_stop = (self.col / TAB_WIDTH + 1) * TAB_WIDTH;
                while self.col < next_stop.min(self.cols) {
                    self.put_char(' ');
                }
            }
            c => {
                if self.col >= self.cols {
                    self.newline();
                }
                self.cells[self.row][self.col] = c;
                self.draw_cell(self.row, self.col);
                self.col += 1;
            }
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < self.rows {
            self.row += 1;
        } else {
            self.cells.copy_within(1..self.rows, 0);
            self.cells[self.rows - 1] = [BLANK; MAX_COLS];
            self.redraw();
        }
    }

    fn redraw(&self) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.draw_cell(row, col);
            }
        }
    }

    fn draw_cell(&self, row: usize, col: usize) {
        let Some(surface) = &self.surface else { return };
        let c = match self.cells[row][col] {
            BLANK => ' ',
            c => c,
        };
        surface.draw_glyph(
            MARGIN + col * font::WIDTH,
            TEXT_TOP + row * font::HEIGHT,
            FONT.glyph(c),
            self.fg,
            self.bg,
        );
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.surface.is_some() {
            s.chars().for_each(|c| self.put_char(c));
        }
        Ok(())
    }
}

/// Clears the screen and starts showing the kernel log. Returns whether a
/// usable framebuffer was found.
pub fn init() -> bool {
    match (CONSOLE.try_lock(), Surface::primary()) {
        (Some(mut console), Some(surface)) => console.activate(surface),
        _ => false,
    }
}

/// Prints to the console, if it's active. Skipped if the console is already
/// in use further up the stack (e.g. a panic while printing).
pub fn print(args: fmt::Arguments) {
    if let Some(mut console) = CONSOLE.try_lock() {
        // Writing to the console cannot fail.
        let _ = console.write_fmt(args);
    }
}

/// Paints the boot stage stripe. Returns false if the console isn't active.
pub fn draw_stripe(color: Rgb) -> bool {
    let Some(console) = CONSOLE.try_lock() else {
        return false;
    };
    let Some(surface) = &console.surface else {
        return false;
    };
    surface.fill_rect(0, 0, surface.width, STRIPE_HEIGHT, surface.encode(color));
    true
}
