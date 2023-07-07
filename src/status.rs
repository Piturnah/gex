//! Module relating to the Status display, including diffs of files.

use std::{
    fmt, fs,
    io::{stdout, Read, Write},
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context, Error, Result};
use crossterm::style::{self, Attribute, Color};
use git2::{ErrorCode::UnbornBranch, Repository};
use nom::{bytes::complete::take_until, IResult};

use crate::{
    config::Options,
    git_process,
    minibuffer::{MessageType, MiniBuffer},
    parse,
    render::{self, Renderer},
};

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

#[derive(Debug, Clone)]
pub struct Hunk {
    diff: String,
    expanded: bool,
}

impl fmt::Display for Hunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use fmt::Write;

        let mut lines = self.diff.lines();
        let Some(head) = lines.next() else {
            return Ok(());
        };
        let mut outbuf = format!(
            "{}{}{}",
            style::SetForegroundColor(Color::Blue),
            if self.expanded { "⌄" } else { "›" },
            head.replace(" @@", &format!(" @@{}", Attribute::Reset))
        );

        if self.expanded {
            for line in lines {
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
        write!(f, "{outbuf}")
    }
}

impl Hunk {
    pub const fn new(diff: String, expanded: bool) -> Self {
        Self { diff, expanded }
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
    hunks: Vec<Hunk>,
    cursor: usize,
    kind: DiffType,
    // The implementation here involving this `selected` field is awful and hacky and I can't wait
    // to refactor it out.
    selected: bool,
}

impl render::Render for FileDiff {
    fn render(&self, f: &mut Renderer) -> fmt::Result {
        use fmt::Write;
        write!(
            f,
            "\r{}{}{}{}",
            if self.expanded { "⌄" } else { "›" },
            match self.kind {
                DiffType::Renamed => "[RENAME] ",
                DiffType::Deleted => "[DELETE] ",
                _ => "",
            },
            self.path,
            Attribute::Reset,
        )?;
        if self.expanded {
            if self.hunks.is_empty() {
                if let Ok(file_content) = fs::read_to_string(&self.path) {
                    write!(f, "{}", Attribute::Reset)?;
                    for l in file_content.lines() {
                        write!(f, "\r\n{}+{l}", style::SetForegroundColor(Color::DarkGreen))?;
                    }
                    if self.selected {
                        f.insert_item_end();
                    }
                }
            } else {
                for (i, hunk) in self.hunks.iter().enumerate() {
                    if self.selected && i + 1 == self.cursor {
                        f.insert_cursor();
                        write!(f, "{}\r\n{}{hunk}", Attribute::Reset, Attribute::Reverse)?;
                        f.insert_item_end();
                    } else {
                        write!(f, "{}\r\n{hunk}", Attribute::Reset)?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl FileDiff {
    fn new(path: &str, kind: DiffType, expanded: bool) -> Self {
        Self {
            path: path.to_string(),
            hunks: Vec::new(),
            cursor: 0,
            selected: false,
            kind,
            expanded,
        }
    }

    /// Fails on the case that we are already on the first hunk
    fn up(&mut self) -> Result<(), ()> {
        self.cursor = self.cursor.checked_sub(1).ok_or(())?;
        Ok(())
    }

    /// Fails on the case that we are already on the final hunk
    fn down(&mut self) -> Result<(), ()> {
        if self.cursor + 1 >= self.len() {
            return Err(());
        }
        self.cursor += 1;
        Ok(())
    }

    /// Move the cursor to the topmost element of this `FileDiff`.
    fn cursor_first(&mut self) {
        self.cursor = 0;
    }

    /// Move the cursor to the last element of this `FileDiff`, if it is expanded.
    fn cursor_last(&mut self) {
        self.cursor = self.len() - 1;
    }

    fn len(&self) -> usize {
        if self.expanded {
            self.hunks.len() + 1
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

#[derive(Debug, Default)]
pub struct Status {
    pub branch: String,
    pub head: String,
    pub file_diffs: Vec<FileDiff>,
    pub count_untracked: usize,
    pub count_unstaged: usize,
    pub count_staged: usize,
    pub cursor: usize,
}

impl render::Render for Status {
    fn render(&self, f: &mut Renderer) -> Result<(), fmt::Error> {
        use fmt::Write;
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
                head.map(|w| format!(" {w}")).collect::<String>()
            )?;
        }

        if self.file_diffs.is_empty() {
            write!(
                f,
                "\r\n{}nothing to commit, working tree clean{}",
                style::SetForegroundColor(Color::Yellow),
                style::SetForegroundColor(Color::Reset)
            )?;
            drop(stdout().flush());
        }

        for (index, file) in self.file_diffs.iter().enumerate() {
            if index == 0 && self.count_untracked != 0 {
                writeln!(
                    f,
                    "\r\n{}Untracked files{} {}({}){}",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor,
                    style::Attribute::Dim,
                    self.count_untracked,
                    style::Attribute::Reset,
                )?;
            } else if index == self.count_untracked && self.count_unstaged != 0 {
                writeln!(
                    f,
                    "\r\n{}Unstaged changes{} {}({}){}",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor,
                    style::Attribute::Dim,
                    self.count_unstaged,
                    style::Attribute::Reset,
                )?;
            } else if index == self.count_untracked + self.count_unstaged {
                writeln!(
                    f,
                    "\r\n{}Staged changes{} {}({}){}",
                    style::SetForegroundColor(Color::Yellow),
                    style::ResetColor,
                    style::Attribute::Dim,
                    self.count_staged,
                    style::Attribute::Reset,
                )?;
            }

            if file.cursor == 0 && self.cursor == index {
                f.insert_cursor();
                write!(f, "{}", Attribute::Reverse)?;
            }
            write!(f, "\r    ")?;
            file.render(f)?;
            writeln!(f, "{}", Attribute::Reset)?;
        }

        Ok(())
    }
}

impl Status {
    pub fn new(repo: &Repository, options: &Options) -> Result<Self> {
        let mut status = Self::default();
        status.fetch(repo, options)?;
        Ok(status)
    }

    pub fn fetch(&mut self, repo: &Repository, options: &Options) -> Result<()> {
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
                // (use "git add <file>..." to include in what will be committed)
                lines.next().context("strange `git status` output")?;
                for line in lines.by_ref() {
                    if line.is_empty() {
                        break;
                    }
                    untracked.push(FileDiff::new(
                        line.trim_start(),
                        DiffType::Untracked,
                        options.auto_expand_files,
                    ));
                }
            } else if line == "Changes to be committed:" {
                // (use "git restore --staged <file>..." to unstage)
                lines.next().context("strange `git status` output")?;
                for line in lines.by_ref() {
                    if line.is_empty() {
                        break;
                    }

                    let parse_result: IResult<&str, &str> = take_until("  ")(line.trim_start());
                    let (line, prefix) = parse_result
                        .map_err(|e| e.to_owned())
                        .context("strange `git status` output")?;

                    staged.push(FileDiff::new(
                        line.trim_start(),
                        match prefix {
                            "" => DiffType::Untracked,        // untracked files
                            "new file:" => DiffType::Created, // staged new files
                            "modified:" => DiffType::Modified,
                            "renamed:" => DiffType::Renamed,
                            "deleted:" => DiffType::Deleted,
                            _ => {
                                return Err(anyhow!(
                                    "unknown file prefix in `git status` output: `{prefix}`"
                                ))
                            }
                        },
                        options.auto_expand_files,
                    ));
                }
            } else if line == "Changes not staged for commit:" {
                // (use "git add <file>..." to update what will be committed)
                // (use "git restore <file>..." to discard changes in working directory)
                lines.next().context("strange `git status` output")?;
                lines.next().context("strange `git status` output")?;
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
                            _ => {
                                return Err(anyhow!(
                                    "unknown file prefix in `git status` output: `{prefix}`"
                                ))
                            }
                        },
                        options.auto_expand_files,
                    ));
                }
            }
        }

        // Get the diff information for unstaged changes
        let diff = git_process(&["diff", "--no-ext-diff"])?;
        let diff = std::str::from_utf8(&diff.stdout).context("malformed stdout from `git diff`")?;
        let hunks = parse::parse_diff(diff)?;
        for mut file in &mut unstaged {
            if let Some(hunks) = hunks.get(file.path.as_str()) {
                file.hunks = hunks
                    .iter()
                    .map(|hunk| Hunk::new(hunk.clone(), options.auto_expand_hunks))
                    .collect();
            }
        }

        // Get the diff information for staged changes
        let staged_diff = git_process(&["diff", "--cached", "--no-ext-diff"])?;
        let staged_diff = std::str::from_utf8(&staged_diff.stdout)
            .context("malformed stdout from `git diff --cached`")?;
        let hunks = parse::parse_diff(staged_diff)?;
        for mut file in &mut staged {
            if let Some(hunks) = hunks.get(file.path.as_str()) {
                file.hunks = hunks
                    .iter()
                    .map(|hunk| Hunk::new(hunk.clone(), options.auto_expand_hunks))
                    .collect();
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

        self.file_diffs = untracked;
        self.file_diffs.append(&mut unstaged);
        self.file_diffs.append(&mut staged);

        if !self.file_diffs.is_empty() && self.cursor >= self.file_diffs.len() {
            self.cursor = self.file_diffs.len() - 1;
        }

        if let Some(file_diff) = self.file_diffs.get_mut(self.cursor) {
            file_diff.selected = true;
        }

        Ok(())
    }

    fn stage_or_unstage(&mut self, command: Stage, mini_buffer: &mut MiniBuffer) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        let file = self
            .file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;
        file.selected = false;

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
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
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

                let mut stderr_buf = String::new();
                patch
                    .stderr
                    // If I understand correctly, reading to EOF should have the added effect
                    // waiting on the child process to finish.
                    .map(|mut stderr| stderr.read_to_string(&mut stderr_buf))
                    .context("failed to read stderr of child process")??;
                mini_buffer.push(&stderr_buf, MessageType::Error);
            }
        }

        self.file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?
            .selected = true;
        Ok(())
    }

