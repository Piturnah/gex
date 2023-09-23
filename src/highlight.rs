use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

use crate::hunk::{DiffLineType, Hunk};

// Diff styles
// TODO: use existing colors from configuration?

const HEADER_MARKER_STYLE: Style = Style {
    foreground: Color::WHITE,
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};

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

pub fn highlight_hunk(hunk: &Hunk, hl: &SyntaxHighlight, syntax: &SyntaxReference) -> String {
    // TODO: move somewhere else?
    let marker_added = as_24_bit_terminal_escaped(&[(MARKER_ADDED_STYLE, "\u{258c}")], true);
    let marker_removed = as_24_bit_terminal_escaped(&[(MARKER_REMOVED_STYLE, "\u{258c}")], true);

    // separate highlighters for added and removed lines to keep the syntax intact
    let mut hl_add = HighlightLines::new(syntax, &hl.theme);
    let mut hl_rem = HighlightLines::new(syntax, &hl.theme);

    let mut buf = String::new();

    let (header_marker, header_content) = {
        let header = hunk.header();
        let header_content = hl_add
            .highlight_line(header.1, &hl.syntax_set)
            .and_then(|_| hl_rem.highlight_line(header.1, &hl.syntax_set));
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
                .highlight_line(line_content, &hl.syntax_set)
                .and_then(|_| hl_rem.highlight_line(line_content, &hl.syntax_set)),
            DiffLineType::Added => hl_add.highlight_line(line_content, &hl.syntax_set),
            DiffLineType::Removed => hl_rem.highlight_line(line_content, &hl.syntax_set),
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

    // according to docs of `as_24_bit_terminal_escaped`
    buf.push_str("\x1b[0m");
    buf
}
