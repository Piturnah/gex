//! Gex configuration.
#![allow(clippy::derivable_impls)]
use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
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
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Options {
    pub auto_expand_files: bool,
    pub auto_expand_hunks: bool,
    pub lookahead_lines: usize,
    pub truncate_lines: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_expand_files: false,
            auto_expand_hunks: true,
            lookahead_lines: 5,
            truncate_lines: true,
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
