# Gex

[![crates.io](https://img.shields.io/crates/v/gex)](https://crates.io/crates/gex)
[![download](https://img.shields.io/crates/d/gex)](https://crates.io/crates/gex)
[![license](https://img.shields.io/crates/l/gex)](https://crates.io/crates/gex)
[![stargazers](https://img.shields.io/github/stars/Piturnah/gex?style=social)](https://github.com/Piturnah/gex/stargazers)

<p align="center">
  <img src="https://user-images.githubusercontent.com/20472367/185642346-7f4b3738-0b75-42c1-9983-6ef7b3b72bde.gif" alt="Gex">
</p>

Git workflow improvement CLI tool inspired by [Magit](https://github.com/magit/magit). **This project is still under initial development**, but I am actively [dogfooding](https://en.wikipedia.org/wiki/Eating_your_own_dog_food) it and features *should* be added relatively quickly.

## Aims

Primarily, this is a personal project since I recently switched to Neovim from Emacs and miss the simplicity and efficiency of using Magit. However, I do have some general aims, which are subject to change:

- [x] Simple - uncluttered UI.
- [x] Intuitive - it should be easy to learn to use gex.
- [x] Cross platform - primary focus on Linux, but should work well on Windows and MacOS.
- [ ] Configurable - certain preferences in gex should be configurable to suit your own workflow.
- [ ] Comprehensive\* - you should be able to use gex to do everything you can do in git.

\* gex supports executing arbitrary git commands with <kbd>:</kbd> for when something is not yet available

## Installation

### Crates.io

[![crates.io](https://img.shields.io/crates/v/gex)](https://crates.io/crates/gex)

> **NOTE:** You will need [Rust](https://www.rust-lang.org/) on your system for this installation method.

```console
$ cargo install gex
```

### Nix

> Maintained by [@Br1ght0ne](https://github.com/Br1ght0ne)

[![nixpkgs](https://repology.org/badge/version-for-repo/nix_unstable/gex.svg)](https://repology.org/project/gex/versions)

```console
$ nix-env -iA nixpkgs.gex
```

## Usage

To enter gex, simply type `gex` in console.

```console
$ gex
```

### Navigation

| Key                            | Action       |
| ------------------------------ | ---------    |
| <kbd>j</kbd> / <kbd>Down</kbd> | Move down    |
| <kbd>k</kbd> / <kbd>Up</kbd>   | Move up      |
| <kbd>Tab</kbd>                 | Expand item  |
| <kbd>g</kbd> / <kbd>K</kbd>    | Go to top    |
| <kbd>G</kbd> / <kbd>J</kbd>    | Go to bottom |

### Git actions

| Key          | Action              |
| ------------ | ------------------- |
| <kbd>s</kbd> | stage item          |
| <kbd>S</kbd> | stage all items     |
| <kbd>u</kbd> | unstage item        |
| <kbd>U</kbd> | unstage all items   |
| <kbd>F</kbd> | pull from remote    |
| <kbd>:</kbd> | execute git command |

### Gex actions

| Key          | Action            |
| ------------ | ----------------- |
| <kbd>c</kbd> | commit menu       |
| <kbd>b</kbd> | enter branch mode |
| <kbd>r</kbd> | refresh           |
| <kbd>q</kbd> | quit gex          |

### Branch mode

| Key                                 | Action              |
| ----------------------------------- | ------------------- |
| <kbd>b</kbd>                        | checkout new branch |
| <kbd>Space</kbd> / <kbd>Enter</kbd> | checkout branch     |
| <kbd>Esc</kbd>                      | exit branch mode    |

## License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option.

## Contributing

If you want to contribute to gex, thank you so much! If you find a bug or want a new feature, please open an [issue](https://github.com/Piturnah/gex/issues) or submit a PR! I am happy to review and merge PRs.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
