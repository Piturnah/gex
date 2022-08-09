//! Module relating to the Status diplay, including diffs of files.
use crossterm::{
    cursor,
    style::{self, Attribute, Color},
};
use nom::{
    bytes::complete::{tag, take_until},
    IResult,
};
use std::{
    fmt, fs,
    io::Write,
    process::{Command, Stdio},
};

use crate::{parse, Expand};

#[derive(Debug)]
enum DiffType {
    Modified,
    Created,
    Untracked,
    Renamed,
    Deleted,
}

#[derive(Debug)]
struct Hunk {
    diffs: Vec<String>,
    expanded: bool,
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

impl Hunk {
    fn new(diffs: Vec<String>) -> Self {
        Self {
            diffs,
            expanded: false,
        }
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

#[derive(Debug)]
pub struct FileDiff {
    path: String,
    expanded: bool,
    diff: Vec<Hunk>,
    cursor: usize,
    kind: DiffType,
}

impl fmt::Display for FileDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}{}{}{}",
            cursor::MoveToColumn(0),
            match self.expanded {
                true => "⌄",
                false => "›",
            },
            match self.kind {
                DiffType::Renamed => "[RENAME] ",
                DiffType::Deleted => "[DELETE] ",
                _ => "",
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
                    for (i, hunk) in self.diff.iter().enumerate() {
                        if i + 1 == self.cursor {
                            write!(f, "{}{}{}", Attribute::Reset, Attribute::Reverse, hunk)?;
                        } else {
                            write!(f, "{}{}", Attribute::Reset, hunk)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl FileDiff {
    fn new(path: &str, kind: DiffType) -> Self {
        Self {
            path: path.to_string(),
            expanded: false,
            diff: Vec::new(),
            cursor: 0,
            kind,
        }
    }

    /// Fails on the case that we are already on the first hunk
    fn up(&mut self) -> Result<(), ()> {
        match self.cursor.checked_sub(1) {
            Some(val) => {
                self.cursor = val;
                return Ok(());
            }
            None => return Err(()),
        }
    }

    /// Fails on the case that we are already on the final hunk
    fn down(&mut self) -> Result<(), ()> {
        self.cursor += 1;
        if self.cursor >= self.len() {
            return Err(());
        }

        Ok(())
    }

    fn len(&self) -> usize {
        match self.expanded {
            true => self.diff.len() + 1,
            false => 1,
        }
    }
}

impl Expand for FileDiff {
    fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    fn expanded(&self) -> bool {
        self.expanded
    }
}

// Enum for `Status.stage_or_unstage`
#[derive(Clone, Copy)]
enum Stage {
    Add,
    Reset,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Stage::Add => "add",
                Stage::Reset => "reset",
            }
        )
    }
}

#[derive(Debug, Default)]
pub struct Status {
    pub branch: String,
    pub diffs: Vec<FileDiff>,
    pub count_untracked: usize,
    pub count_unstaged: usize,
    pub count_staged: usize,
    pub cursor: usize,
}

impl fmt::Display for Status {
    // NOTE: Intended for use in raw mode, hence `writeln!` cannot be used.
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}On branch {}{}{}\n",
            cursor::MoveToColumn(0),
            Attribute::Bold,
            self.branch,
            Attribute::Reset
        )?;

        if self.diffs.is_empty() {
            write!(
                f,
                "\n{}{}nothing to commit, working tree clean{}",
                cursor::MoveToColumn(0),
                style::SetForegroundColor(Color::Yellow),
                style::SetForegroundColor(Color::Reset)
            )?;
        }

