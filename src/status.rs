//! Module relating to the Status display, including diffs of files.

use std::{
    borrow::Cow,
    fmt, fs,
    io::{stdout, Read, Write},
    process::{Command, Output, Stdio},
};

use anyhow::{anyhow, Context, Error, Result};
use crossterm::style::{self, Attribute};
use git2::{ErrorCode::UnbornBranch, Repository};
use nom::{bytes::complete::take_until, IResult};

use crate::{
    config::{Config, Options, CONFIG},
    git_process,
    highlight::{highlight_hunk, DiffHighlighter},
    minibuffer::{MessageType, MiniBuffer},
    parse::{self, parse_hunk_new, parse_hunk_old},
    render::{self, Renderer, ResetAttributes, ResetColor},
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

// TODO(cptp): move hunk structs to `hunk.rs`

/// A hunk displayed in the UI.
#[derive(Debug, Clone)]
pub struct HunkView {
    /// The raw diff of the hunk.
    diff: String,
    /// The highlighted diff of the hunk.
    diff_highlighted: String,
    /// The hunk is currently expanded in the UI.
    expanded: bool,
}

impl HunkView {
    pub const fn new(diff: String, diff_highlighted: String, expanded: bool) -> Self {
        Self {
            diff,
            diff_highlighted,
            expanded,
        }
    }

    pub const fn display(&self, highlighted: bool) -> HunkDisplay {
        HunkDisplay(self, highlighted)
    }
}

impl Expand for HunkView {
    fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    fn expanded(&self) -> bool {
        self.expanded
    }
}

/// Helper struct for [`HunkView`] that implements Display.
///
/// Allows switching between the highlighted and non highlighted version.
#[derive(Clone, Copy)]
pub struct HunkDisplay<'a>(&'a HunkView, bool);

impl<'a> fmt::Display for HunkDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use fmt::Write;
        let config = CONFIG.get().expect("config wasn't initialised");
        let hunk = self.0;
        let highlight = self.1;

        let mut lines = (if highlight {
            &hunk.diff_highlighted
        } else {
            &hunk.diff
        })
        .lines();

        let Some(head) = lines.next() else {
            return Ok(());
        };
        let mut outbuf = format!(
            "{}{}{}",
            style::SetForegroundColor(config.colors.hunk_head),
            if hunk.expanded { "⌄" } else { "›" },
            head.replace(" @@", &format!(" @@{ResetAttributes}"))
        );

        if hunk.expanded {
            for line in lines {
                write!(
                    &mut outbuf,
                    "\r\n{}{line}",
                    style::SetForegroundColor(style::Color::DarkGrey)
                )?;
            }
            // TODO(cptp): highlight white spaces?
            // let ws_error_highlight = CONFIG
            //     .get()
            //     .expect("config is initialised at the start of the program")
            //     .options
            //     .ws_error_highlight;
            // for line in lines {
            //     write!(
            //         &mut outbuf,
            //         "\r\n{}",
            //         if ws_error_highlight.context {
            //             format_trailing_whitespace(&line[1..], config)
            //         } else {
            //             Cow::Borrowed(&line[1..])
            //         }
            //     )?
            // }
        }
        write!(f, "{outbuf}")
    }
}

fn format_trailing_whitespace<'s>(s: &'s str, config: &'_ Config) -> Cow<'s, str> {
    let count_trailing_whitespace = s
        .bytes()
        .rev()
        .take_while(|c| c.is_ascii_whitespace())
        .count();
    if count_trailing_whitespace > 0 {
        Cow::Owned({
            let mut line = s.to_string();
            line.insert_str(
                line.len() - count_trailing_whitespace,
                &format!("{}", style::SetBackgroundColor(config.colors.error)),
            );
            line
        })
    } else {
        Cow::Borrowed(s)
    }
}

