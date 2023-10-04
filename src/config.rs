//! Gex configuration.
#![allow(clippy::derivable_impls)]
use std::{fs, path::PathBuf, str::FromStr, sync::OnceLock};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::style::Color;
use serde::Deserialize;

pub static CONFIG: OnceLock<Config> = OnceLock::new();
#[macro_export]
macro_rules! config {
    () => {
        $crate::config::CONFIG
            .get()
            .expect("config wasn't initialised")
    };
}

/// Command line args.
#[derive(Parser)]
#[command(version = env!("GEX_VERSION"), about)]
pub struct Clargs {
    /// The path to the repository.
    #[clap(default_value = ".")]
    pub path: String,

    /// Path to a config file to use.
    #[clap(short, long, name = "PATH")]
    pub config_file: Option<String>,
}

/// The top-level of the config parsed from the config file.
#[derive(Deserialize, Default, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub options: Options,
    pub colors: Colors,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Options {
    pub auto_expand_files: bool,
    pub auto_expand_hunks: bool,
    pub lookahead_lines: usize,
    pub truncate_lines: bool,
    pub sort_branches: Option<String>,
    pub ws_error_highlight: WsErrorHighlight,
}

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(try_from = "String")]
pub struct WsErrorHighlight {
    pub old: bool,
    pub new: bool,
    pub context: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_expand_files: false,
            auto_expand_hunks: true,
            lookahead_lines: 5,
            truncate_lines: true,
            sort_branches: None,
            ws_error_highlight: WsErrorHighlight::default(),
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct Colors {
    pub foreground: Color,
    pub background: Color,
    pub heading: Color,
    pub hunk_head: Color,
    pub addition: Color,
    pub deletion: Color,
    pub key: Color,
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        // We have to force colour output here regardless of NO_COLOR setting, because then we can
        // handle it ourselves. The NO_COLOR standard specifies that colour output should be
        // enabled when the user has explicitly set it, which can be achieved here by detecting the
        // env variable and then enabling color granularly based on the user config.
        crossterm::style::force_color_output(true);
        if std::env::var("NO_COLOR").map_or(false, |v| !v.is_empty()) {
            Self {
                foreground: Color::Reset,
                background: Color::Reset,
                heading: Color::Reset,
                hunk_head: Color::Reset,
                addition: Color::Reset,
                deletion: Color::Reset,
                key: Color::Reset,
                error: Color::Reset,
            }
        } else {
            Self {
                foreground: Color::Reset,
                background: Color::Reset,
                heading: Color::Yellow,
                hunk_head: Color::Blue,
                addition: Color::DarkGreen,
                deletion: Color::DarkRed,
                key: Color::Green,
                error: Color::Red,
            }
        }
    }
}

impl Config {
    /// Reads the config from the config file (usually `~/.config/gex/config.toml` on Linux) and
    /// returns it along with a Vec of unrecognised keys.
    /// If there is no config file, it will return `Ok(None)`.
    /// If there is a config file but it is unable to parse it, it will return `Err(_)`.
    pub fn read_from_file(path: &Option<String>) -> Result<Option<(Self, Vec<String>)>> {
        let mut config_path;
        if let Some(path) = path {
            config_path = PathBuf::from(path);
        } else if let Some(path) = dirs::config_dir() {
            config_path = path;
            config_path.push("gex");
            config_path.push("config.toml");
        } else {
            return Ok(None);
        }

        let Ok(config) = fs::read_to_string(config_path) else {
            return Ok(None);
        };

        let de = toml::Deserializer::new(&config);
        let mut unused_keys = Vec::new();
        let config = serde_ignored::deserialize(de, |path| {
            unused_keys.push(path.to_string());
        })
        .context("failed to parse config file")?;
        Ok(Some((config, unused_keys)))
    }
}

