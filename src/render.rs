//! Rendering! Woooooo!
use std::fmt;

use crossterm::{
    cursor,
    style::Attribute,
    terminal::{self, ClearType},
};

#[derive(Default)]
pub struct Renderer {
    pub buffer: String,
    cursor_idx: usize,
    start_line: usize,
}

pub trait Render {
    fn render(&self, r: &mut Renderer) -> fmt::Result;
}

impl fmt::Write for Renderer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write!(self.buffer, "{s}")
    }
}

impl Renderer {
    /// Insert the cursor at the next line.
    pub fn insert_cursor(&mut self) {
        self.cursor_idx = self.buffer.lines().count();
    }

    /// Render to stdout.
    pub fn show_and_clear(&mut self, height: usize) {
        print!(
            "{}{}",
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        );

        let count_lines = self.buffer.lines().count();
        if count_lines < height {
            print!("{}", self.buffer);
        } else {
            // Going up.
            if self.cursor_idx < self.start_line {
                self.start_line = self.cursor_idx;
            }
            // Going down.
            else if self.cursor_idx >= self.start_line + height {
                self.start_line = self.cursor_idx - height + 1;
            }
            for l in self.buffer.lines().skip(self.start_line).take(height) {
                print!("\r\n{l}");
            }
        }
        print!("{}", Attribute::Reset);
        self.buffer.clear();
    }
}
