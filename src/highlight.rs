use anyhow::{anyhow, Result};
use crossterm::style;
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

use crate::hunk::{DiffLineType, Hunk};

// Diff styles
// TODO(cptp): make colors configurable

const HEADER_MARKER_STYLE: Style = Style {
    foreground: Color::WHITE,
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};

const MARKER_ADDED: &str = "\u{258c}";
const MARKER_ADDED_STYLE: Style = Style {
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

const MARKER_REMOVED: &str = "\u{258c}";
const MARKER_REMOVED_STYLE: Style = Style {
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

// const BG_ADDED_STRONG: Color = Color {
//     r: 0x11,
//     g: 0x4f,
//     b: 0x05,
//     a: 0xff,
// };
// const BG_REMOVED_STRONG: Color = Color {
//     r: 0x77,
//     g: 0x23,
//     b: 0x05,
//     a: 0xff,
// };

/// Highlighter to use for
#[derive(Debug)]
pub enum DiffHighlighter {
    Simple {
        color_added: crossterm::style::Color,
        color_removed: crossterm::style::Color,
    },
    Syntect {
        syntax_set: SyntaxSet,
        theme: Box<Theme>,
    },
}

// "base16-eighties.dark"
impl DiffHighlighter {
    pub fn syntect(theme_name: &str) -> Result<Self> {
        let theme_set = ThemeSet::load_defaults();
        Ok(Self::Syntect {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: Box::new(
                theme_set
                    .themes
                    .get(theme_name)
                    .ok_or_else(|| anyhow!("Theme '{theme_name}' not found."))?
                    .clone(),
            ),
        })
    }

    // FIXME: it's a bit odd that this is passed as an extra option.
    // Maybe use a "specialized" enum instead that wraps it with the sytect options.
    /// Get file type specific syntax for syntect highlighting.
    ///
    /// Returns None if the `DiffHighlighter::Simple` is used.
    pub fn get_syntax(&self, path: &str) -> Option<&SyntaxReference> {
        match self {
            Self::Simple { .. } => None,
            Self::Syntect { syntax_set, .. } => {
                // TODO: probably better to use std::path?
                let file_ext = path.rsplit('.').next().unwrap_or("");
                Some(
                    syntax_set
                        .find_syntax_by_extension(file_ext)
                        .unwrap_or_else(|| syntax_set.find_syntax_plain_text()),
                )
            }
        }
    }
}

pub fn highlight_hunk(
    hunk: &Hunk,
    hl: &DiffHighlighter,
    syntax: Option<&SyntaxReference>,
) -> String {
    match hl {
        DiffHighlighter::Simple {
            color_added,
            color_removed,
        } => highlight_hunk_simple(hunk, *color_added, *color_removed),
        DiffHighlighter::Syntect { syntax_set, theme } => {
            highlight_hunk_syntect(hunk, syntax_set, theme, syntax.unwrap())
        }
    }
}

fn highlight_hunk_simple(
    hunk: &Hunk,
    color_added: crossterm::style::Color,
    color_removed: crossterm::style::Color,
) -> String {
    let mut buf = String::new();
    let color_added = style::SetForegroundColor(color_added).to_string();
    let color_removed = style::SetForegroundColor(color_removed).to_string();

    let (header_marker, header_content) = hunk.header();
    buf.push_str(header_marker);
    buf.push_str(header_content);
    buf.push('\n');

    for (line_type, line_content) in hunk.lines() {
        match line_type {
            DiffLineType::Unchanged => {
                buf.push(' ');
            }
            DiffLineType::Added => {
                buf.push_str(&color_added);
                buf.push('+');
            }
            DiffLineType::Removed => {
                buf.push_str(&color_removed);
                buf.push('-');
            }
        }
        buf.push_str(line_content);
        buf.push('\n');
    }

    // workaround: remove trailing line break
    buf.pop();
    buf.push_str("\x1b[0m");
    buf
}

fn highlight_hunk_syntect(
    hunk: &Hunk,
    syntax_set: &SyntaxSet,
    theme: &Theme,
    syntax: &SyntaxReference,
) -> String {
    // TODO: move somewhere else?
    let marker_added = as_24_bit_terminal_escaped(&[(MARKER_ADDED_STYLE, MARKER_ADDED)], true);
    let marker_removed =
        as_24_bit_terminal_escaped(&[(MARKER_REMOVED_STYLE, MARKER_REMOVED)], true);

    // separate highlighters for added and removed lines to keep the syntax intact
    let mut hl_add = HighlightLines::new(syntax, theme);
    let mut hl_rem = HighlightLines::new(syntax, theme);

    let mut buf = String::new();

    let (header_marker, header_content) = {
        let header = hunk.header();
        let header_content = hl_add
            .highlight_line(header.1, syntax_set)
            .and_then(|_| hl_rem.highlight_line(header.1, syntax_set));
        (
            as_24_bit_terminal_escaped(&[(HEADER_MARKER_STYLE, header.0)], false),
            header_content.map_or_else(
                |_| header.1.to_owned(),
                |content| as_24_bit_terminal_escaped(&content, false),
            ),
        )
    };

    buf.push_str(&header_marker);
    buf.push_str(&header_content);
    buf.push('\n');

    for (line_type, line_content) in hunk.lines() {
        let ranges = match line_type {
            DiffLineType::Unchanged => hl_add
                .highlight_line(line_content, syntax_set)
                .and_then(|_| hl_rem.highlight_line(line_content, syntax_set)),
            DiffLineType::Added => hl_add.highlight_line(line_content, syntax_set),
            DiffLineType::Removed => hl_rem.highlight_line(line_content, syntax_set),
        };

        let Ok(mut ranges) = ranges else {
            buf.push_str(line_content);
            continue;
        };

        let bg = match line_type {
            DiffLineType::Unchanged => {
                buf.push(' ');
                false
            }
            DiffLineType::Added => {
                buf.push_str(&marker_added);
                for r in &mut ranges {
                    r.0.background = MARKER_ADDED_STYLE.background;
                }
                true
            }
            DiffLineType::Removed => {
                buf.push_str(&marker_removed);
                for r in &mut ranges {
                    r.0.background = MARKER_REMOVED_STYLE.background;
                }
                true
            }
        };

        let highlighted_content = as_24_bit_terminal_escaped(&ranges, bg);
        buf.push_str(&highlighted_content);
        buf.push('\n');
    }
    // workaround: remove trailing line break
    buf.pop();

    // according to docs of `as_24_bit_terminal_escaped`
    buf.push_str("\x1b[0m");
    buf
}
