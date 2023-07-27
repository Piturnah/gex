//! Gex configuration.
#![allow(clippy::derivable_impls)]
use std::{fs, path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::style::Color;
use once_cell::sync::OnceCell;
use serde::Deserialize;

pub static CONFIG: OnceCell<Config> = OnceCell::new();

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
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub options: Options,
    pub colors: Colors,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Options {
    pub auto_expand_files: bool,
    pub auto_expand_hunks: bool,
    pub lookahead_lines: usize,
    pub truncate_lines: bool,
    pub ws_error_highlight: WsErrorHighlight,
}

#[derive(Deserialize, Clone, Copy, Debug)]
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
            ws_error_highlight: WsErrorHighlight::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Colors {
    pub heading: Color,
    pub hunk_head: Color,
    pub addition: Color,
    pub deletion: Color,
    pub key: Color,
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            heading: Color::Yellow,
            hunk_head: Color::Blue,
            addition: Color::DarkGreen,
            deletion: Color::DarkRed,
            key: Color::Green,
            error: Color::Red,
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
        let Ok(Ok(git_config)) = git2::Config::open_default().map(|mut config| config.snapshot())
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
