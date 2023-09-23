use std::ops::Range;

use anyhow::{anyhow, bail, Result};
use line_span::{LineSpan, LineSpans};

// TODO: rename to RawHunk?
pub struct Hunk {
    /// Raw hunk string.
    ///
    /// This may not be modified after the hunk was created, as the other fields index into it.
    raw_hunk: String,
    /// Header marker and content.
    header: (Range<usize>, Range<usize>),
    /// Ranges of `raw_hunk` for each line with the corresponding diff type.
    ///
    /// Doesn't include the starting character of the diff line (`+/-/ `) or the line break.
    lines: Vec<HunkLine>,
}

impl Hunk {
    pub fn from_string(raw_hunk: String) -> Result<Self> {
        let mut lines_iter = raw_hunk.line_spans();
        let header = match lines_iter.next().map(Self::parse_header) {
            Some(Ok(header)) => header,
            Some(Err(e)) => return Err(e),
            _ => bail!("Empty hunk."),
        };
        let lines = lines_iter
            .map(HunkLine::from_line_span)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            raw_hunk,
            header,
            lines,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw_hunk
    }

    pub fn header(&self) -> (&str, &str) {
        (
            &self.raw_hunk[self.header.0.clone()],
            &self.raw_hunk[self.header.1.clone()],
        )
    }

    pub fn lines(&self) -> impl Iterator<Item = (DiffLineType, &str)> {
        self.lines
            .iter()
            .map(|line| (line.diff_type, &self.raw_hunk[line.range.clone()]))
    }

    fn parse_header(line: LineSpan) -> Result<(Range<usize>, Range<usize>)> {
        // TODO
        let range = line.range();
        Ok((range.start..range.start, range))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLineType {
    Unchanged,
    Added,
    Removed,
}

impl TryFrom<char> for DiffLineType {
    type Error = anyhow::Error;

    fn try_from(value: char) -> Result<Self> {
        match value {
            ' ' => Ok(Self::Unchanged),
            '+' => Ok(Self::Added),
            '-' => Ok(Self::Removed),
            c => Err(anyhow!("'{c}' is not a valid diff type character.")),
        }
    }
}

/// A single line (range) of a hunk.
struct HunkLine {
    /// Type of the line.
    diff_type: DiffLineType,
    /// The range of the line in the original string (see [`Hunk`]).
    ///
    /// Doesn't include the first character of the original line (`+`/`-`/` `).
    range: Range<usize>,
}

impl HunkLine {
    fn from_line_span(line: LineSpan) -> Result<Self> {
        let Some(first_char) = line.chars().next() else {
            bail!("");
        };
        let line_range = line.range();
        let diff_type = DiffLineType::try_from(first_char)
            // fall back to unchanged
            // TODO: this shouldn't happen, handle this better?
            .unwrap_or(DiffLineType::Unchanged);
        let range = (line_range.start + first_char.len_utf8())..line_range.end;
        Ok(Self { diff_type, range })
    }
}
