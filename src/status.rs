//! Module relating to the Status display, including diffs of files.

use std::{
    fmt, fs,
    io::{stdout, Write},
    process::{Command, Stdio},
};

use anyhow::{Context, Error, Result};
use crossterm::style::{self, Attribute, Color};
use git2::{ErrorCode::UnbornBranch, Repository};
use nom::{bytes::complete::take_until, IResult};

use crate::{git_process, parse};

pub trait Expand {
    fn toggle_expand(&mut self);
    fn expanded(&self) -> bool;
}

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
        use fmt::Write;
        let mut outbuf = format!(
            "\r\n{}{}{}",
            style::SetForegroundColor(Color::Blue),
            if self.expanded { "⌄" } else { "›" },
            self.diffs[0].replace(" @@", &format!(" @@{}", Attribute::Reset))
        );

        if self.expanded {
            for line in self.diffs.iter().skip(1) {
                write!(
                    &mut outbuf,
                    "\r\n{}{}",
                    match line.chars().next() {
                        Some('+') => style::SetForegroundColor(Color::DarkGreen),
                        Some('-') => style::SetForegroundColor(Color::DarkRed),
                        _ => style::SetForegroundColor(Color::Reset),
                    },
                    line
                )?;
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
            "\r{}{}{}",
            if self.expanded { "⌄" } else { "›" },
            match self.kind {
                DiffType::Renamed => "[RENAME] ",
                DiffType::Deleted => "[DELETE] ",
                _ => "",
            },
            self.path,
        )?;
        if self.expanded {
            if self.diff.is_empty() {
                if let Ok(file_content) = fs::read_to_string(&self.path) {
                    let file_content: String =
                        file_content.lines().collect::<Vec<&str>>().join("\r\n+");

                    write!(
                        f,
                        "\r\n{}{}+{}",
                        Attribute::Reset,
                        style::SetForegroundColor(Color::DarkGreen),
                        file_content
                    )?;
                }
            } else {
                for (i, hunk) in self.diff.iter().enumerate() {
                    if i + 1 == self.cursor {
                        write!(f, "{}{}{}", Attribute::Reset, Attribute::Reverse, hunk)?;
                    } else {
                        write!(f, "{}{}", Attribute::Reset, hunk)?;
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
                Ok(())
            }
            None => Err(()),
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

    /// Move the cursor to the topmost element of this FileDiff.
    fn cursor_first(&mut self) {
        self.cursor = 0;
    }

    /// Move the cursor to the last element of this FileDiff, if it is expanded.
    fn cursor_last(&mut self) {
        self.cursor = self.len() - 1;
    }

    fn len(&self) -> usize {
        if self.expanded {
            self.diff.len() + 1
        } else {
            1
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
    pub head: String,
    pub diffs: Vec<FileDiff>,
    pub count_untracked: usize,
    pub count_unstaged: usize,
    pub count_staged: usize,
    pub cursor: usize,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // Display the current branch
        writeln!(
            f,
            "\rOn branch {}{}{}",
            Attribute::Bold,
            self.branch,
            Attribute::Reset
        )?;

        // Display most recent commit
        if !self.head.is_empty() {
            let mut head = self.head.split_whitespace();
            writeln!(
                f,
                "{}\r\n{}{}{}",
                Attribute::Dim,
                head.next().unwrap(), // !self.head.is_empty()
                Attribute::Reset,
                head.map(|w| format!(" {}", w)).collect::<String>()
            )?;
        }

        if self.diffs.is_empty() {
            write!(
                f,
                "\r\n{}nothing to commit, working tree clean{}",
                style::SetForegroundColor(Color::Yellow),
                style::SetForegroundColor(Color::Reset)
            )?;
            let _ = stdout().flush();
        }

        for (index, file) in self.diffs.iter().enumerate() {
            if index == 0 && self.count_untracked != 0 {
                write!(
                    f,
                    "\r\n{}Untracked files:{}\n",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            } else if index == self.count_untracked && self.count_unstaged != 0 {
                write!(
                    f,
                    "\r\n{}Unstaged changes:{}\n",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            } else if index == self.count_untracked + self.count_unstaged {
                write!(
                    f,
                    "\r\n{}Staged changes:{}\n",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor
                )?;
            }

            if file.cursor == 0 && self.cursor == index {
                write!(f, "{}", Attribute::Reverse)?;
            }
            writeln!(f, "\r    {}{}", file, Attribute::Reset)?;
        }

        Ok(())
    }
}

impl Status {
    pub fn new(repo: &Repository) -> Result<Self> {
        let mut status = Self::default();
        status.fetch(repo)?;
        Ok(status)
    }

    pub fn fetch(&mut self, repo: &Repository) -> Result<()> {
        let output = git_process(&["status"])?;

        let input =
            std::str::from_utf8(&output.stdout).context("malformed stdout from `git status`")?;

        // TODO: When head().is_branch() is false, we should do something different. For example,
        // use `branch: Option<String>` in `Status` and display something different when head
        // detached or on tag, etc.
        let branch = match repo.head() {
            Ok(head) => head
                .shorthand()
                .context("no name found for current HEAD")?
                .to_string(),
            Err(e) => {
                // git2 doesn't provide any API to get the name of an unborn branch, so we have to
                // read it directly :(
                if e.code() == UnbornBranch {
                    let mut head_path = repo.path().to_path_buf();
                    head_path.push("HEAD");
                    fs::read_to_string(&head_path)
                        .with_context(|| format!("couldn't read file: {head_path:?}"))?
                        .lines()
                        .next()
                        .with_context(|| format!("no ref found in {head_path:?}"))?
                        .trim()
                        .strip_prefix("ref: refs/heads/")
                        .with_context(|| format!("unexpected ref path found in {head_path:?}"))?
                        .to_string()
                } else {
                    return Err(Error::new(e)).context("failed to get name of current branch");
                }
            }
        };

        let mut untracked = Vec::new();
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();

        let mut lines = input.lines();
        while let Some(line) = lines.next() {
            if line == "Untracked files:" {
                lines.next().unwrap(); // Skip message from git
                for line in lines.by_ref() {
                    if line.is_empty() {
                        break;
                    }
                    untracked.push(FileDiff::new(line.trim_start(), DiffType::Untracked));
                }
            } else if line == "Changes to be committed:" {
                lines.next().unwrap(); // Skip message from git
                for line in lines.by_ref() {
                    if line.is_empty() {
                        break;
                    }

                    let parse_result: IResult<&str, &str> = take_until("  ")(line.trim_start());
                    let (line, prefix) = parse_result
                        .map_err(|e| e.to_owned())
                        .context("strange `git diff` output")?;

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
                for line in lines.by_ref() {
                    if line.is_empty() {
                        break;
                    }

                    let parse_result: IResult<&str, &str> = take_until("  ")(line.trim_start());
                    let (line, prefix) = parse_result
                        .map_err(|e| e.to_owned())
                        .context("strange diff output")?;

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

        let diff = git_process(&["diff"])?;
        let staged_diff = git_process(&["diff", "--cached"])?;

        let diff = std::str::from_utf8(&diff.stdout).context("malformed stdout from `git diff`")?;
        let diffs = parse::parse_diff(diff)?;
        for mut file in unstaged.iter_mut() {
            if let Some(diff) =
                diffs
                    .iter()
                    .find(|(path, _)| ***path == file.path)
                    .map(|(_, diff)| {
                        diff.iter()
                            .map(|hunk_lines| {
                                Hunk::new(
                                    hunk_lines
                                        .to_owned()
                                        .iter()
                                        .map(|l| l.to_string())
                                        .collect::<Vec<_>>(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
            {
                file.diff = diff
            }
        }

        let staged_diff = std::str::from_utf8(&staged_diff.stdout)
            .context("malformed stdout from `git diff --cached`")?;
        let diffs = parse::parse_diff(staged_diff)?;
        for mut file in staged.iter_mut() {
            if let Some(diff) =
                diffs
                    .iter()
                    .find(|(path, _)| ***path == file.path)
                    .map(|(_, diff)| {
                        diff.iter()
                            .map(|hunk_lines| {
                                Hunk::new(
                                    hunk_lines
                                        .to_owned()
                                        .iter()
                                        .map(|l| l.to_string())
                                        .collect::<Vec<_>>(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
            {
                file.diff = diff
            }
        }

        self.branch = branch;
        self.head = std::str::from_utf8(
            &git_process(&["log", "HEAD", "--pretty=format:%h %s", "-n", "1"])?.stdout,
        )
        .context("invalid utf8 from `git log`")?
        .to_string();
        self.count_untracked = untracked.len();
        self.count_staged = staged.len();
        self.count_unstaged = unstaged.len();

        self.diffs = untracked;
        self.diffs.append(&mut unstaged);
        self.diffs.append(&mut staged);

        if !self.diffs.is_empty() && self.cursor >= self.diffs.len() {
            self.cursor = self.diffs.len() - 1;
        }

        Ok(())
    }

    fn stage_or_unstage(&mut self, command: Stage) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        let file = self
            .diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        match file.cursor {
            0 => {
                let args = match command {
                    Stage::Add => vec!["add", &file.path],
                    Stage::Reset => match file.kind {
                        DiffType::Deleted => vec!["reset", "HEAD", &file.path],
                        _ => vec!["reset", &file.path],
                    },
                };
                git_process(&args)?;
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
                    .context("failed to spawn interactive git process")?;

                let mut stdin = patch.stdin.take().context("failed to open child stdin")?;

                let mut bufs = vec![b"n\n"; i - 1];
                bufs.push(b"y\n");

                std::thread::spawn(move || {
                    for buf in bufs {
                        stdin.write_all(buf).context("failed to patch hunk")?;
                    }
                    Ok::<_, Error>(())
                })
                .join()
                .unwrap()
                .context("failed to patch hunk")?;

                let _ = patch.wait();
            }
        }
        Ok(())
    }

    pub fn stage(&mut self) -> Result<()> {
        self.stage_or_unstage(Stage::Add)
    }

    pub fn unstage(&mut self) -> Result<()> {
        self.stage_or_unstage(Stage::Reset)
    }

    /// Toggles expand on the selected diff item.
    pub fn expand(&mut self) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        let mut file = self
            .diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.cursor == 0 {
            file.expanded = !file.expanded;
        } else {
            file.diff[file.cursor - 1].expanded = !file.diff[file.cursor - 1].expanded;
        }

        Ok(())
    }

    /// Move the cursor up one
    pub fn up(&mut self) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        let file = self
            .diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.up().is_err() {
            match self.cursor.checked_sub(1) {
                Some(v) => {
                    self.cursor = v;
                    let _ = self.diffs[self.cursor].up();
                }
                None => self.cursor = 0,
            }
        }

        Ok(())
    }

    /// Move the cursor down one
    pub fn down(&mut self) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        let file = self
            .diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.down().is_err() {
            self.cursor += 1;
            if self.cursor >= self.diffs.len() {
                self.cursor = self.diffs.len() - 1;
                self.up()?;
            }
        }

        Ok(())
    }

    /// Move the cursor to the first element.
    pub fn cursor_first(&mut self) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        self.diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?
            .cursor_first();
        self.cursor = 0;
        self.diffs
            .get_mut(self.cursor)
            .expect("0th element must exist")
            .cursor_first();
        Ok(())
    }

    /// Move the cursor to the last element.
    pub fn cursor_last(&mut self) -> Result<()> {
        if self.diffs.is_empty() {
            return Ok(());
        }

        let mut file = self
            .diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;
        file.cursor = file.len();
        self.cursor = self.diffs.len() - 1;
        self.diffs
            .get_mut(self.cursor)
            .expect("cursor at `len() - 1`th pos of non-empty diffs")
            .cursor_last();
        Ok(())
    }
}
