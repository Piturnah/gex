# gex
Git workflow improvement CLI tool inspired by [Magit](https://github.com/magit/magit). This project is still under initial development, but I am actively [dogfooding](https://en.wikipedia.org/wiki/Eating_your_own_dog_food) it and features *should* be added relatively quickly.

## Aims
Primarily, this is a personal project since I recently switched to Neovim from Emacs and miss the simplicity and efficiency of using Magit. However, I do have some general aims, which are subject to change:

- [x] Simple - uncluttered UI.
- [x] Intuitive - it should be easy to learn to use gex.
- [ ] Configurable - certain preferences in gex should be configurable to suit your own workflow.
- [ ] Comprehensive - you should be able to use gex to do everything you can do in git.

## Installation
In the future I may publish this crate as a binary to [Crates.io](https://crates.io/). For now you can either install from source, or you can use cargo:

```console
$ cargo install --git https://github.com/Piturnah/gex
```

## Usage

To enter `gex`, simply type `gex` in console.

```console
$ gex
```

### Navigation

| Key                            | Action      |
| ------------------------------ | ---------   |
| <kbd>j</kbd> / <kbd>Down</kbd> | Move down   |
| <kbd>k</kbd> / <kbd>Up</kbd>   | Move up     |
| <kbd>Tab</kbd>                 | Expand item |

### Git actions

| Key          | Action            |
| ------------ | ----------------- |
| <kbd>s</kbd> | stage item        |
| <kbd>S</kbd> | stage all items   |
| <kbd>u</kbd> | unstage item      |
| <kbd>U</kbd> | unstage all items |
| <kbd>c</kbd> | commit\*          |

\* uses default editor configured with git

### Gex actions

| Key                           | Action   |
| ----------------------------- | -------- |
| <kbd>r</kbd>                  | refresh  |
| <kbd>q</kbd> / <kbd>Esc</kbd> | quit gex |

## Contributing

If you want to contribute to gex, thank you so much! If you find a bug or want a new feature, please open an [issue](https://github.com/Piturnah/gex/issues) or submit a PR! I am happy to review and merge PRs.
