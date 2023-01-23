//! This module relates to the message buffer that appears on the bottom of the screen after
//! certain actions are performed and handles the arbitrary git command functionality.

use std::{
    io::{stdout, Write},
    process::Output,
    str,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
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
    /// History of commands sent via `:`.
    command_history: Vec<String>,
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
    pub fn git_command(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        // Clear the git output, if there is any.
        print!(
            "{}{}{}",
            cursor::MoveTo(0, term_height.saturating_sub(self.current_height as u16)),
            terminal::Clear(ClearType::FromCursorDown),
            cursor::Show,
        );

        let mut command = String::with_capacity(term_width as usize - 5);
        let mut cursor = 0;
        let mut history_cursor = 0;
        loop {
            print!(
                "{}{}\r\n{}:git {command}{}",
                cursor::MoveTo(0, term_height - 2),
                "\u{2574}".repeat(term_width.into()),
                terminal::Clear(ClearType::CurrentLine),
                cursor::MoveToColumn(cursor + 5),
            );
            drop(stdout().flush());

            if let Event::Key(key_event) =
                event::read().context("failed to read a terminal event")?
            {
                match key_event.code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        command.insert(cursor.into(), c);
                        cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if cursor > 0 {
                            cursor -= 1;
                            command.remove(cursor.into());
                        }
                    }
                    KeyCode::Left => cursor = cursor.saturating_sub(1),
                    KeyCode::Right => {
                        if (cursor as usize) < command.len() {
                            cursor += 1;
                        }
                    }
                    KeyCode::Up => {
                        if history_cursor < self.command_history.len() {
                            history_cursor += 1;
                            self.command_history[self.command_history.len() - history_cursor]
                                .clone_into(&mut command);
                            cursor = command.len() as u16;
                        }
                    }
                    KeyCode::Down => {
                        history_cursor = history_cursor.saturating_sub(1);
                        if history_cursor == 0 {
                            command.clear();
                        } else {
                            self.command_history[self.command_history.len() - history_cursor]
                                .clone_into(&mut command);
                        }
                        cursor = command.len() as u16;
                    }
                    KeyCode::Esc => {
                        print!("{}", cursor::Hide);
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        crossterm::execute!(stdout(), cursor::MoveToColumn(0))?;
        terminal::disable_raw_mode().context("failed to disable raw mode")?;
        self.push_command_output(&git_process(
            &command.split_whitespace().collect::<Vec<_>>(),
        )?);
        terminal::enable_raw_mode().context("failed to enable raw mode")?;

        self.command_history.push(command);

        print!("{}", cursor::Hide);
        Ok(())
    }

    /// Render the most recent unsent message.
    pub fn render(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        if let Some((msg, msg_type)) = self.messages.pop() {
            // Make sure raw mode is disabled so we can just print the message.
            terminal::disable_raw_mode().context("failed to exit raw mode")?;
            self.current_height = msg.lines().count() + 1;
            match msg_type {
                MessageType::Note => print!(
                    "{}{}\n{msg}",
                    cursor::MoveTo(0, term_height.saturating_sub(self.current_height as u16)),
                    "─".repeat(term_width.into()),
                ),
                MessageType::Error => print!(
                    "{}{}\n{}{msg}{}",
                    cursor::MoveTo(0, term_height.saturating_sub(self.current_height as u16)),
                    "─".repeat(term_width.into()),
                    SetForegroundColor(Color::Red),
                    SetForegroundColor(Color::Reset),
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