    pub fn stage(&mut self, mini_buffer: &mut MiniBuffer) -> Result<()> {
        self.stage_or_unstage(Stage::Add, mini_buffer)
    }

    pub fn unstage(&mut self, mini_buffer: &mut MiniBuffer) -> Result<()> {
        self.stage_or_unstage(Stage::Reset, mini_buffer)
    }

    /// Toggles expand on the selected diff item.
    pub fn expand(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        let mut file = self
            .file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.cursor == 0 {
            file.expanded = !file.expanded;
        } else {
            file.hunks[file.cursor - 1].expanded = !file.hunks[file.cursor - 1].expanded;
        }

        Ok(())
    }

    /// Move the cursor up one
    pub fn up(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        let file = self
            .file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.up().is_err() {
            match self.cursor.checked_sub(1) {
                Some(v) => {
                    self.cursor = v;
                    file.selected = false;
                    let new_file = self
                        .file_diffs
                        .get_mut(self.cursor)
                        .context("cursor at invalid position")?;
                    new_file.selected = true;
                    if new_file.expanded() {
                        new_file.cursor_last();
                    }
                }
                None => self.cursor = 0,
            }
        }

        Ok(())
    }

    /// Move the cursor down one
    pub fn down(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        let count_file_diffs = self.file_diffs.len();
        let file = self
            .file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;

        if file.down().is_err() {
            if self.cursor + 1 >= count_file_diffs {
                return Ok(());
            }

            self.cursor += 1;
            file.selected = false;
            let new_file = self
                .file_diffs
                .get_mut(self.cursor)
                .context("cursor at invalid position")?;
            new_file.selected = true;
            if new_file.expanded() {
                new_file.cursor_first();
            }
        }

        Ok(())
    }

    /// Move the cursor to the first element.
    pub fn cursor_first(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        self.file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?
            .selected = false;
        self.cursor = 0;
        let new_file = self
            .file_diffs
            .get_mut(self.cursor)
            .expect("0th element must exist, !self.file_diffs.is_empty()");
        new_file.cursor_first();
        new_file.selected = true;
        Ok(())
    }

    /// Move the cursor to the last element.
    pub fn cursor_last(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        self.file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?
            .selected = false;
        self.cursor = self.file_diffs.len() - 1;
        let new_file = self
            .file_diffs
            .get_mut(self.cursor)
            .expect("cursor at `len() - 1`th pos of non-empty diffs");
        new_file.cursor_last();
        new_file.selected = true;
        Ok(())
    }
}
