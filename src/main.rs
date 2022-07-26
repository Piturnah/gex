use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{self, Attribute, Color, Colors},
    terminal::{self, ClearType},
};
use nom::{
    bytes::complete::{tag, take_till},
    character::is_newline,
    error::Error,
    IResult,
};
use std::{
    fmt, fs,
    process::{self, Command},
};

#[derive(Debug, Default)]
struct Status<'a> {
    branch: &'a str,
    untracked: Vec<Item<'a>>,
    unstaged: Vec<Item<'a>>,
    staged: Vec<Item<'a>>,
    cursor: usize,
}

#[derive(Debug, Default)]
struct Item<'a> {
    path: &'a str,
    expanded: bool,
}

impl<'a> Item<'a> {
    fn new(path: &'a str) -> Self {
        Self {
            path,
            expanded: false,
        }
    }
}

impl<'a> fmt::Display for Item<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}   {} {}",
            cursor::MoveToColumn(0),
            match self.expanded {
                true => "v",
                false => "-",
            },
            self.path,
        )?;
        if self.expanded {
            if let Ok(file_content) = fs::read_to_string(self.path) {
                let file_content: String = file_content
                    .lines()
                    .collect::<Vec<&str>>()
                    .join(&format!("\n{}", cursor::MoveToColumn(0)));

                write!(
                    f,
                    "\n{}{}{}{}",
                    Attribute::Reset,
                    cursor::MoveToColumn(0),
                    style::SetColors(Colors::new(Color::Black, Color::DarkGreen)),
                    file_content
                )?;
            }
        }
        Ok(())
    }
}

impl<'a> Status<'a> {
    fn parse(input: &'a str) -> IResult<&str, Self> {
        let branch_line = input
            .lines()
            .next()
            .expect("not a valid `git status` output");
        let (branch, _) = tag("On branch ")(branch_line)?;

        Ok((
            "",
            Status {
                branch,
                ..Default::default()
            },
        ))
    }

    fn expand(&mut self) {
        let mut index = self.cursor;
        if self.cursor >= self.untracked.len() {
            index -= self.untracked.len();
            self.staged[index].expanded = !self.staged[index].expanded;
        }
        self.untracked[index].expanded = !self.untracked[index].expanded;
    }

    fn len(&self) -> usize {
        self.untracked.len() + self.unstaged.len() + self.staged.len()
    }
}

impl<'a> fmt::Display for Status<'a> {
    // NOTE: Intended for use in raw mode, hence `writeln!` cannot be used.
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}On branch {}\n\n",
            cursor::MoveToColumn(0),
            self.branch,
        )?;

        write!(f, "{}Untracked files:\n", cursor::MoveToColumn(0))?;
        for (index, path) in self.untracked.iter().enumerate() {
            if self.cursor == index {
                write!(f, "{}", Attribute::Reverse)?;
            }
            writeln!(
                f,
                "{}    {}{}",
                cursor::MoveToColumn(0),
                path,
                Attribute::Reset
            )?;
        }

        write!(f, "\n{}Changed files:\n", cursor::MoveToColumn(0))?;
        for (index, path) in self.unstaged.iter().enumerate() {
            if self.cursor == index + self.untracked.len() {
                write!(f, "{}", Attribute::Reverse)?;
            }
            writeln!(
                f,
                "{}    {}{}",
                cursor::MoveToColumn(0),
                path,
                Attribute::Reset
            )?;
        }

        write!(f, "\n{}Staged for commit:\n", cursor::MoveToColumn(0))?;
        for (index, path) in self.staged.iter().enumerate() {
            if self.cursor == index + self.untracked.len() + self.unstaged.len() {
                write!(f, "{}", Attribute::Reverse)?;
            }
            write!(
                f,
                "{}    {}{}\n",
                cursor::MoveToColumn(0),
                path,
                Attribute::Reset
            )?;
        }

        Ok(())
    }
}

fn main() {
    let mut status = Status {
        branch: "main",
        untracked: vec![
            Item::new(".gitignore"),
            Item::new("Cargo.toml"),
            Item::new("src/"),
        ],
        staged: vec![Item::new("Cargo.lock")],
        ..Default::default()
    };

    terminal::enable_raw_mode().expect("couldn't put terminal in raw mode");
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
                KeyCode::Char('j') | KeyCode::Down => {
                    status.cursor += 1;
                    if status.cursor >= status.len() {
                        status.cursor = status.len() - 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    status.cursor = status.cursor.checked_sub(1).unwrap_or(0)
                }
                KeyCode::Tab => status.expand(),
                KeyCode::Char('q') => {
                    terminal::disable_raw_mode().unwrap();
                    print!("{}", cursor::Show);
                    process::exit(0);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
