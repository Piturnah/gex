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
    process::{self, Command, Stdio},
};

mod parse;

#[derive(Debug, Default)]
struct Status {
    branch: String,
    untracked: Vec<File>,
    unstaged: Vec<File>,
    staged: Vec<File>,
    cursor: usize,
}

#[derive(Debug, Default)]
struct File {
    path: String,
    expanded: bool,
    diff: Vec<Hunk>,
}

#[derive(Debug)]
struct Hunk {
    diffs: Vec<String>,
    expanded: bool,
}

// TODO: Reorganise a bit
impl Hunk {
    fn new(diffs: Vec<String>) -> Self {
        Self {
            diffs,
            expanded: true,
        }
    }
}

impl File {
    fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            expanded: false,
            diff: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        match self.expanded {
            true => self.diff.len(),
            false => 0,
        }
    }
}

trait Expand {
    fn toggle_expand(&mut self);
    fn expanded(&self) -> bool;
}

impl Expand for File {
    fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    fn expanded(&self) -> bool {
        self.expanded
    }
}

impl Expand for Hunk {
    fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    fn expanded(&self) -> bool {
        self.expanded
    }
}

impl fmt::Display for File {
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
            match self.diff.is_empty() {
                true => {
                    if let Ok(file_content) = fs::read_to_string(&self.path) {
                        let file_content: String = file_content
                            .lines()
                            .collect::<Vec<&str>>()
                            .join(&format!("\n{}+ ", cursor::MoveToColumn(0)));

                        write!(
                            f,
                            "\n{}{}{}+{}",
                            Attribute::Reset,
                            cursor::MoveToColumn(0),
                            style::SetForegroundColor(Color::DarkGreen),
                            file_content
                        )?;
                    }
                }
                false => {
                    for hunk in &self.diff {
                        write!(f, "{}{}", Attribute::Reset, hunk)?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Hunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut outbuf = format!(
            "\n{}{}{}{}",
            cursor::MoveToColumn(0),
            style::SetForegroundColor(Color::Blue),
            match self.expanded {
                true => "⌄",
                false => "›",
            },
            self.diffs[0].replace(" @@", &format!(" @@{}", Attribute::Reset))
        );

        if self.expanded {
            for line in self.diffs.iter().skip(1) {
                outbuf += &format!(
                    "\n{}{}{}",
                    cursor::MoveToColumn(0),
                    match line.chars().nth(0) {
                        Some('+') => style::SetForegroundColor(Color::DarkGreen),
                        Some('-') => style::SetForegroundColor(Color::DarkRed),
                        _ => style::SetForegroundColor(Color::Reset),
                    },
                    line
                );
            }
        }
        write!(f, "{}", outbuf)
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
                    untracked.push(File::new(line.trim_start()));
                }
            } else if line == "Changes to be committed:" {
                lines.next().unwrap(); // Skip message from git
                'staged: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'staged;
                    }
                    staged.push(File::new(
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
                    unstaged.push(File::new(
                        line.trim_start()
                            .strip_prefix("modified:")
                            .unwrap()
                            .trim_start(),
                    ));
                }
            }
        }

        let diff = Command::new("git")
            .arg("diff")
            .output()
            .expect("failed to run `git diff`");
        let staged_diff = Command::new("git")
            .args(["diff", "--cached"])
            .output()
            .expect("failed to run `git diff --cached`");

        let diff = std::str::from_utf8(&diff.stdout).unwrap();
        let diffs = parse::parse_diff(&diff);
        'outer_unstaged: for (path, diff) in diffs {
            for mut file in &mut unstaged {
                if file.path == path {
                    file.diff = diff
                        .iter()
                        .map(|d| {
                            Hunk::new(
                                d.to_owned()
                                    .iter()
                                    .map(|l| l.to_string())
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>();
                    continue 'outer_unstaged;
                }
            }
        }

        let staged_diff = std::str::from_utf8(&staged_diff.stdout).unwrap();
        let diffs = parse::parse_diff(&staged_diff);
        'outer_staged: for (path, diff) in diffs {
            for mut file in &mut staged {
                if file.path == path {
                    file.diff = diff
                        .iter()
                        .map(|d| {
                            Hunk::new(
                                d.to_owned()
                                    .iter()
                                    .map(|l| l.to_string())
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>();
                    continue 'outer_staged;
                }
            }
        }

        self.branch = branch.to_string();
        self.untracked = untracked;
        self.staged = staged;
        self.unstaged = unstaged;
    }

    fn get_mut(&mut self, mut index: usize) -> &mut File {
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
        let file = self.get_mut(self.cursor);
        Command::new("git")
            .args(["add", &file.path])
            .output()
            .expect("failed to run `git add`");
        self.fetch();
    }

    fn unstage(&mut self) {
        let file = self.get_mut(self.cursor);
        Command::new("git")
            .args(["restore", "--staged", &file.path])
            .output()
            .expect("failed to run `git restore --staged`");
        self.fetch();
    }

    fn expand(&mut self) {
        let mut file = self.get_mut(self.cursor);
        file.expanded = !file.expanded;
    }

    fn len(&self) -> usize {
        self.untracked.len() + self.unstaged.len() + self.staged.len()
    }
}

impl fmt::Display for Status {
    // NOTE: Intended for use in raw mode, hence `writeln!` cannot be used.
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}On branch {}\n", cursor::MoveToColumn(0), self.branch,)?;

        if self.untracked.len() > 0 {
            write!(
                f,
                "\n{}{}Untracked files:{}\n",
                cursor::MoveToColumn(0),
                style::SetForegroundColor(Color::Yellow),
                style::ResetColor
            )?;
        }
        for (index, file) in self.untracked.iter().enumerate() {
            if self.cursor == index {
                write!(f, "{}", Attribute::Reverse)?;
            }
            writeln!(
                f,
                "{}    {}{}",
                cursor::MoveToColumn(0),
                file,
                Attribute::Reset
            )?;
        }

        if self.unstaged.len() > 0 {
            write!(
                f,
                "\n{}{}Changed files:{}\n",
                cursor::MoveToColumn(0),
                style::SetForegroundColor(Color::Yellow),
                style::ResetColor
            )?;
        }
        for (index, file) in self.unstaged.iter().enumerate() {
            if self.cursor
                == index
                    + self.untracked.len()
                    + self
                        .unstaged
                        .iter()
                        .take(index)
                        .map(|file| file.len())
                        .sum::<usize>()
            {
                write!(f, "{}", Attribute::Reverse)?;
            }
            writeln!(
                f,
                "{}    {}{}",
                cursor::MoveToColumn(0),
                file,
                Attribute::Reset
            )?;
        }

        if self.staged.len() > 0 {
            write!(
                f,
                "\n{}{}Staged for commit:{}\n",
                cursor::MoveToColumn(0),
                style::SetForegroundColor(Color::Yellow),
                style::ResetColor
            )?;
        }
        for (index, file) in self.staged.iter().enumerate() {
            if self.cursor == index + self.untracked.len() + self.unstaged.len() {
                write!(f, "{}", Attribute::Reverse)?;
            }
            write!(
                f,
                "{}    {}{}\n",
                cursor::MoveToColumn(0),
                file,
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
