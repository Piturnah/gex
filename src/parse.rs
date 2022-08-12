use nom::{bytes::complete::tag, character::complete::not_line_ending, IResult};
use std::collections::HashMap;

pub fn parse_diff(input: &str) -> HashMap<&str, Vec<Vec<&str>>> {
    let mut diffs = HashMap::new();
    for diff in input
        .lines()
        .collect::<Vec<_>>()
        .split(|l| l.starts_with("diff"))
        .skip(1)
    {
        diffs.insert(get_path(diff), get_hunks(diff));
    }
    diffs
}

fn get_path<'a>(diff: &[&'a str]) -> &'a str {
    let diff: IResult<&str, &str> = tag("+++ b/")(diff.get(2).unwrap_or_else(|| return &""));
    let (diff, _) = match diff {
        Ok((diff, rest)) => (diff, rest),
        _ => return "",
    };
    let path: IResult<&str, &str> = not_line_ending(diff);
    let (_, path) = path.unwrap();
    path
}

fn get_hunks<'a>(diff: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut hunks = Vec::new();
    for line in diff {
        if line.starts_with("@@") {
            hunks.push(vec![*line]);
        } else if let Some(last_diff) = hunks.last_mut() {
            last_diff.push(*line)
        }
    }
    hunks
}
