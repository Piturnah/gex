<p align="center">
  <img src="https://github.com/user-attachments/assets/e1cf91bd-2a6e-4d10-9e9e-f0b1c78a8484" alt="Gex">
</p>

# Gex

[![crates.io](https://img.shields.io/crates/v/gex)](https://crates.io/crates/gex)
[![download](https://img.shields.io/crates/d/gex)](https://crates.io/crates/gex)
[![license](https://img.shields.io/crates/l/gex)](https://crates.io/crates/gex)
[![stargazers](https://img.shields.io/github/stars/Piturnah/gex?style=social)](https://github.com/Piturnah/gex/stargazers)

**NOTE: GEX IS UNFINISHED SOFTWARE.** As a result, many features are missing, and the interface can change at any moment.

<p align="center">
  <img src="https://user-images.githubusercontent.com/20472367/185642346-7f4b3738-0b75-42c1-9983-6ef7b3b72bde.gif" alt="Gex">
</p>

Git workflow improvement CLI tool inspired by [Magit](https://github.com/magit/magit). **This project is still under initial development**, but I am actively [dogfooding](https://en.wikipedia.org/wiki/Eating_your_own_dog_food) it and features *should* be added relatively quickly.

## Aims

Primarily, this is a personal project since I recently switched to Neovim from Emacs and miss the simplicity and efficiency of using Magit. However, I do have some general aims, which are subject to change:

- [x] Simple - uncluttered UI.
- [x] Intuitive - it should be easy to learn to use gex.
- [x] Cross platform - primary focus on Linux, but should work well on Windows and MacOS.
- [x] [Configurable](./#Configuration) - certain preferences in gex should be configurable to suit your own workflow.
- [ ] Comprehensive\* - you should be able to use gex to do everything you can do in git.

\* gex supports executing arbitrary git commands with <kbd>:</kbd> for when something is not yet available

## Non-Aims

- Magit port

While it serves as a major inspiration, I am not trying to 1:1 port the behaviour and functionality of Magit.

## Installation

### Crates.io

[![crates.io](https://img.shields.io/crates/v/gex)](https://crates.io/crates/gex)

> **NOTE:** You will need [Rust](https://www.rust-lang.org/) on your system for this installation method.

```console
$ cargo install gex
```

### Other

Gex packages are also maintained by the community in a handful of repositories.

[![Packaging status](https://repology.org/badge/vertical-allrepos/gex.svg)](https://repology.org/project/gex/versions)

## Usage

To enter gex simply type `gex` in console, optionally providing a path.

```console
$ gex
```

Full usage:

```console
$ gex --help

Git workflow improvement CLI tool inspired by Magit

Usage: gex [OPTIONS] [PATH]

Arguments:
  [PATH]  The path to the repository [default: .]

Options:
  -c, --config-file <PATH>  Path to a config file to use
  -h, --help                Print help
  -V, --version             Print version
```

### Navigation

| Key                               | Action                |
| --------------------------------- | ------------          |
| <kbd>j</kbd> / <kbd>Down</kbd>    | Move down             |
| <kbd>k</kbd> / <kbd>Up</kbd>      | Move up               |
| <kbd>J</kbd>                      | Jump to next file     |
| <kbd>K</kbd>                      | Jump to previous file |
| <kbd>Tab</kbd> / <kbd>Space</kbd> | Toggle expand         |
| <kbd>g</kbd>                      | Go to top             |
| <kbd>G</kbd>                      | Go to bottom          |

### Gex actions

| Key            | Action              |
| ------------   | ------------------- |
| <kbd>s</kbd>   | stage item          |
| <kbd>S</kbd>   | stage all items     |
| <kbd>u</kbd>   | unstage item        |
| <kbd>U</kbd>   | unstage all items   |
| <kbd>e</kbd>   | edit file/hunk      |
| <kbd>F</kbd>   | pull from remote    |
| <kbd>:</kbd>   | execute git command |
| <kbd>!</kbd>   | execute subprocess  |
| <kbd>r</kbd>   | refresh             |
| <kbd>Esc</kbd> | cancel current      |
| <kbd>q</kbd>   | quit gex            |

### Gex commands

| Key          | Action            |
| ------------ | ----------------- |
| <kbd>c</kbd> | commit            |
| <kbd>b</kbd> | branch            |
| <kbd>p</kbd> | push              |
| <kbd>z</kbd> | stash             |

## Configuration

Gex will look for a config file in the following places:

| OS      | Path                                                |
| ------- | --------------------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME/gex/config.toml`                  |
| MacOS   | `$HOME/Library/Application Support/gex/config.toml` |
| Windows | `{FOLDERID_RoamingAppData}/gex/config.toml`         |

On all platforms, gex will try `$HOME/.config/gex/config.toml` as a fallback.

Here is an example `config.toml`:

```toml
[options]
auto_expand_files = false
auto_expand_hunks = true
editor = "nvim" # defaults to git's core.editor or $EDITOR or "vi"
lookahead_lines = 5
sort_branches = "-committerdate" # key to pass to `git branch --sort`. https://git-scm.com/docs/git-for-each-ref#_field_names
truncate_lines = true # `false` is not recommended - see #37
ws_error_highlight = "new" # override git's diff.wsErrorHighlight

# Named colours use the terminal colour scheme. You can also describe your colours
# by hex string "#RRGGBB", RGB "rgb_(r,g,b)" or by Ansi "ansi_(value)".
#
# This example uses a Gruvbox colour theme.
[colors]
foreground = "#ebdbb2"
background = "#282828"
heading = "#fabd2f"
hunk_head = "#d3869b"
addition = "#b8bb26"
deletion = "#fb4934"
key = "#d79921"
error = "#cc241d"

[keymap.navigation]
move_down     = ['j', "Down"]
move_up       = ['k', "Up"]
next_file     = ['J']
previous_file = ['K']
toggle_expand = [" ", "Tab"]
goto_top      = ['g']
goto_bottom   = ['G']
```

## Versioning

A `0.X` version increase indicates some change that could reasonably break someone's workflow. This is quite hard to define, so apologies if it does not meet your expectations. Usually this means changing a default setting or redesigning parts of the UI.

A `0.x.Y` version increase indicates a change that should not break any workflow - i.e. fixing a bugs or adding features.

Whichever number is increased does not deliberately correlate with the *size* of the update.

`1.0.0` will come when I consider the software to be "finished", subject to small improvements/features or bug fixes. What this means is very subjective, and my own thoughts on this are likely to evolve as the project progresses.

## License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
