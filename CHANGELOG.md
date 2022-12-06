# Changelog

## [Unreleased](https://github.com/Piturnah/gex/compare/v0.3.3...master)
### Added
- Warning when opening Gex with locale other than English ([#13](https://github.com/Piturnah/gex/issues/13))
- Colour coding of `--help` flag output
### Changed
- Minibuffer now maintains a stack of messages so messages are not lost if more than one is sent per frame
- Display an error instead of panicking on invalid UTF8 from a git process
### Removed
- Colouring of `+` and `-` in stdout propagation
### Fixed
- Sometimes showing empty messages in minibuffer, for example after creating a commit

## [0.3.3](https://github.com/Piturnah/gex/compare/v0.3.2...v0.3.3) - 2022-08-30
### Changed
- Errors are reported properly instead of panicking

## [0.3.2](https://github.com/Piturnah/gex/compare/v0.3.1...v0.3.2) - 2022-08-21
### Fixed
- Showing first heading in bold before initial commit
- Not showing previous commit information on initial commit ([#6](https://github.com/Piturnah/gex/issues/6))
- Displaying diff of new files with an extra space at beginning of all lines other than first

## [0.3.1](https://github.com/Piturnah/gex/compare/v0.3.0...v0.3.1) - 2022-08-19
### Fixed
- gex crashes on repositories with no commits

## [0.3.0](https://github.com/Piturnah/gex/compare/v0.2.2...v0.3.0) - 2022-08-19
### Added
- Most recent commit hash and title displayed in status
- <kbd>:</kbd> to execute arbitrary git command
- Two new commit commands
  - extend - add additional changes to previous commit
  - amend - fix commit message
- UI to display available commit commands
- Colouring of `+` and `-` in stdout propagation
- `--help` or `-h` flag for help information
### Changed
- Use <kbd>c</kbd> <kbd>c</kbd> to create a commit
- `-v` flag changed to `-V`
- User is notified of unrecognised command line arguments and gex exits instead of quietly ignoring
- Status says "Unstaged changes" and "Staged changes" instead of "files"
- Propagate all of stdout instead of only first line
### Fixed
- Showing empty stdout or stderr in the case that the exit code didn't match 
- gex not recognising git repositories from within subdirectories ([#2](https://github.com/Piturnah/gex/issues/2))

## [0.2.2](https://github.com/Piturnah/gex/compare/v0.2.1...v0.2.2) - 2022-08-15
### Added
- `--version` or `-v` flag to display gex version
- Notice if there are no existing branches in branch list
- Propagation of errors and stdout from git subprocesses
- <kbd>F</kbd> to pull remote changes
### Fixed
- gex freezing on viewing branch list before initial commit

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
