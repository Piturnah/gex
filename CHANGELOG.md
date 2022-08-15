# Changelog

## [Unreleased]
### Added
- Propagation of errors and stdout from git subprocesses
- <kbd>F</kbd> to pull remote changes

## [0.2.1](https://github.com/Piturnah/gex/compare/v0.2.0...v0.2.1) - 2022-08-12
### Fixed
- gex crashing on untracked files in some cases
- gex not displaying "working tree clean" message

## [0.2.0](https://github.com/Piturnah/gex/compare/v0.1.0...v0.2.0) - 2022-08-12
### Added
- Press <kbd>b</kbd> to switch to a new `Branch` mode where you can switch between local branches with <kbd>Space</kbd>
- Checkout new branch within gex (<kbd>b</kbd> in `Branch` mode)
- Exit `Branch` mode with <kbd>Esc</kbd>
- Init git repository by running gex in a folder that is not a git repository
- Indication that working tree is clean
- [DELETE] or [RENAME] indicators in status view
### Changed
- <kbd>Esc</kbd> can no longer be used to exit gex
- Current branch name is now highlighted in bold
### Fixed
- gex crashing on attempts to perform actions when working tree clean
- gex crashing on encountering deleted files

## [0.1.0](https://github.com/Piturnah/gex/commits/v0.1.0) - 2022-08-05
### Added
- `git status` display with diff information and current branch
- Keyboard navigation between diffs of files and hunks
- Diff items can be expanded or collapsed with <kbd>Tab</kbd>
- Diff items can be (un)staged one at a time or all at once
- Status can be refreshed at any time with <kbd>r</kbd>
- Commits can be made from within gex, using git's `core.editor`
- Quit gex using <kbd>q</kbd> / <kbd>Esc</kbd>
