# Changelog

## [Unreleased](https://github.com/Piturnah/gex/compare/v0.4.0...main)
### Added
- Scrolling on cursor movement if content goes off-screen ([#1](https://github.com/Piturnah/gex/issues/1))
- Use <kbd>Ret</kbd> to toggle expansion of items

## [0.4.0](https://github.com/Piturnah/gex/compare/v0.3.8...v0.4.0) - 2023-07-03
### Added
- Counts indicating the number of staged/unstaged changes
- Improvements to minibuffer
  - Emacs-style cursor motion
  - Support for <kbd>Home</kbd> and <kbd>End</kbd>
- Basic stashing functionality with <kbd>z</kbd>
- Basic push functionality with <kbd>p</kbd>
### Changed
- Use <kbd>b</kbd> <kbd>b</kbd> to open the branch list
- Use <kbd>b</kbd> <kbd>n</kbd> to create a new branch
### Fixed
- Receiving double inputs on certain terminals such as Windows Terminal
- LF/CRLF warning breaks UI on hunk staging ([#26](https://github.com/Piturnah/gex/issues/26))
- Bad diff preview when external diff tool is enabled ([#28](https://github.com/Piturnah/gex/pull/28))
- Terminal left in bad state in case of panic
- Cursor disappears in status view after jumping to top ([#31](https://github.com/Piturnah/gex/issues/31))

## [0.3.8](https://github.com/Piturnah/gex/compare/v0.3.7...v0.3.8) - 2023-04-23
### Added
- Support for <kbd>Del</kbd> in minibuffer
### Changed
- Cursor switches to bar when navigating left and right in minibuffer
- Formatting of `--help` information (Clap v4)

## [0.3.7](https://github.com/Piturnah/gex/compare/v0.3.6...v0.3.7) - 2023-01-25
### Added
- Improvements to arbitary git command execution
  - Reuse commands from history with up and down arrow keys ([#19](https://github.com/Piturnah/gex/issues/19))
  - Border above the input line while typing a command
  - Esc to exit writing git command
  - Navigate currently typing git command with left and right arrow keys
### Fixed
- Error reporting for unrecognised file prefixes from git ([#18](https://github.com/Piturnah/gex/pull/18))

## [0.3.6](https://github.com/Piturnah/gex/compare/v0.3.5...v0.3.6) - 2022-12-28
### Added
- Optional argument for the repository path
### Fixed
- Not clearing text underneath commit menu ([#11](https://github.com/Piturnah/gex/issues/11))

## [0.3.5](https://github.com/Piturnah/gex/compare/v0.3.4...v0.3.5) - 2022-12-21
### Fixed
- Stdout propagation from command execution causing top of display to go off-screen
- Crashing on jumping to top/bottom of diffs when there are no diffs
- Not refreshing after executing a command with <kbd>:</kbd>
- Error reporting for failed hunk patch

## [0.3.4](https://github.com/Piturnah/gex/compare/v0.3.3...v0.3.4) - 2022-12-07
### Added
- Warning when opening Gex with locale other than English ([#13](https://github.com/Piturnah/gex/issues/13))
- New navigation controls
  - <kbd>g</kbd> / <kbd>K</kbd> to jump to first element of list
  - <kbd>G</kbd> / <kbd>J</kbd> to jump to last element of list
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
