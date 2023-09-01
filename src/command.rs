use std::{
    fmt,
    io::stdout,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use crossterm::{cursor, terminal};

use crate::{branch::BranchList, config::Config, git_process, minibuffer::MiniBuffer, State, View};

macro_rules! commands {
    ($($key:literal: $cmd:tt => [$($subkey:literal: $subcmd:tt),+$(,)?]),*$(,)?) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug)]
            pub enum GexCommand { $($cmd),* }
            impl GexCommand {
                pub const fn commands() -> &'static [(char, Self)] {
                    &[$(($key, Self::$cmd)),*]
                }
                pub const fn subcommands(&self) -> &[(char, SubCommand)] {
                    match self {
                        $(Self::$cmd => {
                            &[$((
                                $subkey,
                                SubCommand::$cmd([<$cmd:lower>]::SubCommand::$subcmd)
                            )),*]
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
    'b': Branch => ['b': Checkout, 'n': New],
    'c': Commit => ['c': Commit, 'a': Amend, 'e': Extend],
    'p': Push => ['p': Remote, 'f': Force],
    'z': Stash => ['s': Stash, 'p': Pop],
}

impl GexCommand {
    #[allow(clippy::enum_glob_use)]
    pub fn handle_input(self, key: char, state: &mut State, config: &Config) -> Result<()> {
        use SubCommand::*;
        let State {
            ref mut status,
            ref mut view,
            repo,
            ..
        } = state;
        let Some((_, cmd)) = self.subcommands().iter().find(|(c, _)| key == *c) else {
            return Ok(());
        };

        match cmd {
            Branch(subcmd) => {
                use branch::SubCommand;
                match subcmd {
                    SubCommand::New => {
                        let checkout = BranchList::checkout_new()?;
                        MiniBuffer::push_command_output(&checkout);
                        status.fetch(repo, &config.options)?;
                        *view = View::Status;
                    }
                    SubCommand::Checkout => {
                        state.branch_list.fetch()?;
                        *view = View::BranchList;
                    }
                }
            }
            Commit(subcmd) => {
                use commit::SubCommand;
                match subcmd {
                    SubCommand::Commit => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        MiniBuffer::push_command_output(
                            &Command::new("git")
                                .arg("commit")
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo, &config.options)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;
                    }
                    SubCommand::Extend => {
                        MiniBuffer::push_command_output(
                            &Command::new("git")
                                .args(["commit", "--amend", "--no-edit"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo, &config.options)?;
                    }
                    SubCommand::Amend => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        MiniBuffer::push_command_output(
                            &Command::new("git")
                                .args(["commit", "--amend"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(repo, &config.options)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;
                    }
                }
                *view = View::Status;
            }
            Push(subcmd) => {
                use push::SubCommand;
                // For now we are just temporarily disabling the raw mode so that if the user is
                // aksed for credentials then they can provide them that way.
                crossterm::execute!(stdout(), cursor::MoveToColumn(0), cursor::Show)?;
                terminal::disable_raw_mode().context("failed to disable raw mode")?;
                match subcmd {
                    SubCommand::Remote => MiniBuffer::push_command_output(&git_process(&["push"])?),
                    SubCommand::Force => {
                        MiniBuffer::push_command_output(&git_process(&["push", "--force"])?);
                    }
                }
                crossterm::execute!(stdout(), cursor::Hide)?;
                terminal::enable_raw_mode().context("failed to enable raw mode")?;
                *view = View::Status;
            }
            Stash(subcmd) => {
                use stash::SubCommand;
                match subcmd {
                    SubCommand::Stash => MiniBuffer::push_command_output(&git_process(&["stash"])?),
                    SubCommand::Pop => {
                        MiniBuffer::push_command_output(&git_process(&["stash", "pop"])?);
                    }
                }
                status.fetch(repo, &config.options)?;
                *view = View::Status;
            }
        }

        Ok(())
    }
}
