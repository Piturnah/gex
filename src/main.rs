use std::{
    io::{stdin, stdout, BufRead, Write},
    path::Path,
    process::{self, Command, Output, Stdio},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{Attribute, Color, SetForegroundColor},
    terminal::{self, ClearType},
};
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

pub fn git_process(args: &[&str]) -> Output {
    Command::new("git").args(args).output().unwrap_or_else(|_| {
        panic!(
            "failed to run `git{}`",
            args.iter().map(|a| " ".to_string() + a).collect::<String>()
        )
    })
}

fn main() {
    clap::command!()
        .color(clap::ColorChoice::Never)
        .get_matches();

    let top_level_stdout = git_process(&["rev-parse", "--show-toplevel"]).stdout;

    let top_level_stdout = Path::new(
        std::str::from_utf8(&top_level_stdout)
            .expect("`git rev-parse` did not give valid utf-8")
            .trim_end(),
    );

    if top_level_stdout.is_dir() {
        std::env::set_current_dir(top_level_stdout).expect("failed to set working directory");
    } else {
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
    let mut msg_buffer_height = 0;

    loop {
        let (term_width, term_height) =
            terminal::size().expect("failed to query terminal dimensions");

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
            terminal::disable_raw_mode().unwrap();

            match output.status.success() {
                true => {
                    // NOTE: I am still unsure if we want to propagate stdout on success. I fear
                    // that it may clutter the UI and a successful change should be communicated
                    // through seeing the results in gex anyway.
                    let git_msg = std::str::from_utf8(&output.stdout)
                        .unwrap()
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
                    let git_msg = std::str::from_utf8(&output.stderr).unwrap().trim();
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

            terminal::enable_raw_mode().unwrap();
            let _ = stdout().flush();

            git_output = None;
        } else {
            msg_buffer_height = 0;
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
                        state = State::Commit;
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
                    KeyCode::Char(':') => {
                        terminal::disable_raw_mode().expect("failed to disable raw mode");

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
                            .expect("no stdin")
                            .expect("malformed stdin");

                        git_output =
                            Some(git_process(&input.split_whitespace().collect::<Vec<_>>()));

                        print!("{}", cursor::Hide);
                        terminal::enable_raw_mode().expect("failed to enable raw mode");
                        status.fetch();
                    }
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
                State::Commit => match event.code {
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

                        state = State::Status;
                    }
                    KeyCode::Char('e') => {
                        git_output = Some(
                            Command::new("git")
                                .args(["commit", "--amend", "--no-edit"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .expect("failed to run `git commit`"),
                        );
                        status.fetch();

                        state = State::Status;
                    }
                    KeyCode::Char('a') => {
                        crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                            .expect("failed to leave alternate screen");
                        git_output = Some(
                            Command::new("git")
                                .args(["commit", "--amend"])
                                .stdout(Stdio::inherit())
                                .stdin(Stdio::inherit())
                                .output()
                                .expect("failed to run `git commit`"),
                        );
                        status.fetch();
                        crossterm::execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)
                            .expect("failed to enter alternate screen");

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
