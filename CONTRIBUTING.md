# Contributing

First of all, thanks for your interest in contributing to Gex!

All types of contributions are greatly appreciated, some of which are outlined below.

## Table of contents

- [Testing new features](#testing-new-features)
- [Reporting bugs](#reporting-bugs)
- [Suggesting features](#suggesting-features)
- [Submitting code](#submitting-code)

## Testing new features

The easiest way for you to help with the development of Gex is by testing features that haven't been released yet. To do this, simply install Gex from source, for example:

```console
$ cargo install --git https://github.com/Piturnah/gex
```

Then you should see something similar to the following:

```console
$ gex --version
gex 0.3.5-dev (efd4bdb1ee986f80e349f58fceddfa239e077d0e)
```

The `main` branch of Gex should always be working, and contains features to be released in the next version. So if something goes wrong, please report it! See more at [Reporting bugs](#reporting-bugs).

## Reporting bugs

Another straightforward way to help out is by reporting anything that might go wrong during using Gex. This can be done via the [issue tracker](https://github.com/Piturnah/gex/issues) where you'll find a template for bug reports. If you're using a development version of gex, it would be a great help to try and see if your issue is also reproducable on the latest release!

## Suggesting features

If there is a feature missing from Gex that you feel should be present, there are a couple of things you can do.

- Submit a Pull Request
- Open an issue

Submitting a PR with your new feature implemented makes it much more likely to be introduced quickly. However, if it is a big change then you run the risk of your PR being closed if we disagree. Therefore, for bigger changes I recommend opening an issue first where we can discuss the feature (unless you don't mind running the risk of wasting your time!).

Opening an [issue](https://github.com/Piturnah/gex/issues) is a great way to start a discussion about a feature you want to be supported by Gex.

## Submitting code

A welcome way to help is by submitting a [Pull Request](https://github.com/Piturnah/gex/pulls) into the repository. I am happy to review PRs and will do my best to work with you to get it merged. Simple PRs are also appreciated, such as things like fixiing tpyos. I encourage you to submit a PR if possible as it greatly speeds up the process of making the change you want to see.

If you want to contribute code but don't already have a feature or bug fix in mind, check out the [issue tracker](https://github.com/Piturnah/gex/issues)! Optionally leave a comment saying you'd like to give a particular issue a go at resolving.

To avoid style and linter nitpicks, before submitting your PR please make sure you have run the following commands:

```console
$ cargo fmt
$ cargo clippy
```

If clippy fails on a false positive, or a true positive but you think the lint is unreasonable anyway in this case, then feel free to add the lint to the allow list either locally or for the whole crate. I may ask you to justify this when I review your PR if it's unclear.
