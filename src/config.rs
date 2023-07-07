//! Gex configuration.
#![allow(clippy::derivable_impls)]
use std::{fs, path::PathBuf};

use clap::Parser;
use serde::Deserialize;

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
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_expand_files: false,
            auto_expand_hunks: false,
        }
    }
}

impl Config {
    /// Reads the config from the config file (usually `~/.config/gex/config.toml` on Linux) and
    /// returns it along with a Vec of unrecognised keys.
    pub fn read_from_file(path: &Option<String>) -> Option<(Self, Vec<String>)> {
        let mut config_path;
        if let Some(path) = path {
            config_path = PathBuf::from(path);
        } else {
            config_path = dirs::config_dir()?;
            config_path.push("gex");
            config_path.push("config.toml");
        }

        fs::read_to_string(config_path)
            .map(|conf| {
                let de = toml::Deserializer::new(&conf);
                let mut unused_keys = Vec::new();
                let config = serde_ignored::deserialize(de, |path| {
                    unused_keys.push(path.to_string());
                })
                .unwrap();
                (config, unused_keys)
            })
            .ok()
    }
}
