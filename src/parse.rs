use std::{collections::HashMap, iter};

use anyhow::{Context, Result};
use itertools::Itertools;
use nom::{bytes::complete::tag, character::complete::not_line_ending, IResult};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, Style, Theme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

// Diff styles:
const MARKER_NONE: Style = Style {
    foreground: Color::WHITE,
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};

const MARKER_ADDED: Style = Style {
    foreground: Color {
        r: 0x78,
        g: 0xde,
        b: 0x0c,
        a: 0xff,
    },
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};
const MARKER_REMOVED: Style = Style {
    foreground: Color {
        r: 0xd3,
        g: 0x2e,
        b: 0x09,
        a: 0xff,
    },
    background: Color::BLACK,
    font_style: FontStyle::empty(),
};

#[derive(Debug)]
pub struct SyntaxHighlight {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlight {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            // TODO: theme configuration
            theme: ThemeSet::load_defaults().themes["base16-eighties.dark"].clone(),
        }
    }

    fn get_syntax(&self, path: &str) -> &SyntaxReference {
        // TODO: probably better to use std::path?
        let file_ext = path.rsplit('.').next().unwrap_or("");
        self.syntax_set
            .find_syntax_by_extension(file_ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }
}

// TODO: just a workaround so that Status::default() still works
impl Default for SyntaxHighlight {
    fn default() -> Self {
        Self::new()
    }
}

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
        let syntax = highlight.get_syntax(path);
        let mut highlighter = HighlightLines::new(syntax, &highlight.theme);
        let hunks = get_hunks(diff)?
            .into_iter()
            .map(|hunk| {
                let s: String = hunk
                    .lines()
                    .map(|line| {
                        let Some(diff_char) = line.chars().next() else {
                            return line.to_owned();
                        };

                        let diff_char_len = diff_char.len_utf8();

                        // add marker and one space
                        let (marker, diff_line) = match diff_char {
                            '+' => ((MARKER_ADDED, "+ "), &line[diff_char_len..]),
                            '-' => ((MARKER_REMOVED, "- "), &line[diff_char_len..]),
                            _ => ((MARKER_NONE, " "), line),
                        };

                        let Ok(ranges) =
                            highlighter.highlight_line(diff_line, &highlight.syntax_set)
                        else {
                            // Syntax highighting failed, fallback to no highlighting
                            // TODO: propagate error?
                            return diff_line.to_owned();
                        };

                        let ranges: Vec<_> = iter::once(marker).chain(ranges).collect();
                        as_24_bit_terminal_escaped(&ranges, false)
                    })
                    .join("\n");
                s
            })
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
