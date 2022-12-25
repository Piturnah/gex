//! This module relates to the message buffer that appears on the bottom of the screen after
//! certain actions are performed and handles the arbitrary git command functionality.

use std::{
    io::{stdin, stdout, BufRead, Write},
    process::Output,
    str,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    style::{Color, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::git_process;

#[derive(Default)]
pub struct MiniBuffer {
    /// The messages to be sent are maintained in this struct as a stack.
    messages: Vec<(String, MessageType)>,
    /// The current height of the buffer, including the border.
    current_height: usize,
}

pub enum MessageType {
    Note,
    Error,
}

impl MiniBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new message onto the message stack.
    pub fn push(&mut self, msg: String, msg_type: MessageType) {
        self.messages.push((msg, msg_type));
    }

    pub fn push_command_output(&mut self, output: &Output) {
        if !output.stdout.is_empty() {
            match str::from_utf8(&output.stdout) {
                Ok(s) => self.push(s.trim().to_string(), MessageType::Note),
                Err(e) => self.push(
                    format!("Received invalid UTF8 stdout from git: {e}"),
                    MessageType::Error,
                ),
            }
        }
        if !output.stderr.is_empty() {
            self.push(
                str::from_utf8(&output.stderr).map_or_else(
                    |e| format!("Received invalid UTF8 stderr from git: {e}"),
                    |msg| msg.trim().to_string(),
                ),
                MessageType::Error,
            );
        }
    }

    /// Call to enter Command mode. It will be exited automatically in the render call after the
    /// command is sent.
    pub fn git_command(&mut self, term_height: u16) -> Result<()> {
        terminal::disable_raw_mode().context("failed to disable raw mode")?;

        // Clear the git output, if there is any.
        for i in 0..=self.current_height.min(term_height.into()) {
            print!(
                "{}{}",
                cursor::MoveTo(0, term_height - i as u16),
                terminal::Clear(ClearType::UntilNewLine)
            );
        }

        print!(
            "{}{}:git ",
            cursor::MoveTo(0, term_height - 1),
            cursor::Show
        );
        drop(stdout().flush());
        let input = stdin()
            .lock()
            .lines()
            .next()
            .context("no stdin")?
            .context("malformed stdin")?;

        self.push_command_output(&git_process(&input.split_whitespace().collect::<Vec<_>>())?);

        print!("{}", cursor::Hide);
        terminal::enable_raw_mode().context("failed to enable raw mode")
    }

    /// Render the most recent unsent message.
    pub fn render(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        if let Some((msg, msg_type)) = self.messages.pop() {
            // Make sure raw mode is disabled so we can just print the message.
            terminal::disable_raw_mode().context("failed to exit raw mode")?;
            self.current_height = msg.lines().count() + 1;
            match msg_type {
                MessageType::Note => print!(
                    "{}{:─<term_width$}\n{}",
                    cursor::MoveTo(0, term_height.saturating_sub(self.current_height as u16)),
                    "",
                    msg,
                    term_width = term_width as usize,
                ),
                MessageType::Error => print!(
                    "{}{:─<term_width$}\n{}{}{}",
                    cursor::MoveTo(0, term_height.saturating_sub(self.current_height as u16)),
                    "",
                    SetForegroundColor(Color::Red),
                    msg,
                    SetForegroundColor(Color::Reset),
                    term_width = term_width as usize,
                ),
            }
            terminal::enable_raw_mode().context("failed to enable raw mode")?;
            drop(stdout().flush());
        } else {
            self.current_height = 0;
        }
        Ok(())
    }
}
