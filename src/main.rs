use std::{
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Output, Stdio},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Color, SetForegroundColor},
    terminal::{self, ClearType},
};

mod branch;
pub mod parse;
mod status;

use branch::BranchList;
use status::Status;

pub trait Expand {
    fn toggle_expand(&mut self);
    fn expanded(&self) -> bool;
}

enum State {
    Status,
    Branch,
}

pub fn git_process(args: &[&str]) -> Output {
    Command::new("git").args(args).output().unwrap_or_else(|_| {
        panic!(
            "failed to run `git{}`",
            args.iter().map(|a| " ".to_string() + a).collect::<String>()
        )
    })
}

fn main() {
    for arg in std::env::args() {
        if arg == "--version" || arg == "-v" {
            println!("gex version {}", env!("CARGO_PKG_VERSION"));
            process::exit(0);
        }
    }

    if !Path::new("./.git").is_dir() {
        print!("Not a git repository. Initialise one? [y/N]");
        let _ = stdout().flush();
        if let Some(Ok(input)) = stdin().lock().lines().next() {
            if input.to_lowercase() != "y" {
                process::exit(0);
            }

            git_process(&["init"]);
        }
    }

    let mut status = Status::new();
    let mut branch_list = BranchList::new();
    let mut git_output: Option<Output> = None;

    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .expect("failed to enter alternate screen");
    terminal::enable_raw_mode().expect("failed to put terminal in raw mode");
    print!("{}", cursor::Hide);

    let mut state = State::Status;

    loop {
        match state {
            State::Status => {
                print!(
                    "{}{}{}{}",
                    cursor::MoveToRow(0),
                    terminal::Clear(ClearType::All),
                    status,
                    cursor::MoveToColumn(0)
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

        if let Some(output) = git_output {
            let (term_width, term_height) =
                terminal::size().expect("failed to query terminal dimensions");

            terminal::disable_raw_mode().unwrap();

            match output.status.success() {
                true => {
                    // NOTE: I am still unsure if we want to propagate stdout on success. I fear
                    // that it may clutter the UI and a successful change should be communicated
                    // through seeing the results in gex anyway.
                    let git_msg = std::str::from_utf8(&output.stdout)
                        .unwrap()
                        .lines()
                        .next()
                        .unwrap_or("");

                    if !git_msg.is_empty() {
                        print!(
                            "{}{:─<term_width$}\n{}",
                            cursor::MoveTo(0, term_height - 2),
                            "",
                            git_msg,
                            term_width = term_width as usize,
                        );
                    }
                }
                false => {
                    let git_msg = std::str::from_utf8(&output.stderr).unwrap().trim_end();
                    print!(
                        "{}{:─<term_width$}\n{}{}{}",
                        cursor::MoveTo(0, term_height - git_msg.lines().count() as u16 - 1),
                        "",
                        SetForegroundColor(Color::Red),
                        git_msg,
                        SetForegroundColor(Color::Reset),
                        term_width = term_width as usize,
                    );
                }
            }

            terminal::enable_raw_mode().unwrap();
            let _ = stdout().flush();

            git_output = None;
        }

        if let Event::Key(event) = event::read().unwrap() {
            match state {
                State::Status => match event.code {
                    KeyCode::Char('j') | KeyCode::Down => status.down(),
                    KeyCode::Char('k') | KeyCode::Up => status.up(),
                    KeyCode::Char('s') => status.stage(),
                    KeyCode::Char('S') => {
                        git_output = Some(git_process(&["add", "."]));
                        status.fetch();
                    }
                    KeyCode::Char('u') => status.unstage(),
                    KeyCode::Char('U') => {
                        git_output = Some(git_process(&["reset"]));
                        status.fetch();
                    }
                    KeyCode::Tab => status.expand(),
                    KeyCode::Char('c') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .expect("failed to leave alternate screen");
                        git_output = Some(
                            Command::new("git")
                                .arg("commit")
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .expect("failed to run `git commit`"),
                        );
                        status.fetch();
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .expect("failed to enter alternate screen");
                    }
                    KeyCode::Char('F') => {
                        git_output = Some(git_process(&["pull"]));
                        status.fetch();
                    }
                    KeyCode::Char('b') => {
                        branch_list.fetch();
                        state = State::Branch;
                    }
                    KeyCode::Char('r') => status.fetch(),
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().unwrap();
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .expect("failed to leave alternate screen");
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
                        git_output = Some(branch_list.checkout());
                        status.fetch();
                        state = State::Status;
                    }
                    KeyCode::Char('b') => {
                        git_output = Some(BranchList::checkout_new());
                        status.fetch();
                        state = State::Status;
                    }
                    KeyCode::Esc => state = State::Status,
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().unwrap();
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .expect("failed to leave alternate screen");
                        process::exit(0);
                    }
                    _ => {}
                },
            };
        }
    }
}
