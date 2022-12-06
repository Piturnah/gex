//! This module relates to the message buffer that appears on the bottom of the screen after
//! certain actions are performed.

use std::io::{stdout, Write};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    style::{Color, SetForegroundColor},
    terminal,
};

/// The messages to be sent are maintained in this struct as a stack.
pub struct Messages(Vec<(String, MessageType)>);

pub enum MessageType {
    Note,
    Error,
}

impl Messages {
    pub fn new() -> Self {
        Messages(Vec::new())
    }

    /// Push a new message onto the message stack.
    pub fn push(&mut self, msg: String, msg_type: MessageType) {
        self.0.push((msg, msg_type));
    }

    /// Render the most recent unsent message.
    pub fn render(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        if let Some((msg, msg_type)) = self.0.pop() {
            // Make sure raw mode is disabled so we can just print the message.
            terminal::disable_raw_mode().context("failed to exit raw mode")?;
            let msg_buffer_height = msg.lines().count() + 1;
            match msg_type {
                MessageType::Note => print!(
                    "{}{:─<term_width$}\n{}",
                    cursor::MoveTo(0, term_height.saturating_sub(msg_buffer_height as u16)),
                    "",
                    msg,
                    term_width = term_width as usize,
                ),
                MessageType::Error => print!(
                    "{}{:─<term_width$}\n{}{}{}",
                    cursor::MoveTo(0, term_height.saturating_sub(msg_buffer_height as u16)),
                    "",
                    SetForegroundColor(Color::Red),
                    msg,
                    SetForegroundColor(Color::Reset),
                    term_width = term_width as usize,
                ),
            }
            terminal::enable_raw_mode().context("failed to enable raw mode")?;
            let _ = stdout().flush();
        }
        Ok(())
    }
}
