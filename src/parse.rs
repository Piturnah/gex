use std::collections::HashMap;

use anyhow::{Context, Result};
use itertools::Itertools;
use nom::{bytes::complete::tag, character::complete::not_line_ending, IResult};

/// The returned hashmap associates a filename with a `Vec` of `String` where the strings contain
/// the content of each hunk.
pub fn parse_diff(input: &str) -> Result<HashMap<&str, Vec<String>>> {
    let mut diffs = HashMap::new();
    for diff in input
        .lines()
        .collect::<Vec<_>>()
        .split(|l| l.starts_with("diff"))
        .skip(1)
    {
        diffs.insert(get_path(diff)?, get_hunks(diff)?);
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

fn get_hunks(diff: &[&str]) -> Result<Vec<String>> {
    let mut hunks = Vec::new();
    let hunk_groups = diff.iter().group_by(|line| line.starts_with("@@"));
    let mut hunk_groups = hunk_groups.into_iter();
    loop {
        let Some((key, mut lines)) = hunk_groups.next() else { break };
        let hunk_head = *lines.next().context("expected another line in diff")?;
        if !key {
            continue;
        }
        // XXX: `lines` should never have more than one item but we're not actually checking that.

        let (_key, hunk_tail) = hunk_groups
            .next()
            .context("strange output from `git diff`")?;
        hunks.push(
            std::iter::once(hunk_head)
                .chain(hunk_tail.copied())
                .join("\n"),
        );
    }
    Ok(hunks)
}
