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

pub mod parse;
mod status;

use status::Status;

pub trait Expand {
    fn toggle_expand(&mut self);
    fn expanded(&self) -> bool;
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
    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .expect("failed to enter alternate screen");
    terminal::enable_raw_mode().expect("failed to put terminal in raw mode");
    print!("{}", cursor::Hide);
    loop {
        println!(
            "{}{}{}{}",
            cursor::MoveToRow(0),
            terminal::Clear(ClearType::All),
            status,
            cursor::MoveToColumn(0)
        );
        match event::read().unwrap() {
            Event::Key(event) => match event.code {
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
                KeyCode::Char('r') => status.fetch(),
                KeyCode::Char('q') | KeyCode::Esc => {
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
            _ => {}
        }
    }
}
