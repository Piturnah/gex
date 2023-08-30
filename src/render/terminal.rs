//! Terminal commands related to rendering. These often just wrap commands from [`crossterm`].
//!
//! This exists because when resetting the terminal colours or clearing the screen we may have to
//! handle the case where the user has set custom FG/BG colours specially.

use std::fmt;

use crossterm::{
    cursor,
    style::{self, Color},
    terminal::{self, ClearType},
};

use crate::config;

/// See [`Clear`](`crossterm::terminal::Clear`).
pub struct Clear(pub ClearType);

impl fmt::Display for Clear {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match config!().colors.background {
            Color::Reset => write!(f, "{}", crossterm::terminal::Clear(self.0)),
            color => {
                let Ok((cols, rows)) = terminal::size() else {
                    return Err(fmt::Error);
                };

                write!(f, "{}", style::SetBackgroundColor(color))?;
                match self.0 {
                    ClearType::All => {
                        write!(f, "{}{}", cursor::SavePosition, cursor::MoveTo(0, 0),)?;
                        for _ in 0..rows {
                            write!(f, "{:width$}", ' ', width = cols as usize)?;
                        }
                        write!(f, "{}", cursor::RestorePosition)
                    }
                    ClearType::Purge => write!(
                        f,
                        "{}{}",
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::Purge),
                        Self(ClearType::All)
                    ),
                    ClearType::FromCursorDown => {
                        let Ok((_, row)) = cursor::position() else {
                            return Err(fmt::Error);
                        };
                        write!(f, "{}{}", cursor::SavePosition, cursor::MoveToNextLine(1))?;
                        for _ in 0..rows - row {
                            write!(f, "{:width$}", ' ', width = cols as usize)?;
                        }
                        write!(f, "{}", cursor::RestorePosition)
                    }
                    ClearType::FromCursorUp => {
                        let Ok((_, row)) = cursor::position() else {
                            return Err(fmt::Error);
                        };
                        write!(f, "{}{}", cursor::SavePosition, cursor::MoveTo(0, 0))?;
                        for _ in 0..=row {
                            write!(f, "{:width$}", ' ', width = cols as usize)?;
                        }
                        write!(f, "{}", cursor::RestorePosition)
                    }
                    ClearType::CurrentLine => write!(
                        f,
                        "{}{:width$}{}",
                        cursor::MoveToColumn(0),
                        ' ',
                        cursor::MoveToColumn(0),
                        width = cols as usize
                    ),
                    ClearType::UntilNewLine => {
                        let Ok((col, _)) = cursor::position() else {
                            return Err(fmt::Error);
                        };
                        write!(
                            f,
                            "{:width$}{}",
                            ' ',
                            cursor::MoveToColumn(col),
                            width = (cols - col) as usize
                        )
                    }
                }
            }
        }
    }
}

/// See [`ResetColor`](`crossterm::style::ResetColor`).
pub struct ResetColor;

impl fmt::Display for ResetColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}",
            crossterm::style::SetForegroundColor(config!().colors.foreground),
            crossterm::style::SetBackgroundColor(config!().colors.background),
        )
    }
}

/// See [`Attribute`](`crossterm::style::Attribute`).
///
/// Simulates the `Reset` variant.
pub struct ResetAttributes;

impl fmt::Display for ResetAttributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", crossterm::style::Attribute::Reset, ResetColor,)
    }
}