        for (index, file) in self.diffs.iter().enumerate() {
            if index == 0 && self.count_untracked != 0 {
                write!(
                    f,
                    "\n{}{}Untracked files:{}\n",
                    cursor::MoveToColumn(0),
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            } else if index == self.count_untracked && self.count_unstaged != 0 {
                write!(
                    f,
                    "\n{}{}Unstaged files:{}\n",
                    cursor::MoveToColumn(0),
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            } else if index == self.count_untracked + self.count_unstaged {
                write!(
                    f,
                    "\n{}{}Staged files:{}\n",
                    cursor::MoveToColumn(0),
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            }

            if file.cursor == 0 && self.cursor == index {
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

        Ok(())
    }
}

impl Status {
    pub fn new() -> Self {
        let mut status = Self::default();
        status.fetch();
        status
    }

    pub fn fetch(&mut self) {
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
                    untracked.push(FileDiff::new(line.trim_start(), DiffType::Untracked));
                }
            } else if line == "Changes to be committed:" {
                lines.next().unwrap(); // Skip message from git
                'staged: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'staged;
                    }

                    let parse_result: IResult<&str, &str> = take_until("  ")(line.trim_start());
                    let (line, prefix) = parse_result.expect("strange diff output");

                    staged.push(FileDiff::new(
                        line.trim_start(),
                        match prefix {
                            "" => DiffType::Untracked,        // untracked files
                            "new file:" => DiffType::Created, // staged new files
                            "modified:" => DiffType::Modified,
                            "renamed:" => DiffType::Renamed,
                            "deleted:" => DiffType::Deleted,
                            _ => panic!("Unknown prefix: `{}`", prefix),
                        },
                    ));
                }
            } else if line == "Changes not staged for commit:" {
                lines.next().unwrap(); // Skip message from git
                lines.next().unwrap();
                'unstaged: while let Some(line) = lines.next() {
                    if line == "" {
                        break 'unstaged;
                    }

                    let parse_result: IResult<&str, &str> = take_until("  ")(line.trim_start());
                    let (line, prefix) = parse_result.expect("strange diff output");

                    unstaged.push(FileDiff::new(
                        line.trim_start(),
                        match prefix {
                            "" => DiffType::Untracked,        // untracked files
                            "new file:" => DiffType::Created, // staged new files
                            "modified:" => DiffType::Modified,
                            "renamed:" => DiffType::Renamed,
                            "deleted:" => DiffType::Deleted,
                            _ => panic!("Unknown prefix: `{}`", prefix),
                        },
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
        self.count_untracked = untracked.len();
        self.count_staged = staged.len();
        self.count_unstaged = unstaged.len();

        self.diffs = untracked;
        self.diffs.append(&mut unstaged);
        self.diffs.append(&mut staged);

        if self.diffs.len() > 0 && self.cursor >= self.diffs.len() {
            self.cursor = self.diffs.len() - 1;
        }
    }

    fn stage_or_unstage(&mut self, command: Stage) {
        if self.diffs.len() == 0 {
            return;
        }

        let file = self.diffs.get_mut(self.cursor).unwrap();
        match file.cursor {
            0 => {
                Command::new("git")
                    .args(match command {
                        Stage::Add => vec!["add", &file.path],
                        // TODO: "HEAD" breaks on initial commit
                        Stage::Reset => vec!["reset", "HEAD", &file.path],
                    })
                    .output()
                    .expect(&format!("failed to run `git {}`", command));
            }
            i => {
                let mut patch = Command::new("git")
                    .args(match command {
                        Stage::Add => ["add", "-p", &file.path],
                        Stage::Reset => ["reset", "-p", &file.path],
                    })
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("failed to spawn interactive git process");

                let mut stdin = patch.stdin.take().expect("failed to open child stdin");

                let mut bufs = Vec::with_capacity(i);
                for _ in 0..i - 1 {
                    bufs.push(b"n\n");
                }
                bufs.push(b"y\n");

                std::thread::spawn(move || {
                    for buf in bufs {
                        stdin.write_all(buf).expect("failed to patch hunk");
                    }
                });

                let _ = patch.wait();
            }
        }
        self.fetch();
    }

    pub fn stage(&mut self) {
        self.stage_or_unstage(Stage::Add);
    }

    pub fn unstage(&mut self) {
        self.stage_or_unstage(Stage::Reset);
    }

    /// Toggles expand on the selected diff item.
    pub fn expand(&mut self) {
        if self.diffs.len() == 0 {
            return;
        }

        let mut file = self.diffs.get_mut(self.cursor).unwrap();
        if file.cursor == 0 {
            file.expanded = !file.expanded;
        } else {
            file.diff[file.cursor - 1].expanded = !file.diff[file.cursor - 1].expanded;
        }
    }

    /// Move the cursor up one
    pub fn up(&mut self) {
        if self.diffs.len() == 0 {
            return;
        }

        let file = self.diffs.get_mut(self.cursor).unwrap();
        if file.up().is_err() {
            match self.cursor.checked_sub(1) {
                Some(v) => {
                    self.cursor = v;
                    let _ = self.diffs[self.cursor].up();
                }
                None => self.cursor = 0,
            }
        }
    }

    /// Move the cursor down one
    pub fn down(&mut self) {
        if self.diffs.len() == 0 {
            return;
        }

        let file = self.diffs.get_mut(self.cursor).unwrap();
        if file.down().is_err() {
            self.cursor += 1;
            if self.cursor >= self.diffs.len() {
                self.cursor = self.diffs.len() - 1;
                self.up();
            }
        }
    }
}
