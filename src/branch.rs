use std::{
    fmt,
    io::{stdin, stdout, BufRead, Write},
    process::Output,
};

use anyhow::{Context, Result};
use crossterm::{
    cursor,
    style::{Attribute, Color, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::git_process;

pub struct BranchList {
    pub branches: Vec<String>,
    pub cursor: usize,
}

impl fmt::Display for BranchList {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use fmt::Write;

        if self.branches.is_empty() {
            return write!(
                f,
                "{}No branches yet.{}\r\n\nMake a commit or press b again to switch branch.",
                SetForegroundColor(Color::Yellow),
                SetForegroundColor(Color::Reset),
            );
        }

        for (i, branch) in self.branches.iter().enumerate() {
            if branch.starts_with('*') {
                write!(f, "{}", SetForegroundColor(Color::Yellow))?;
            }
            if i == self.cursor {
                let mut branch = branch.to_string();
                branch.insert_str(2, &format!("{}", Attribute::Reverse));
                write!(&mut branch, "{}", Attribute::Reset)?;
                writeln!(f, "\r{branch}")?;
            } else {
                writeln!(f, "\r{branch}")?;
            }
            if branch.starts_with('*') {
                write!(f, "{}", SetForegroundColor(Color::Reset))?;
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
            terminal::Clear(ClearType::All),
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
