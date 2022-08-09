use crossterm::{cursor, style::Attribute};
use std::{fmt, process::Command};

pub struct BranchList {
    pub branches: Vec<String>,
    pub cursor: usize,
}

impl fmt::Display for BranchList {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for (i, branch) in self.branches.iter().enumerate() {
            if i == self.cursor {
                write!(
                    f,
                    "{}{}{}{}\n",
                    cursor::MoveToColumn(0),
                    Attribute::Reverse,
                    branch,
                    Attribute::Reset
                )?;
            } else {
                write!(f, "{}{}\n", cursor::MoveToColumn(0), branch)?;
            }
        }
        Ok(())
    }
}

impl BranchList {
    pub fn new() -> Self {
        let mut branch_list = Self {
            branches: Vec::new(),
            cursor: 0,
        };
        branch_list.fetch();
        branch_list
    }

    pub fn fetch(&mut self) {
        let branches = Command::new("git")
            .arg("branch")
            .output()
            .expect("failed to run `git branch`");

        self.branches = std::str::from_utf8(&branches.stdout)
            .expect("broken stdout from `git branch`")
            .lines()
            .map(|l| l.to_string())
            .collect::<Vec<_>>();
    }

    pub fn checkout(&self) {
        Command::new("git")
            .args(["checkout", &self.branches[self.cursor][2..]])
            .output()
            .expect("failed to run `git checkout`");
    }
}
