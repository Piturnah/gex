use std::{fmt, process::Output};

use anyhow::{Context, Result};
use crossterm::style::{Attribute, SetForegroundColor};

use crate::{
    config::CONFIG,
    git_process,
    minibuffer::{MessageType, MiniBuffer},
    render::{self, Renderer, ResetAttributes},
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
        let config = CONFIG.get().expect("config wasn't initialised");

        let output = match config.options.sort_branches.as_ref() {
            Some(sort_value) => {
                let output = git_process(&["branch", &format!("--sort={sort_value}")])?;
                if output.status.success() {
                    output
                } else {
                    MiniBuffer::push(
                        &format!(
                            "`git branch --sort={sort_value}` failed!\n\n{}",
                            String::from_utf8_lossy(&output.stderr)
                        ),
                        MessageType::Error,
                    );
                    git_process(&["branch"])?
                }
            }
            None => git_process(&["branch"])?,
        };

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

    pub fn checkout_new(name: &str) -> Result<()> {
        MiniBuffer::push_command_output(&git_process(&["checkout", "-b", name])?);
        Ok(())
    }
}
