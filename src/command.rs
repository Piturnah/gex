use std::{
    fmt,
    io::stdout,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use crossterm::{cursor, terminal};
use git2::Repository;

use crate::{branch::BranchList, minibuffer::MiniBuffer, status::Status, State};

macro_rules! commands {
    ($($cmd:tt => [$($key:literal: $subcmd:tt),+$(,)?]),*$(,)?) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug)]
            pub enum GexCommand { $($cmd),* }
            impl GexCommand {
                pub const fn subcommands(&self) -> &[(char, SubCommand)] {
                    match self {
                        $(Self::$cmd => {
                            &[
                                $(($key, SubCommand::$cmd([<$cmd:lower>]::SubCommand::$subcmd))),*
                            ]
                        }),*
                    }
                }
            }

            #[derive(Clone, Copy)]
            pub enum SubCommand { $($cmd([<$cmd:lower>]::SubCommand)),* }
            impl fmt::Display for SubCommand {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    match self { $(Self::$cmd(subcmd) => write!(f, "{subcmd}")),* }
                }
            }

            $(
                pub mod [<$cmd:lower>] {
                    use std::fmt;
                    #[derive(Debug, Clone, Copy)]
                    pub enum SubCommand { $($subcmd),* }
                    impl fmt::Display for SubCommand {
                        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            match self {
                                $(Self::$subcmd => write!(f, stringify!([<$subcmd:lower>]))),*
                            }
                        }
                    }
                }
            )*
        }
    }
}

commands! {
    Branch => ['b': Checkout, 'n': New],
    Commit => ['c': Commit, 'a': Amend, 'e': Extend],
}

impl GexCommand {
    #[allow(clippy::enum_glob_use)]
    pub fn handle_input(
        self,
        key: char,
        mini_buffer: &mut MiniBuffer,
        status: &mut Status,
        repo: &Repository,
        state: &mut State,
    ) -> Result<()> {
        use SubCommand::*;
        let Some((_, cmd)) = self.subcommands().iter().find(|(c, _)| key == *c) else {
            return Ok(());
        };

        match cmd {
            Branch(subcmd) => {
                use branch::SubCommand;
                match subcmd {
                    SubCommand::New => {
                        let checkout = BranchList::checkout_new()?;
                        mini_buffer.push_command_output(&checkout);
                        status.fetch(repo)?;
                        *state = State::Status;
                    }
                    SubCommand::Checkout => {
                        *state = State::BranchList;
                    }
                }
            }
            Commit(subcmd) => {
                use commit::SubCommand;
                match subcmd {
                    SubCommand::Commit => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        mini_buffer.push_command_output(
                            &Command::new("git")
                                .arg("commit")
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;
                    }
                    SubCommand::Extend => {
                        mini_buffer.push_command_output(
                            &Command::new("git")
                                .args(["commit", "--amend", "--no-edit"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo)?;
                    }
                    SubCommand::Amend => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        mini_buffer.push_command_output(
                            &Command::new("git")
                                .args(["commit", "--amend"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;
                    }
                }
                *state = State::Status;
            }
        }

        Ok(())
    }
}