impl WsErrorHighlight {
    /// The default value defined by git.
    const GIT_DEFAULT: Self = Self {
        old: false,
        new: true,
        context: false,
    };
    const NONE: Self = Self {
        old: false,
        new: false,
        context: false,
    };
    const ALL: Self = Self {
        old: true,
        new: true,
        context: true,
    };
}

impl Default for WsErrorHighlight {
    /// If none was provided by the gex config, we will look in the git config. If we couldn't get
    /// that one then we'll just provide `Self::GIT_DEFAULT`.
    fn default() -> Self {
        let Ok(git_config) = git2::Config::open_default().and_then(|mut config| config.snapshot())
        else {
            return Self::GIT_DEFAULT;
        };

        let Ok(value) = git_config.get_str("diff.wsErrorHighlight") else {
            return Self::GIT_DEFAULT;
        };

        Self::from_str(value).unwrap_or(Self::GIT_DEFAULT)
    }
}

// NOTE: If anyone is reading this, do you happen to know why this impl is even needed? Really
// feels like this should be provided by default is `FromStr` is implemented on the type.
impl TryFrom<String> for WsErrorHighlight {
    type Error = anyhow::Error;
    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl FromStr for WsErrorHighlight {
    type Err = anyhow::Error;
    /// Highlight whitespace errors in the context, old or new lines of the diff. Multiple values
    /// are separated by by comma, none resets previous values, default reset the list to new and
    /// all is a shorthand for old,new,context.
    ///
    /// <https://git-scm.com/docs/git-diff#Documentation/git-diff.txt---ws-error-highlightltkindgt>
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut result = Self::GIT_DEFAULT;
        for opt in s.split(',') {
            match opt {
                "all" => result = Self::ALL,
                "default" => result = Self::GIT_DEFAULT,
                "none" => result = Self::NONE,
                "old" => result.old = true,
                "new" => result.new = true,
                "context" => result.context = true,
                otherwise => {
                    return Err(anyhow::Error::msg(format!(
                        "unrecognised option in `ws_error_highlight`: {otherwise}"
                    )))
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;

    // Should be up to date with the example config in the README.
    #[test]
    fn parse_readme_example() {
        const INPUT: &str = "
[options]
auto_expand_files = false
auto_expand_hunks = true
lookahead_lines = 5
truncate_lines = true # `false` is not recommended - see #37
sort_branches = \"-committerdate\" # filter to pass to `git branch --sort`. https://git-scm.com/docs/git-for-each-ref#_field_names 
ws_error_highlight = \"new\" # override git's diff.wsErrorHighlight

# Named colours use the terminal colour scheme. You can also describe your colours
# by hex string \"#RRGGBB\", RGB \"rgb_(r,g,b)\" or by Ansi \"ansi_(value)\".
#
# This example uses a Gruvbox colour theme.
[colors]
foreground = \"#ebdbb2\"
background = \"#282828\"
heading = \"#fabd2f\"
hunk_head = \"#d3869b\"
addition = \"#b8bb26\"
deletion = \"#fb4934\"
key = \"#d79921\"
error = \"#cc241d\"
";
        assert_eq!(
            toml::from_str(INPUT),
            Ok(Config {
                options: Options {
                    auto_expand_files: false,
                    auto_expand_hunks: true,
                    lookahead_lines: 5,
                    truncate_lines: true,
                    sort_branches: Some("-committerdate".to_string()),
                    ws_error_highlight: WsErrorHighlight {
                        old: false,
                        new: true,
                        context: false
                    }
                },
                colors: Colors {
                    foreground: Color::from((235, 219, 178)),
                    background: Color::from((40, 40, 40)),
                    heading: Color::from((250, 189, 47)),
                    hunk_head: Color::from((211, 134, 155)),
                    addition: Color::from((184, 187, 38)),
                    deletion: Color::from((251, 73, 52)),
                    key: Color::from((215, 153, 33)),
                    error: Color::from((204, 36, 29))
                }
            })
        )
    }
}
