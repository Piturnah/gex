use std::{
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Output, Stdio},
};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Attribute, Color, SetForegroundColor},
    terminal::{self, ClearType},
};
use git2::Repository;
use phf::phf_ordered_map;

mod branch;
pub mod parse;
mod status;

use branch::BranchList;
use status::Status;

#[derive(PartialEq)]
enum State {
    Status,
    Commit,
    Branch,
}

static COMMIT_CMDS: phf::OrderedMap<char, &str> = phf_ordered_map! {
    'c' => "commit",
    'e' => "extend",
    'a' => "amend",
};

pub fn git_process(args: &[&str]) -> Result<Output> {
    Command::new("git").args(args).output().with_context(|| {
        format!(
            "failed to run `git{}`",
            args.iter().map(|a| " ".to_string() + a).collect::<String>()
        )
    })
}

fn run() -> Result<()> {
    clap::command!()
        .color(clap::ColorChoice::Never)
        .get_matches();

    // Attempt to find a git repository at or above current path
    let repo = match Repository::discover(Path::new(".")) {
        Ok(repo) => repo,
        Err(_) => {
            print!("Not a git repository. Initialise one? [y/N]");
            let _ = stdout().flush();
            let input = stdin()
                .lock()
                .lines()
                .next()
                .expect("couldn't read stdin")
                .expect("malformed stdin");
            if input.to_lowercase() != "y" {
                process::exit(0);
            }

            Repository::init(Path::new(".")).context("failed to initialise git repository")?
        }
    };

    // Set working directory in case the repository is not the current directory
    std::env::set_current_dir(repo.path().parent().context("`.git` cannot be root dir")?)
        .context("failed to set working directory")?;

    let mut status = Status::new(&repo)?;
    let mut branch_list = BranchList::new()?;
    let mut git_output: Option<Output> = None;

    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    terminal::enable_raw_mode().context("failed to put terminal in raw mode")?;
    print!("{}", cursor::Hide);

    let mut state = State::Status;
    let mut msg_buffer_height = 0;

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
                    ),)
                    .collect::<String>(),
                SetForegroundColor(Color::Reset),
                term_width = term_width as usize,
            );

            let _ = stdout().flush();
        }

        if let Some(output) = git_output {
            terminal::disable_raw_mode().context("failed to exit raw mode")?;

            match output.status.success() {
                true => {
                    // NOTE: I am still unsure if we want to propagate stdout on success. I fear
                    // that it may clutter the UI and a successful change should be communicated
                    // through seeing the results in gex anyway.
                    let git_msg = std::str::from_utf8(&output.stdout)
                        .context("malformed stdout from git")?
                        .trim()
                        .replace(
                            '+',
                            &format!(
                                "{}+{}",
                                SetForegroundColor(Color::DarkGreen),
                                SetForegroundColor(Color::Reset)
                            ),
                        )
                        .replace(
                            '-',
                            &format!(
                                "{}-{}",
                                SetForegroundColor(Color::DarkRed),
                                SetForegroundColor(Color::Reset)
                            ),
                        );
                    if !git_msg.is_empty() {
                        msg_buffer_height = git_msg.lines().count() + 1;
                        print!(
                            "{}{:─<term_width$}\n{}",
                            cursor::MoveTo(0, term_height.saturating_sub(msg_buffer_height as u16)),
                            "",
                            git_msg,
                            term_width = term_width as usize,
                        );
                    }
                }
                false => {
                    let git_msg = std::str::from_utf8(&output.stderr)
                        .context("malformed stderr from git")?
                        .trim();
                    if !git_msg.is_empty() {
                        msg_buffer_height = git_msg.lines().count() + 1;
                        print!(
                            "{}{:─<term_width$}\n{}{}{}",
                            cursor::MoveTo(0, term_height.saturating_sub(msg_buffer_height as u16)),
                            "",
                            SetForegroundColor(Color::Red),
                            git_msg,
                            SetForegroundColor(Color::Reset),
                            term_width = term_width as usize,
                        );
                    }
                }
            }

            terminal::enable_raw_mode().context("failed to enable raw mode")?;
            let _ = stdout().flush();

            git_output = None;
        } else {
            msg_buffer_height = 0;
        }

        if let Event::Key(event) = event::read().context("failed to read a terminal event")? {
            match state {
                State::Status => match event.code {
                    KeyCode::Char('j') | KeyCode::Down => status.down()?,
                    KeyCode::Char('k') | KeyCode::Up => status.up()?,
                    KeyCode::Char('s') => {
                        status.stage()?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('S') => {
                        git_output = Some(git_process(&["add", "."])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('u') => {
                        status.unstage()?;
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('U') => {
                        git_output = Some(git_process(&["reset"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Tab => status.expand()?,
                    KeyCode::Char('c') => {
                        state = State::Commit;
                    }
                    KeyCode::Char('F') => {
                        git_output = Some(git_process(&["pull"])?);
                        status.fetch(&repo)?;
                    }
                    KeyCode::Char('b') => {
                        branch_list.fetch()?;
                        state = State::Branch;
                    }
                    KeyCode::Char('r') => status.fetch(&repo)?,
                    KeyCode::Char(':') => {
                        terminal::disable_raw_mode().context("failed to disable raw mode")?;

                        // Clear the git output, if there is any. In future maybe organise the
                        // output / "terminal" as some kind of minibuffer so this is simpler.
                        for i in 0..=msg_buffer_height.min(term_height.into()) {
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
                        let _ = stdout().flush();
                        let input = stdin()
                            .lock()
                            .lines()
                            .next()
                            .context("no stdin")?
                            .context("malformed stdin")?;

                        git_output =
                            Some(git_process(&input.split_whitespace().collect::<Vec<_>>())?);

                        print!("{}", cursor::Hide);
                        terminal::enable_raw_mode().context("failed to enable raw mode")?;
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
                State::Commit => match event.code {
                    KeyCode::Char('c') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .context("failed to leave alternate screen")?;
                        git_output = Some(
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
                        git_output = Some(
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
                        git_output = Some(
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
                        branch_list.cursor += 1;
                        if branch_list.cursor >= branch_list.branches.len() {
                            branch_list.cursor = branch_list.branches.len() - 1;
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        git_output = Some(branch_list.checkout()?);
                        status.fetch(&repo)?;
                        state = State::Status;
                    }
                    KeyCode::Char('b') => {
                        git_output = Some(BranchList::checkout_new()?);
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
    run().map_err(|e| {
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
