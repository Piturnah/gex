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
        diffs.insert(get_path(diff), get_changes(diff));
    }
    diffs
}

fn get_path<'a>(diff: &[&'a str]) -> &'a str {
    let diff: IResult<&str, &str> = tag("+++ b/")(diff[2]);
    let (diff, _) = match diff {
        Ok((diff, rest)) => (diff, rest),
        _ => return "",
    };
    let path: IResult<&str, &str> = not_line_ending(diff);
    let (_, path) = path.unwrap();
    path
}

fn get_changes<'a>(diff: &[&'a str]) -> Vec<Vec<&'a str>> {
    let mut changes = Vec::new();
    for line in diff {
        if line.starts_with("@@") {
            changes.push(vec![*line]);
        } else if let Some(last_diff) = changes.last_mut() {
            last_diff.push(*line)
        }
    }
    changes
}
