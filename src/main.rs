#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::module_name_repetitions,
    clippy::let_underscore_untyped
)]

use std::{
    cmp, env,
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Output},
};

use anyhow::{Context, Result};
use clap::Arg;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    style::{Attribute, Color, SetForegroundColor},
    terminal::{self, ClearType},
};
use git2::Repository;

use crate::{
    command::GexCommand,
    minibuffer::{MessageType, MiniBuffer},
};

mod branch;
mod command;
mod minibuffer;
mod parse;
mod status;

use branch::BranchList;
use status::Status;

pub enum View {
    Status,
    BranchList,
    Command(GexCommand),
}

pub fn git_process(args: &[&str]) -> Result<Output> {
    Command::new("git").args(args).output().with_context(|| {
        format!(
            "failed to run `git{}`",
            args.iter().map(|a| " ".to_string() + a).collect::<String>()
        )
    })
}

fn run(path: &Path) -> Result<()> {
    // Attempt to find a git repository at or above current path
    let repo = if let Ok(repo) = Repository::discover(path) {
        repo
    } else {
        print!("Not a git repository. Initialise one? [y/N]");
        drop(stdout().flush());
        let input = stdin()
            .lock()
            .lines()
            .next()
            .context("couldn't read stdin")?
            .context("malformed stdin")?;
        if input.to_lowercase() != "y" {
            process::exit(0);
        }

        Repository::init(path).context("failed to initialise git repository")?
    };

    // Set working directory in case the repository is not the current directory
    std::env::set_current_dir(repo.path().parent().context("`.git` cannot be root dir")?)
        .context("failed to set working directory")?;

    let mut status = Status::new(&repo)?;
    let mut branch_list = BranchList::new()?;
    let mut mini_buffer = MiniBuffer::new();

    // Non-English locale settings are currently unsupported. See
    // https://github.com/Piturnah/gex/issues/13.
    if !env::var("LANG")
        .map(|s| s.starts_with("en"))
        .unwrap_or(true)
    {
        mini_buffer.push("WARNING: Non-English locale detected. For now, Gex only supports English locale setting.
Set locale to English, e.g.:

        $ LANG=en_GB gex

See https://github.com/Piturnah/gex/issues/13.", MessageType::Error);
    }

    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    terminal::enable_raw_mode().context("failed to put terminal in raw mode")?;
    print!("{}", cursor::Hide);

    let mut view = View::Status;

    // Structure of the event loop
    //
    // 1. Clear the terminal
    // 2. Render status or branch list
    // 3. Render option overlay
    // 4. Render minibuffer messages
    // 5. Wait for event and update state
    //
    loop {
        let (term_width, term_height) =
            terminal::size().context("failed to query terminal dimensions")?;

        match view {
            View::Status | View::Command(_) => {
                print!(
                    "{}{}{status}\r",
                    cursor::MoveToRow(0),
                    terminal::Clear(ClearType::All),
                );
            }
            View::BranchList => {
                print!(
                    "{}{}{branch_list}",
                    cursor::MoveToRow(0),
                    terminal::Clear(ClearType::All),
                );
                drop(stdout().flush());
            }
        }

        // Display the available subcommands
        if let View::Command(cmd) = view {
            let subcmds = cmd.subcommands();
            print!(
                "{}{title:═^term_width$}{}{}{}",
                cursor::MoveTo(0, term_height - 1 - subcmds.len() as u16),
                terminal::Clear(ClearType::FromCursorDown),
                subcmds
                    .iter()
                    .map(|(k, v)| format!(
                        "\r\n {}{}{k}{} => {v}",
                        SetForegroundColor(Color::Green),
                        Attribute::Bold,
                        Attribute::Reset,
                    ))
                    .collect::<String>(),
                SetForegroundColor(Color::Reset),
                term_width = term_width as usize,
                title = format!(" {cmd:?} Options "),
            );

            drop(stdout().flush());
        }

        mini_buffer.render(term_width, term_height)?;

        // Handle input
        //
        // Check what event we get. If we got an event other than a key event, we don't need to
        // handle it so we break. If we got a key event with KeyEventKind::Release, we try again in
        // the loop to avoid re-rendering. If it's a key event without KeyEventKind::Release,
        // handle it and break.
        loop {
            let Event::Key(event) = event::read().context("failed to read a terminal event")? else {
                break;
            };
            if event.kind == KeyEventKind::Release {
                continue;
            }

            match view {
                View::Status => match event.code {
                    KeyCode::Char('j') | KeyCode::Down => status.down()?,
                    KeyCode::Char('k') | KeyCode::Up => status.up()?,
                    KeyCode::Char('G' | 'J') => status.cursor_last()?,
                    KeyCode::Char('g' | 'K') => status.cursor_first()?,
                    KeyCode::Char('s') => {
                        status.stage(&mut mini_buffer)?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('S') => {
                        mini_buffer.push_command_output(&git_process(&["add", "."])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('u') => {
                        status.unstage(&mut mini_buffer)?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('U') => {
                        mini_buffer.push_command_output(&git_process(&["reset"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Tab => status.expand()?,
                    KeyCode::Char('c') => {
                        view = View::Command(GexCommand::Commit);
                    }
                    KeyCode::Char('F') => {
                        mini_buffer.push_command_output(&git_process(&["pull"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('b') => {
                        branch_list.fetch()?;
                        view = View::Command(GexCommand::Branch);
                    }
                    KeyCode::Char('r') => status.fetch(&repo)?,
                    KeyCode::Char(':') => {
                        mini_buffer.git_command(term_width, term_height)?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to disable raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    _ => {}
                },
                View::BranchList => match event.code {
                    KeyCode::Char('k') | KeyCode::Up => {
                        branch_list.cursor = branch_list.cursor.saturating_sub(1);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        branch_list.cursor =
                            cmp::min(branch_list.cursor + 1, branch_list.branches.len() - 1);
                    }
                    KeyCode::Char('g' | 'K') => branch_list.cursor = 0,
                    KeyCode::Char('G' | 'J') => {
                        branch_list.cursor = branch_list.branches.len() - 1;
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        mini_buffer.push_command_output(&branch_list.checkout()?);
                        status.fetch(&repo)?;
                        view = View::Status;
                    }
                    KeyCode::Esc => view = View::Status,
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to disable raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    _ => {}
                },
                View::Command(cmd) => match event.code {
                    KeyCode::Esc => view = View::Status,
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to exit raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    KeyCode::Char(c) => {
                        cmd.handle_input(c, &mut mini_buffer, &mut status, &repo, &mut view)?;
                    }
                    _ => {}
                },
            };
            break;
        }
    }
}

fn main() -> Result<()> {
    let matches = clap::command!()
        .version(env!("GEX_VERSION"))
        .arg(
            Arg::new("path")
                .default_value(".")
                .value_name("PATH")
                .help("The path to the repository"),
        )
        .get_matches();
    let path = matches
        .get_one::<String>("path")
        .expect("default value provided");

    run(Path::new(path)).map_err(|e| {
        // We don't want to do anything if these fail since then we'll lose the original error
        // message we are trying to propagate
        drop(terminal::disable_raw_mode());
        drop(crossterm::execute!(
            stdout(),
            terminal::LeaveAlternateScreen,
            cursor::Show,
            cursor::MoveToColumn(0)
        ));
        e
    })
}
