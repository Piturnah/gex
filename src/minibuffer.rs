//! This module relates to the message buffer that appears on the bottom of the screen after
//! certain actions are performed and handles the arbitrary git command functionality.

use std::{
    env,
    io::{stdout, Write},
    process::{Command, Output},
    rc::Rc,
    str,
    sync::Mutex,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::SetForegroundColor,
    terminal::{self, ClearType},
};
use itertools::Itertools;

use crate::{config, git_process, render::Clear, View};

/// The messages to be sent to the buffer are maintained in this mutex as a stack.
pub static MESSAGES: Mutex<Vec<(String, MessageType)>> = Mutex::new(Vec::new());

/// The callback type for getting input.
pub type Callback = Rc<dyn Fn(Option<&str>) -> Result<()>>;

#[derive(PartialEq, Eq, Default)]
enum State {
    #[default]
    Normal,
    Input,
}

#[derive(Default)]
enum History {
    #[default]
    Command,
    Git,
}

#[derive(Default)]
pub struct MiniBuffer {
    /// History of commands sent via `:`.
    git_command_history: Vec<String>,
    /// History of commands sent via `!`.
    command_history: Vec<String>,

    buffer: String,
    prompt: &'static str,
    cursor: usize,
    history_cursor: usize,
    // Which history to use.
    history: History,
    state: State,
}

#[derive(Debug)]
pub enum MessageType {
    Note,
    Error,
}

