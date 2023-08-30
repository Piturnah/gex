use std::{
    fmt,
    io::{stdin, stdout, BufRead, Write},
    process::Output,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    style::{Attribute, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::{
    config::CONFIG,
    git_process,
    render::{self, Clear, Renderer, ResetAttributes},
};

pub struct BranchList {
    pub branches: Vec<String>,
    pub cursor: usize,
}

impl render::Render for BranchList {
    fn render(&self, f: &mut Renderer) -> fmt::Result {
        use fmt::Write;
        let config = CONFIG.get().expect("config wasn't initialised");

        if self.branches.is_empty() {
            return write!(
                f,
                "{}No branches yet.{}\r\n\nMake a commit or press b again to switch branch.",
                SetForegroundColor(config.colors.heading),
                SetForegroundColor(config.colors.foreground),
            );
        }

        for (i, branch) in self.branches.iter().enumerate() {
            if branch.starts_with('*') {
                write!(f, "{}", SetForegroundColor(config.colors.heading))?;
            }
            if i == self.cursor {
                let mut branch = branch.to_string();
                branch.insert_str(2, &format!("{}", Attribute::Reverse));
                write!(&mut branch, "{ResetAttributes}")?;
                f.insert_cursor();
                writeln!(f, "\r{branch}")?;
            } else {
                writeln!(f, "\r{branch}")?;
            }
            if branch.starts_with('*') {
                write!(f, "{}", SetForegroundColor(config.colors.foreground))?;
            }
        }
        Ok(())
    }
}

impl BranchList {
    pub fn new() -> Result<Self> {
        let mut branch_list = Self {
            branches: Vec::new(),
            cursor: 0,
        };
        branch_list.fetch()?;
        Ok(branch_list)
    }

    pub fn fetch(&mut self) -> Result<()> {
        let output = git_process(&["branch"])?;

        self.branches = std::str::from_utf8(&output.stdout)
            .context("broken stdout from `git branch`")?
            .lines()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();

        Ok(())
    }

    pub fn checkout(&self) -> Result<Output> {
        git_process(&["checkout", &self.branches[self.cursor][2..]])
    }

    pub fn checkout_new() -> Result<Output> {
        terminal::disable_raw_mode().context("failed to exit raw mode")?;
        print!(
            "{}{}{}Name for the new branch: ",
            cursor::MoveTo(0, 0),
            Clear(ClearType::All),
            cursor::Show
        );
        drop(stdout().flush());

        let input = stdin()
            .lock()
            .lines()
            .next()
            .context("no stdin")?
            .context("malformed stdin")?;

        terminal::enable_raw_mode().context("failed to enter raw mode")?;
        print!("{}", cursor::Hide);

        git_process(&["checkout", "-b", &input])
    }
}
