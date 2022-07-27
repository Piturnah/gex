use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style::{self, Attribute, Color},
    terminal::{self, ClearType},
};
use nom::{bytes::complete::tag, IResult};
use std::{
    fmt, fs,
    io::stdout,
    process::{self, Command},
};

#[derive(Debug, Default)]
struct Status {
    branch: String,
    untracked: Vec<Item>,
    unstaged: Vec<Item>,
    staged: Vec<Item>,
    cursor: usize,
}

#[derive(Debug, Default)]
struct Item {
    path: String,
    expanded: bool,
}

impl Item {
    fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            expanded: false,
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}{}{}",
            cursor::MoveToColumn(0),
            match self.expanded {
                true => "⌄",
                false => "›",
            },
            self.path,
        )?;
        if self.expanded {
            if let Ok(file_content) = fs::read_to_string(&self.path) {
                let file_content: String = file_content
                    .lines()
                    .collect::<Vec<&str>>()
                    .join(&format!("\n{}+ ", cursor::MoveToColumn(0)));

                write!(
                    f,
                    "\n{}{}{}+ {}",
                    Attribute::Reset,
                    cursor::MoveToColumn(0),
                    style::SetForegroundColor(Color::DarkGreen),
                    file_content
                )?;
            }
        }
        Ok(())
    }
}

impl Status {
    fn new() -> Self {
        let mut status = Self::default();
        status.fetch();
        status
    }

    fn fetch(&mut self) {
        let output = Command::new("git")
            .arg("status")
            .output()
            .expect("failed to execute `git status`");

        let input = std::str::from_utf8(&output.stdout).unwrap();

        let mut lines = input.lines();
        let branch_line = lines.next().expect("not a valid `git status` output");
        let branch: IResult<&str, &str> = tag("On branch ")(branch_line);
        let (branch, _) = branch.unwrap();

        let mut untracked = Vec::new();
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        while let Some(line) = lines.next() {
            if line == "Untracked files:" {
                lines.next().unwrap(); // Skip message from git
                'untrackeds: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'untrackeds;
                    }
                    untracked.push(Item::new(line.trim_start()));
                }
            } else if line == "Changes to be committed:" {
                lines.next().unwrap(); // Skip message from git
                'staged: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'staged;
                    }
                    staged.push(Item::new(
                        line.trim_start()
                            .strip_prefix("modified:")
                            .unwrap_or_else(|| line.trim_start().strip_prefix("new file:").unwrap())
                            .trim_start(),
                    ));
                }
            } else if line == "Changes not staged for commit:" {
                lines.next().unwrap(); // Skip message from git
                lines.next().unwrap();
                'unstaged: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'unstaged;
                    }
                    unstaged.push(Item::new(
                        line.trim_start()
                            .strip_prefix("modified:")
                            .unwrap()
                            .trim_start(),
                    ));
                }
            }
        }

        self.branch = branch.to_string();
        self.untracked = untracked;
        self.staged = staged;
        self.unstaged = unstaged;
    }

    fn get_mut(&mut self, mut index: usize) -> &mut Item {
        if index >= self.untracked.len() {
            index -= self.untracked.len();
            if index >= self.unstaged.len() {
                index -= self.unstaged.len();
                return self.staged.get_mut(index).unwrap();
            }
            return self.unstaged.get_mut(index).unwrap();
        }
        return self.untracked.get_mut(index).unwrap();
    }

    fn stage(&mut self) {
        let item = self.get_mut(self.cursor);
        Command::new("git")
            .args(["add", &item.path])
            .output()
            .expect("failed to run `git add`");
        self.fetch();
    }

    fn unstage(&mut self) {
        let item = self.get_mut(self.cursor);
        Command::new("git")
            .args(["restore", "--staged", &item.path])
            .output()
            .expect("failed to run `git restore --staged`");
        self.fetch();
    }

    fn expand(&mut self) {
        let mut item = self.get_mut(self.cursor);
        item.expanded = !item.expanded;
    }

    fn len(&self) -> usize {
        self.untracked.len() + self.unstaged.len() + self.staged.len()
    }
}

impl fmt::Display for Status {
    // NOTE: Intended for use in raw mode, hence `writeln!` cannot be used.
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}On branch {}\n\n",
            cursor::MoveToColumn(0),
            self.branch,
        )?;

        write!(
            f,
            "{}{}Untracked files:{}\n",
            cursor::MoveToColumn(0),
            style::SetForegroundColor(Color::Yellow),
            style::ResetColor
        )?;
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

        write!(
            f,
            "\n{}{}Changed files:{}\n",
            cursor::MoveToColumn(0),
            style::SetForegroundColor(Color::Yellow),
            style::ResetColor
        )?;
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

        write!(
            f,
            "\n{}{}Staged for commit:{}\n",
            cursor::MoveToColumn(0),
            style::SetForegroundColor(Color::Yellow),
            style::ResetColor
        )?;
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
                KeyCode::Char('j') | KeyCode::Down => {
                    status.cursor += 1;
                    if status.cursor >= status.len() {
                        status.cursor = status.len() - 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    status.cursor = status.cursor.checked_sub(1).unwrap_or(0)
                }
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
                KeyCode::Char('q') => {
                    terminal::disable_raw_mode().unwrap();
                    crossterm::execute!(stdout(), terminal::LeaveAlternateScreen)
                        .expect("failed to leave alternate screen");
                    print!("{}", cursor::Show);
                    process::exit(0);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