#[derive(Debug)]
pub struct FileDiff {
    path: String,
    expanded: bool,
    hunks: Vec<HunkView>,
    cursor: usize,
    kind: DiffType,
    // The implementation here involving this `selected` field is awful and hacky and I can't wait
    // to refactor it out.
    selected: bool,
}

impl render::Render for FileDiff {
    fn render(&self, f: &mut Renderer) -> fmt::Result {
        use fmt::Write;
        let config = CONFIG.get().expect("config wasn't initialised");
        write!(
            f,
            "\r{}{}{}{ResetAttributes}",
            if self.expanded { "⌄" } else { "›" },
            match self.kind {
                DiffType::Renamed => "[RENAME] ",
                DiffType::Deleted => "[DELETE] ",
                _ => "",
            },
            self.path,
        )?;
        if self.expanded {
            if self.hunks.is_empty() {
                if let Ok(file_content) = fs::read_to_string(&self.path) {
                    let ws_error_highlight = config.options.ws_error_highlight;

                    write!(f, "{ResetAttributes}")?;
                    for l in file_content.lines() {
                        write!(
                            f,
                            "\r\n{}+{l}",
                            style::SetForegroundColor(config.colors.addition),
                            l = if ws_error_highlight.new {
                                format_trailing_whitespace(l, config)
                            } else {
                                Cow::Borrowed(l)
                            }
                        )?;
                    }
                    if self.selected {
                        f.insert_item_end();
                    }
                }
            } else {
                for (i, hunk) in self.hunks.iter().enumerate() {
                    if self.selected && i + 1 == self.cursor {
                        f.insert_cursor();
                        write!(
                            f,
                            "{ResetAttributes}\r\n{}{}",
                            Attribute::Reverse,
                            hunk.display(true)
                        )?;
                        f.insert_item_end();
                    } else {
                        write!(f, "{ResetAttributes}\r\n{}", hunk.display(false))?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl FileDiff {
    fn new(path: &str, kind: DiffType, expanded: bool, cursor: usize) -> Self {
        Self {
            path: path.to_string(),
            hunks: Vec::new(),
            selected: false,
            kind,
            expanded,
            cursor,
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

#[derive(Debug)]
pub struct Status {
    pub branch: String,
    pub head: String,
    pub file_diffs: Vec<FileDiff>,
    pub count_untracked: usize,
    pub count_unstaged: usize,
    pub count_staged: usize,
    pub cursor: usize,
    // TODO(cptp): this feels a bit out of place here, maybe move it to `Config`
    highlight: DiffHighlighter,
}

impl render::Render for Status {
    fn render(&self, f: &mut Renderer) -> Result<(), fmt::Error> {
        use fmt::Write;
        let config = CONFIG.get().expect("config wasn't initialised");
        // Display the current branch
        writeln!(
            f,
            "\rOn branch {}{}{}",
            Attribute::Bold,
            self.branch,
            ResetAttributes,
        )?;

        // Display most recent commit
        if !self.head.is_empty() {
            let mut head = self.head.split_whitespace();
            writeln!(
                f,
                "{}\r\n{}{}{}",
                Attribute::Dim,
                head.next().unwrap(), // !self.head.is_empty()
                ResetAttributes,
                head.map(|w| format!(" {w}")).collect::<String>()
            )?;
        }

        if self.file_diffs.is_empty() {
            write!(
                f,
                "\r\n{}nothing to commit, working tree clean{}",
                style::SetForegroundColor(config.colors.heading),
                style::SetForegroundColor(config.colors.foreground)
            )?;
            drop(stdout().flush());
        }

        for (index, file) in self.file_diffs.iter().enumerate() {
            if index == 0 && self.count_untracked != 0 {
                writeln!(
                    f,
                    "\r\n{}Untracked files{} {}({}){}",
                    style::SetForegroundColor(config.colors.heading),
                    ResetColor,
                    style::Attribute::Dim,
                    self.count_untracked,
                    ResetAttributes
                )?;
            } else if index == self.count_untracked && self.count_unstaged != 0 {
                writeln!(
                    f,
                    "\r\n{}Unstaged changes{} {}({}){}",
                    style::SetForegroundColor(config.colors.heading),
                    ResetColor,
                    style::Attribute::Dim,
                    self.count_unstaged,
                    ResetAttributes
                )?;
            } else if index == self.count_untracked + self.count_unstaged {
                writeln!(
                    f,
                    "\r\n{}Staged changes{} {}({}){}",
                    style::SetForegroundColor(config.colors.heading),
                    ResetColor,
                    style::Attribute::Dim,
                    self.count_staged,
                    ResetAttributes
                )?;
            }

            if file.cursor == 0 && self.cursor == index {
                f.insert_cursor();
                write!(f, "{}", Attribute::Reverse)?;
            }
            write!(f, "\r    ")?;
            file.render(f)?;
            writeln!(f, "{ResetAttributes}")?;
        }

        Ok(())
    }
}

impl Status {
    pub fn new(repo: &Repository, config: &Config) -> Result<Self> {
        let highlight = match &config.options.syntax_highlighting {
            Some(theme_name) => DiffHighlighter::syntect(theme_name)?,
            None => DiffHighlighter::Simple {
                color_added: config.colors.addition,
                color_removed: config.colors.deletion,
            },
        };
        let mut status = Self {
            branch: String::default(),
            head: String::default(),
            file_diffs: Vec::default(),
            count_untracked: 0,
            count_unstaged: 0,
            count_staged: 0,
            cursor: 0,
            highlight,
        };
        status.fetch(repo, &config.options)?;
        Ok(status)
    }

    pub fn fetch(&mut self, repo: &Repository, options: &Options) -> Result<()> {
        // Leaving ourselves a lot of room to optimise and tidy up in here :D
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
                    let path = line.trim_start();
                    let previous_entry = self
                        .file_diffs
                        .iter()
                        .take(self.count_untracked)
                        .find(|f| f.path == path);
                    untracked.push(FileDiff::new(
                        path,
                        DiffType::Untracked,
                        previous_entry.map_or(options.auto_expand_files, |f| f.expanded),
                        previous_entry.map_or(0, |f| f.cursor),
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

                    let path = line.trim_start();
                    let previous_entry = self
                        .file_diffs
                        .iter()
                        .skip(self.count_untracked)
                        .take(self.count_unstaged)
                        .find(|f| f.path == path);
                    unstaged.push(FileDiff::new(
                        path,
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
                        previous_entry.map_or(options.auto_expand_files, |f| f.expanded),
                        previous_entry.map_or(0, |f| f.cursor),
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

                    let path = line.trim_start();
                    let previous_entry = self
                        .file_diffs
                        .iter()
                        .skip(self.count_untracked + self.count_unstaged)
                        .find(|f| f.path == path);
                    staged.push(FileDiff::new(
                        path,
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
                        previous_entry.map_or(options.auto_expand_files, |f| f.expanded),
                        previous_entry.map_or(0, |f| f.cursor),
                    ));
                }
            }
        }

        // Get the diff information for unstaged changes
        let diff = git_process(&["diff", "--no-ext-diff"])?;
        Self::populate_diffs(
            &mut unstaged,
            &self.file_diffs,
            &diff,
            options,
            &self.highlight,
        )
        .context("failed to populate unstaged file diffs")?;

        // Get the diff information for staged changes
        let diff = git_process(&["diff", "--cached", "--no-ext-diff"])?;
        Self::populate_diffs(
            &mut staged,
            &self.file_diffs,
            &diff,
            options,
            &self.highlight,
        )
        .context("failed to populate staged file diffs")?;

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

        for file_diff in self.file_diffs.iter_mut().filter(|f| f.cursor >= f.len()) {
            file_diff.cursor = file_diff.len() - 1;
        }

        if !self.file_diffs.is_empty() && self.cursor >= self.file_diffs.len() {
            self.cursor = self.file_diffs.len() - 1;
        }

        if let Some(file_diff) = self.file_diffs.get_mut(self.cursor) {
            file_diff.selected = true;
        }

        Ok(())
    }

    /// Takes a vec `file_diffs` containing `FileDiff` elements that have only the name populated,
    /// and populates their hunks based on the parsing of `diff`, and the `prev_file_diffs`.
    fn populate_diffs(
        file_diffs: &mut Vec<FileDiff>,
        prev_file_diffs: &[FileDiff],
        diff: &Output,
        options: &Options,
        highlight: &DiffHighlighter,
    ) -> Result<()> {
        let diff = std::str::from_utf8(&diff.stdout).context("malformed stdout from `git diff`")?;
        let hunks = parse::parse_diff(diff)?;
        for file in file_diffs {
            // Get the syntax info for the specific file type here, since the hunks below are
            // all from the same file.
            let syntax = highlight.get_syntax(&file.path);
            if let Some(hunks) = hunks.get(file.path.as_str()) {
                // Get all the diffs entries of this file from the previous iteration.
                let previous_file_entries = prev_file_diffs.iter().filter(|f| f.path == file.path);
                file.hunks = hunks
                    .iter()
                    .map(|hunk| {
                        let expanded = previous_file_entries
                            .clone()
                            .find_map(|f| {
                                f.hunks.iter().find(|h| {
                                    let h_header =
                                        h.diff.lines().next().expect("hunk should never be empty");
                                    let hunk_header =
                                        // TODO(cptp): this has to be hunk.header().0 once the parsing is correct
                                        hunk.header().1;
                                    (parse_hunk_new(h_header).unwrap_or_else(|e| panic!("{e:?}"))
                                        == parse_hunk_new(hunk_header)
                                            .unwrap_or_else(|e| panic!("{e:?}")))
                                        || (parse_hunk_old(h_header)
                                            .unwrap_or_else(|e| panic!("{e:?}"))
                                            == parse_hunk_old(hunk_header)
                                                .unwrap_or_else(|e| panic!("{e:?}")))
                                })
                            })
                            .map_or(options.auto_expand_hunks, |h| h.expanded);

                        let highlighted = highlight_hunk(hunk, highlight, syntax);
                        HunkView::new(hunk.raw().to_owned(), highlighted, expanded)
                    })
                    .collect();
            }
        }
        Ok(())
    }

    fn stage_or_unstage(&mut self, command: Stage) -> Result<()> {
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
                MiniBuffer::push(&stderr_buf, MessageType::Error);
            }
        }

        self.file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?
            .selected = true;
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
        if self.file_diffs.is_empty() {
            return Ok(());
        }

        let file = self
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

    /// Jump to previous file.
    pub fn file_up(&mut self) -> Result<()> {
        if self.file_diffs.is_empty() {
            return Ok(());
        }
        let file = self
            .file_diffs
            .get_mut(self.cursor)
            .context("cursor is at invalid position")?;
        if file.cursor == 0 {
            file.selected = false;
            self.cursor = self.cursor.saturating_sub(1);
            let new_file = self
                .file_diffs
                .get_mut(self.cursor)
                .expect("self.cursor >= 0, !self.file_diffs.is_empty");
            new_file.selected = true;
            new_file.cursor = 0;
        } else {
            file.cursor = 0;
        }
        Ok(())
    }

    /// Jump to next file.
    pub fn file_down(&mut self) -> Result<()> {
        if self.cursor < self.file_diffs.len() - 1 {
            self.file_diffs
                .get_mut(self.cursor)
                .context("cursor is at invalid position")?
                .selected = false;
            self.cursor += 1;
            let new_file = self
                .file_diffs
                .get_mut(self.cursor)
                .expect("self.cursor < self.file_diffs.len");
            new_file.selected = true;
            new_file.cursor = 0;
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
