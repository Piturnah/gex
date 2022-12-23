use std::{
    cmp, env,
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Output, Stdio},
};

use anyhow::{Context, Result};
use clap::Arg;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Attribute, Color, SetForegroundColor},
    terminal::{self, ClearType},
};
use git2::Repository;

use crate::minibuffer::{MessageType, MiniBuffer};

mod branch;
mod minibuffer;
mod parse;
mod status;

use branch::BranchList;
use status::Status;

#[derive(PartialEq)]
enum State {
    Status,
    Commit,
    Branch,
}

const COMMIT_CMDS: [(char, &str); 3] = [('c', "commit"), ('e', "extend"), ('a', "amend")];

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
        let _ = stdout().flush();
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

See https://github.com/Piturnah/gex/issues/13.".to_string(), MessageType::Error);
    }

    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    terminal::enable_raw_mode().context("failed to put terminal in raw mode")?;
    print!("{}", cursor::Hide);

    let mut state = State::Status;

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

        match state {
            State::Status | State::Commit => {
                print!(
                    "{}{}{}\r",
                    cursor::MoveToRow(0),
                    terminal::Clear(ClearType::All),
                    status,
                );
            }
            State::Branch => {
                print!(
                    "{}{}{}",
                    cursor::MoveToRow(0),
                    terminal::Clear(ClearType::All),
                    branch_list
                );
                let _ = stdout().flush();
            }
        }

        // Display the available commit commands
        if state == State::Commit {
            print!(
                "{}{:═^term_width$}{}{}{}",
                cursor::MoveTo(0, term_height - 1 - COMMIT_CMDS.len() as u16),
                "Commit Options",
                SetForegroundColor(Color::Red),
                COMMIT_CMDS
                    .into_iter()
                    .map(|(k, v)| format!(
                        "\r\n {}{}{}{} => {}",
                        SetForegroundColor(Color::Green),
                        Attribute::Bold,
                        k,
                        Attribute::Reset,
                        v
                    ))
                    .collect::<String>(),
                SetForegroundColor(Color::Reset),
                term_width = term_width as usize,
            );

            let _ = stdout().flush();
        }

        mini_buffer.render(term_width, term_height)?;

        if let Event::Key(event) = event::read().context("failed to read a terminal event")? {
            match state {
                State::Status => match event.code {
                    KeyCode::Char('j') | KeyCode::Down => status.down()?,
                    KeyCode::Char('k') | KeyCode::Up => status.up()?,
                    KeyCode::Char('G') | KeyCode::Char('J') => status.cursor_last()?,
                    KeyCode::Char('g') | KeyCode::Char('K') => status.cursor_first()?,
                    KeyCode::Char('s') => {
                        status.stage()?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('S') => {
                        mini_buffer.push_command_output(git_process(&["add", "."])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('u') => {
                        status.unstage()?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('U') => {
                        mini_buffer.push_command_output(git_process(&["reset"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Tab => status.expand()?,
                    KeyCode::Char('c') => {
                        state = State::Commit;
                    }
                    KeyCode::Char('F') => {
                        mini_buffer.push_command_output(git_process(&["pull"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('b') => {
                        branch_list.fetch()?;
                        state = State::Branch;
                    }
                    KeyCode::Char('r') => status.fetch(&repo)?,
                    KeyCode::Char(':') => {
                        mini_buffer.git_command(term_height)?;
                        status.fetch(&repo)?
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
                State::Commit => match event.code {
                    KeyCode::Char('c') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        mini_buffer.push_command_output(
                            Command::new("git")
                                .arg("commit")
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(&repo)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;

                        state = State::Status;
                    }
                    KeyCode::Char('e') => {
                        mini_buffer.push_command_output(
                            Command::new("git")
                                .args(["commit", "--amend", "--no-edit"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(&repo)?;

                        state = State::Status;
                    }
                    KeyCode::Char('a') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        mini_buffer.push_command_output(
                            Command::new("git")
                                .args(["commit", "--amend"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .context("failed to run `git commit`")?,
                        );
                        status.fetch(&repo)?;
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .context("failed to enter alternate screen")?;

                        state = State::Status;
                    }
                    KeyCode::Esc => state = State::Status,
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
                    _ => {}
                },
                State::Branch => match event.code {
                    KeyCode::Char('k') | KeyCode::Up => {
                        branch_list.cursor = branch_list.cursor.saturating_sub(1);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        branch_list.cursor =
                            cmp::min(branch_list.cursor + 1, branch_list.branches.len() - 1)
                    }
                    KeyCode::Char('g') | KeyCode::Char('K') => branch_list.cursor = 0,
                    KeyCode::Char('G') | KeyCode::Char('J') => {
                        branch_list.cursor = branch_list.branches.len() - 1
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        mini_buffer.push_command_output(branch_list.checkout()?);
                        status.fetch(&repo)?;
                        state = State::Status;
                    }
                    KeyCode::Char('b') => {
                        mini_buffer.push_command_output(BranchList::checkout_new()?);
                        status.fetch(&repo)?;
                        state = State::Status;
                    }
                    KeyCode::Esc => state = State::Status,
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
            };
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
        let _ = terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            stdout(),
            terminal::LeaveAlternateScreen,
            cursor::Show,
            cursor::MoveToColumn(0)
        );
        e
    })
}