impl MiniBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty() -> bool {
        MESSAGES.try_lock().expect("couldn't get mutex").is_empty()
    }

    /// Push a new message onto the message stack.
    pub fn push(msg: &str, msg_type: MessageType) {
        if !msg.is_empty() {
            MESSAGES
                .try_lock()
                .expect("couldn't get mutex")
                .push((msg.trim().to_string(), msg_type));
        }
    }

    pub fn push_command_output(output: &Output) {
        match str::from_utf8(&output.stdout) {
            Ok(s) => Self::push(s, MessageType::Note),
            Err(e) => Self::push(
                &format!("Received invalid UTF8 stdout from git: {e}"),
                MessageType::Error,
            ),
        }
        match str::from_utf8(&output.stderr) {
            Ok(s) => Self::push(s, MessageType::Error),
            Err(e) => Self::push(
                &format!("Received invalid UTF8 stderr from git: {e}"),
                MessageType::Error,
            ),
        }
    }

    /// Get some user input from this minibuffer and run `callback` on it.
    pub fn get_input(&mut self, callback: Callback, prompt: Option<&'static str>, view: &mut View) {
        self.cursor = 0;
        self.buffer.clear();
        self.history_cursor = 0;
        self.state = State::Input;
        self.prompt = prompt.unwrap_or("");
        // This clone should be very cheap as we should never be calling this method while already
        // in View::Input.
        debug_assert!(!matches!(view, View::Input(..)));
        *view = View::Input(callback, Box::new(view.clone()));
    }

    /// `return_view`: the [`View`](crate::View) to switch to after exiting `View::Input`.
    ///
    /// # Notes
    ///
    /// Should only be called as part of the main event loop.
    pub fn handle_input(
        &mut self,
        key_event: KeyEvent,
        callback: &Callback,
        return_view: View,
        view: &mut View,
    ) -> Result<()> {
        let Self {
            ref mut buffer,
            ref mut cursor,
            ref mut history_cursor,
            ..
        } = self;
        let history = match self.history {
            History::Command => &mut self.command_history,
            History::Git => &mut self.git_command_history,
        };
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Enter, _) => {
                history.push(self.buffer.clone());
                callback(Some(&self.buffer))?;
                self.state = State::Normal;
                self.buffer.clear();
                *view = return_view;
            }
            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                *cursor = cursor.saturating_sub(1);
            }
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                if *cursor < buffer.len() {
                    *cursor += 1;
                }
            }
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                if *history_cursor < history.len() {
                    *history_cursor += 1;
                    history[history.len() - *history_cursor].clone_into(buffer);
                    *cursor = buffer.len();
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                *history_cursor = history_cursor.saturating_sub(1);
                if *history_cursor == 0 {
                    buffer.clear();
                } else {
                    history[history.len() - *history_cursor].clone_into(buffer);
                }
                *cursor = buffer.len();
            }
            (KeyCode::Home, _) => *cursor = 0,
            (KeyCode::End, _) => *cursor = buffer.len(),
            (KeyCode::Char('b'), KeyModifiers::ALT) => {
                while *cursor > 0 {
                    *cursor = cursor.saturating_sub(1);
                    if word_boundary(buffer, *cursor) {
                        break;
                    }
                }
            }
            (KeyCode::Char('f'), KeyModifiers::ALT) => {
                while *cursor < buffer.len() {
                    *cursor += 1;
                    if word_boundary(buffer, *cursor) {
                        break;
                    }
                }
            }
            (KeyCode::Char(c), _) => {
                buffer.insert(*cursor, c);
                *cursor += 1;
            }
            (KeyCode::Backspace, _) => {
                if *cursor > 0 {
                    *cursor -= 1;
                    buffer.remove(*cursor);
                }
            }
            (KeyCode::Delete, _) => {
                if (*cursor) < buffer.len() || *cursor == 0 && buffer.len() == 1 {
                    buffer.remove(*cursor);
                } else if !buffer.is_empty() {
                    buffer.pop();
                    *cursor -= 1;
                }
            }
            (KeyCode::Esc, _) => {
                callback(None)?;
                self.state = State::Normal;
                self.buffer.clear();
                *view = return_view;
            }
            _ => {}
        }
        Ok(())
    }

    /// Get a git command or shell command from the user and execute it.
    pub fn command(&mut self, git_cmd: bool, view: &mut View) {
        let (prompt, history) = if git_cmd {
            (":git ", History::Git)
        } else {
            ("!", History::Command)
        };
        self.history = history;
        self.get_input(
            Rc::new(move |cmd: Option<&str>| {
                crossterm::execute!(stdout(), cursor::MoveToColumn(0))?;
                terminal::disable_raw_mode().context("failed to disable raw mode")?;
                if let Some(cmd) = cmd {
                    let cmd_output = if git_cmd {
                        let output = git_process(&cmd.split_whitespace().collect::<Vec<_>>());
                        Some(output)
                    } else {
                        // TODO: lazy_static the shell or something.
                        let output = env::var("SHELL").map_or_else(
                            |_| {
                                let mut words = cmd.split_whitespace();
                                words
                                    .next()
                                    .map(|cmd| Command::new(cmd).args(words).output())
                            },
                            |sh| Some(Command::new(sh).args(["-c", cmd]).output()),
                        );

                        output.map(|o| o.context("failed to run command"))
                    };
                    match cmd_output {
                        Some(Ok(cmd_output)) => Self::push_command_output(&cmd_output),
                        Some(Err(e)) => Self::push(&format!("{e:?}"), MessageType::Error),
                        None => {}
                    }
                }
                terminal::enable_raw_mode().context("failed to enable raw mode")?;
                print!("{}", cursor::Hide);
                Ok(())
            }),
            Some(prompt),
            view,
        );
    }

    /// Render the contents of the buffer.
    pub fn render(&mut self, term_width: u16, term_height: u16) -> Result<()> {
        if self.state == State::Normal {
            if self.buffer.is_empty() {
                return Ok(());
            }
            // Make sure raw mode is disabled so we can just print the message.
            terminal::disable_raw_mode().context("failed to exit raw mode")?;
        }

        let (border, prompt) = match self.state {
            State::Normal => ("─", ""),
            State::Input => ("\u{2574}", self.prompt),
        };

        let current_height = std::cmp::max(self.buffer.lines().count() + 1, 2) as u16;
        print!(
            "{}{}{}\r\n{prompt}{}",
            cursor::MoveTo(0, term_height.saturating_sub(current_height)),
            Clear(ClearType::FromCursorDown),
            border.repeat(term_width.into()),
            self.buffer,
        );

        match self.state {
            State::Normal => {
                terminal::enable_raw_mode().context("failed to enable raw mode")?;
                self.buffer.clear();
            }
            State::Input => {
                print!(
                    "{}{}{}",
                    cursor::Show,
                    cursor::MoveToColumn((self.cursor + prompt.len()) as u16),
                    if self.buffer.len() == self.cursor {
                        SetCursorStyle::DefaultUserShape
                    } else {
                        SetCursorStyle::SteadyBar
                    },
                );
            }
        }

        drop(stdout().flush());
        Ok(())
    }

    /// Pops the most recent message sent into the minibuffer.
    pub fn pop_message(&mut self) {
        let Some((msg, msg_type)) = MESSAGES.try_lock().expect("couldn't get mutex lock").pop()
        else {
            return;
        };
        self.buffer = match msg_type {
            MessageType::Note => msg,
            MessageType::Error => format!("{}{msg}", SetForegroundColor(config!().colors.error)),
        };
    }
}

/// Checks if idx is on an Emacs-style word boundary in the buffer.
/// <https://www.gnu.org/software/emacs/manual/html_node/elisp/Syntax-Class-Table.html>
fn word_boundary(buffer: &str, idx: usize) -> bool {
    buffer
        .chars()
        .tuple_windows()
        .nth(idx.saturating_sub(1))
        .map_or(true, |(c1, c2)| {
            !c1.is_alphanumeric() && c2.is_alphanumeric()
        })
}
