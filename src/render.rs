//! Rendering! Woooooo!
//!
//! This module implements a type [`Renderer`] and a trait [`Render`].
use std::fmt;

use crossterm::{
    cursor,
    style::{Attribute, ResetColor},
    terminal::{self, ClearType},
};

/// The [`Renderer`] type contains a buffer to be rendered to the screen. It handles scrolling based
/// on the cursor's position and will only write the lines that should be visible.
#[derive(Default)]
pub struct Renderer {
    buffer: String,
    /// The indexes of the first and last lines of the section we want to prioritise rendering.
    selected_item: (usize, usize),
    /// This field contains the starting line index from the buffer at the time of the previous
    /// show. Can also be thought of as a "target" starting line, which will be updated based on
    /// where the cursor is.
    start_line: usize,
}

/// Types implementing [`Render`] can write to the given [`Renderer`] and update its cursor
/// position.
pub trait Render {
    /// This function is used to render the Self to the given [`Renderer`], `r`. [`Renderer`]
    /// implements [`fmt::Write`](std::fmt::Write) so the natural way to do this is to use methods
    /// from `Write` to write to the Renderer's buffer.
    ///
    /// You should also use [`Renderer::insert_cursor`] right before writing any line that should
    /// be the cursor position.
    fn render(&self, r: &mut Renderer) -> fmt::Result;
}

impl fmt::Write for Renderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.buffer, "{s}")
    }
}

/// Truncates a string to the given `length`, ignoring ANSI escape sequences.
fn truncate_ansi(s: &str, length: usize) -> &str {
    struct Performer(usize);
    impl vte::Perform for Performer {
        fn print(&mut self, _c: char) {
            self.0 += 1;
        }
    }
    let mut performer = Performer(0);
    let mut parser = vte::Parser::new();
    let bytes = s.as_bytes().iter().enumerate();
    for (i, b) in bytes {
        parser.advance(&mut performer, *b);
        if performer.0 > length {
            return &s[0..i];
        }
    }
    s
}

impl Renderer {
    /// Insert the cursor at the next line.
    pub fn insert_cursor(&mut self) {
        let next_line = self.buffer.lines().count();
        self.selected_item = (next_line, next_line);
    }

    /// Insert after the end of the selected item. For if your "cursor" is more than one line long.
    ///
    /// E.g.
    /// ```
    /// r.insert_cursor();
    /// writeln!(r, "multi\n line\n item");
    /// r.insert_item_end();
    /// ```
    pub fn insert_item_end(&mut self) {
        self.selected_item.1 = self.buffer.lines().count() - 1;
    }

    /// Render to stdout and clear the buffer.
    pub fn show_and_clear(
        &mut self,
        width: usize,
        height: usize,
        lookahead: usize,
        truncate: bool,
    ) {
        print!(
            "{}{}",
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        );

        let (cursor_start_idx, cursor_end_idx) = self.selected_item;
        let count_lines = self.buffer.lines().count();
        if count_lines < height {
            print!("{}", self.buffer);
        } else {
            // Distance to end of buffer is less than the terminal height.
            if count_lines - self.start_line < height {
                self.start_line = count_lines - height;
            }
            // Selection bigger than the terminal height.
            else if cursor_end_idx - cursor_start_idx > height {
                self.start_line = cursor_start_idx;
            }
            // Going down.
            else if cursor_end_idx + lookahead >= self.start_line + height {
                self.start_line = (cursor_end_idx + lookahead).min(count_lines - 1) - (height - 1);
            }
            // Going up.
            else if cursor_start_idx.saturating_sub(lookahead) < self.start_line {
                self.start_line = cursor_start_idx.saturating_sub(lookahead);
            }

            if truncate {
                for l in self
                    .buffer
                    .lines()
                    .skip(self.start_line)
                    .take(height)
                    .map(|l| truncate_ansi(l, width))
                {
                    print!("\r\n{l}{}{}", Attribute::Reset, ResetColor);
                }
            } else {
                for l in self.buffer.lines().skip(self.start_line).take(height) {
                    print!("\r\n{l}");
                }
            }
        }
        print!("{}", Attribute::Reset);
        self.buffer.clear();
    }
}
