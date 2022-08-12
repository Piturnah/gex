use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    terminal::{self, ClearType},
};
use std::{
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Stdio},
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

fn main() {
    if !Path::new("./.git").is_dir() {
        print!("Not a git repository. Initialise one? [y/N]");
        let _ = stdout().flush();
        if let Some(Ok(input)) = stdin().lock().lines().next() {
            if input.to_lowercase() != "y" {
                process::exit(0);
            }

            Command::new("git")
                .arg("init")
                .output()
                .expect("failed to run `git init`");
        }
    }

    let mut status = Status::new();
    let mut branch_list = BranchList::new();

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
            }
        }
        if let Event::Key(event) = event::read().unwrap() {
            match state {
                State::Status => match event.code {
                    KeyCode::Char('j') | KeyCode::Down => status.down(),
                    KeyCode::Char('k') | KeyCode::Up => status.up(),
                    KeyCode::Char('s') => status.stage(),
                    KeyCode::Char('S') => {
                        Command::new("git")
                            .args(["add", "."])
                            .output()
                            .expect("couldn't run `git add .`");
                        status.fetch();
                    }
                    KeyCode::Char('u') => status.unstage(),
                    KeyCode::Char('U') => {
                        Command::new("git")
                            .arg("reset")
                            .output()
                            .expect("failed to run `git reset`");
                        status.fetch();
                    }
                    KeyCode::Tab => status.expand(),
                    KeyCode::Char('c') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .expect("failed to leave alternate screen");
                        Command::new("git")
                            .arg("commit")
                            .stdout(Stdio::inherit())
                            .stdin(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .output()
                            .expect("failed to run `git commit`");
                        status.fetch();
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .expect("failed to enter alternate screen");
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
                        branch_list.cursor = branch_list.cursor.checked_sub(1).unwrap_or(0);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        branch_list.cursor += 1;
                        if branch_list.cursor >= branch_list.branches.len() {
                            branch_list.cursor = branch_list.branches.len() - 1;
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        branch_list.checkout();
                        status.fetch();
                        state = State::Status;
                    }
                    KeyCode::Char('b') => {
                        BranchList::checkout_new();
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
