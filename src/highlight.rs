use std::iter;

use itertools::Itertools;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

// Diff styles
// TODO: use existing colors from configuration?
const MARKER_NONE: Style = Style {
    foreground: Color::WHITE,
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};

const MARKER_ADDED: Style = Style {
    foreground: Color {
        r: 0x78,
        g: 0xde,
        b: 0x0c,
        a: 0xff,
    },
    background: Color {
        r: 0x0a,
        g: 0x28,
        b: 0x00,
        a: 0xff,
    },
    font_style: FontStyle::empty(),
};

const MARKER_REMOVED: Style = Style {
    foreground: Color {
        r: 0xd3,
        g: 0x2e,
        b: 0x09,
        a: 0xff,
    },
    background: Color {
        r: 0x3f,
        g: 0x0e,
        b: 0x00,
        a: 0xff,
    },
    font_style: FontStyle::empty(),
};

#[derive(Debug)]
pub struct SyntaxHighlight {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlight {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            // TODO: theme configuration
            theme: ThemeSet::load_defaults().themes["base16-eighties.dark"].clone(),
        }
    }

    pub fn get_syntax(&self, path: &str) -> &SyntaxReference {
        // TODO: probably better to use std::path?
        let file_ext = path.rsplit('.').next().unwrap_or("");
        self.syntax_set
            .find_syntax_by_extension(file_ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }
}

// TODO: just a workaround so that Status::default() still works
impl Default for SyntaxHighlight {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffStatus {
    Unchanged,
    Added,
    Removed,
}

pub fn highlight_hunk(hunk: &str, highlight: &SyntaxHighlight, syntax: &SyntaxReference) -> String {
    let mut highlighter = HighlightLines::new(syntax, &highlight.theme);

    if let Some(hunk) = try_highlight_single_line(hunk, highlight, syntax) {
        return hunk;
    }
    
    // syntax highlight each line
    hunk.lines()
        .map(|line| {
            let Some(diff_char) = line.chars().next() else {
                return line.to_owned();
            };

            let diff_char_len = diff_char.len_utf8();

            // add marker and one space
            let (marker, diff_line, status) = match diff_char {
                '+' => (
                    (MARKER_ADDED, "\u{258c} "),
                    &line[diff_char_len..],
                    DiffStatus::Added,
                ),
                '-' => (
                    (MARKER_REMOVED, "\u{258c} "),
                    &line[diff_char_len..],
                    DiffStatus::Removed,
                ),
                _ => ((MARKER_NONE, " "), line, DiffStatus::Unchanged),
            };

            let Ok(ranges) = highlighter.highlight_line(diff_line, &highlight.syntax_set) else {
                // Syntax highlighting failed, fallback to no highlighting
                // TODO: propagate error?
                return diff_line.to_owned();
            };

            let mut ranges: Vec<_> = iter::once(marker).chain(ranges).collect();

            match status {
                DiffStatus::Unchanged => (),
                DiffStatus::Added => {
                    for r in &mut ranges {
                        r.0.background = MARKER_ADDED.background;
                    }
                }
                DiffStatus::Removed => {
                    for r in &mut ranges {
                        r.0.background = MARKER_REMOVED.background;
                    }
                }
            }

            as_24_bit_terminal_escaped(&ranges, status != DiffStatus::Unchanged)
        })
        .join("\n")
}

fn try_highlight_single_line(hunk: &str, highlight: &SyntaxHighlight, syntax: &SyntaxReference) -> Option<String> {
    // TODO: attempt highlighting single line changes with strong inline highlight

    // - foo(bar, baz);
    //            --- strong background highlight (red)
    // + foo(bar, BAZ);
    //            --- strong background highlight (green)
    
    None
}
