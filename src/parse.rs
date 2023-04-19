use std::collections::HashMap;

use anyhow::{Context, Result};
use nom::{bytes::complete::tag, character::complete::not_line_ending, IResult};

pub fn parse_diff(input: &str) -> Result<HashMap<&str, Vec<Vec<&str>>>> {
    let mut diffs = HashMap::new();
    for diff in input
        .lines()
        .collect::<Vec<_>>()
        .split(|l| l.starts_with("diff"))
        .skip(1)
    {
        diffs.insert(get_path(diff)?, get_hunks(diff));
    }
    Ok(diffs)
}

fn get_path<'a>(diff: &[&'a str]) -> Result<&'a str> {
    let diff: IResult<&str, &str> = tag("+++ b/")(diff.get(2).unwrap_or(&""));
    let Ok((diff, _)) = diff else { return Ok("") };
    let path: IResult<&str, &str> = not_line_ending(diff);
    let (_, path) = path
        .map_err(|e| e.to_owned())
        .context("failed to parse a path from diff")?;
    Ok(path)
}

fn get_hunks<'a>(diff: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut hunks = Vec::new();
    for line in diff {
        if line.starts_with("@@") {
            hunks.push(vec![*line]);
        } else if let Some(last_diff) = hunks.last_mut() {
            last_diff.push(*line);
        }
    }
    hunks
}
