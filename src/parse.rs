use std::collections::HashMap;

use anyhow::{Context, Result};
use itertools::Itertools;
use nom::{bytes::complete::tag, character::complete::not_line_ending, IResult};

use crate::highlight::{highlight_hunk, SyntaxHighlight};

/// The returned hashmap associates a filename with a `Vec` of `String` where the strings contain
/// the content of each hunk.
pub fn parse_diff<'a>(
    input: &'a str,
    highlight: &SyntaxHighlight,
) -> Result<HashMap<&'a str, Vec<String>>> {
    // HACK: persist this somewhere else
    let mut diffs = HashMap::new();
    for diff in input
        .lines()
        .collect::<Vec<_>>()
        .split(|l| l.starts_with("diff"))
        .skip(1)
    {
        let path = get_path(diff)?;
        // get language syntax here, since all hunks are from the same file
        let syntax = highlight.get_syntax(path);
        let hunks = get_hunks(diff)?
            .iter()
            .map(|hunk| highlight_hunk(hunk, highlight, syntax))
            .collect();
        diffs.insert(path, hunks);
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
        let Some((key, mut lines)) = hunk_groups.next() else {
            break;
        };
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

/// Gets the `old` part of the hunk header
/// E.g. @@ -305,6 +305,7 @@ --> "305,6"
pub fn parse_hunk_old(header: &str) -> Result<&str> {
    let (_, header) = header
        .split_once('-')
        .with_context(|| format!("tried to parse strange hunk header: {header}"))?;
    let (old, _) = header
        .split_once(' ')
        .with_context(|| format!("tried to parse strange hunk header: {header}"))?;
    Ok(old)
}

/// Gets the `new` part of the hunk header
/// E.g. @@ -305,6 +305,7 @@ --> "305,7"
pub fn parse_hunk_new(header: &str) -> Result<&str> {
    let (_, header) = header
        .split_once('+')
        .with_context(|| format!("tried to parse strange hunk header: {header}"))?;
    let (old, _) = header
        .split_once(' ')
        .with_context(|| format!("tried to parse strange hunk header: {header}"))?;
    Ok(old)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::parse::SyntaxHighlight;

    const ISSUE_62: &str = "diff --git a/asteroid-loop/index.html b/asteroid-loop/index.html
index d79df71..e2d1e9f 100644
--- a/asteroid-loop/index.html
+++ b/asteroid-loop/index.html
@@ -14,15 +14,10 @@
     var unityInstance = UnityLoader.instantiate(\"unityContainer\", \"Build/thing build.json\", { onProgress: UnityProgress });
   </script>
   <script src=\"https://code.jquery.com/jquery-1.10.2.js\"></script>
-  <script>
-    $(function () {
-      $(\"#header\").load(\"/assets/header.html\");
-    });
-  </script>
 </head>
 
 <body>
-  <div id=\"header\"></div>
+  <#header />
   <div class=\"webgl-content\">
     <div id=\"unityContainer\" style=\"width: 960px; height: 540px\"></div>
     <div class=\"footer\">
@@ -32,4 +27,4 @@
   </div>
 </body>
 
-</html>
\\ No newline at end of file
+</html>";

    #[test_case(ISSUE_62 ; "issue 62")]
    fn parse(diff: &str) {
        let highlight = SyntaxHighlight::new();
        let parsed = super::parse_diff(diff, &highlight);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.len(), 1);
    }
}
