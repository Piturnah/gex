#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::redundant_closure_for_method_calls,
    clippy::module_name_repetitions,
    clippy::let_underscore_untyped
)]

use std::{
    cmp, env,
    io::{stdin, stdout, BufRead, Write},
    panic,
    process::{self, Command, Output},
    rc::Rc,
};

use anyhow::{Context, Result};
use clap::Parser;
use config::Clargs;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    style::{Attribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use git2::Repository;

use crate::{
    command::GexCommand,
    config::{Config, CONFIG},
    minibuffer::{Callback, MessageType, MiniBuffer},
    render::{Clear, Render, ResetAttributes},
};

mod branch;
mod command;
mod config;
mod debug;
mod minibuffer;
mod parse;
mod render;
mod status;

use branch::BranchList;
use render::Renderer;
use status::Status;

pub struct State {
    view: View,
    minibuffer: MiniBuffer,
    status: Status,
    branch_list: BranchList,
    repo: Repository,
    renderer: Renderer,
}

#[derive(Clone)]
pub enum View {
    Status,
    BranchList,
    Command(GexCommand),
    Input(Callback, Box<View>),
}

pub fn git_process(args: &[&str]) -> Result<Output> {
    Command::new("git").args(args).output().with_context(|| {
        format!(
            "failed to run `git{}`",
            args.iter().map(|a| " ".to_string() + a).collect::<String>()
        )
    })
}

fn run(clargs: &Clargs) -> Result<()> {
    // Attempt to find a git repository at or above current path
    let repo = if let Ok(repo) = Repository::discover(&clargs.path) {
        repo
    } else {
        print!("Not a git repository. Initialise one? [y/N]");
        drop(stdout().flush());
        let input = stdin()
            .lock()
            .lines()
            .next()
            .context("couldn't read stdin")?
            .context("malformed stdin")?;
        if input.to_lowercase() != "y" {
            process::exit(0);
        }

        Repository::init(&clargs.path).context("failed to initialise git repository")?
    };

    // Set working directory in case the repository is not the current directory
    std::env::set_current_dir(repo.path().parent().context("`.git` cannot be root dir")?)
        .context("failed to set working directory")?;

    let minibuffer = MiniBuffer::new();

    let config = CONFIG.get_or_init(|| {
        Config::read_from_file(&clargs.config_file)
            .unwrap_or_else(|e| {
                MiniBuffer::push(&format!("{e:?}"), MessageType::Error);
                Some((Config::default(), Vec::new()))
            })
            .map_or_else(Config::default, |(config, unused_keys)| {
                if !unused_keys.is_empty() {
                    let mut warning = String::from("Unknown keys in config file:");
                    for key in unused_keys {
                        warning.push_str("\n    ");
                        warning.push_str(&key);
                    }
                    MiniBuffer::push(&warning, MessageType::Error);
                }
                config
            })
    });

    let status = Status::new(&repo, &config.options)?;
    let branch_list = BranchList::new()?;
    let view = View::Status;
    let renderer = Renderer::default();

    let mut state = State {
        view,
        minibuffer,
        status,
        branch_list,
        repo,
        renderer,
    };

    // Non-English locale settings are currently unsupported. See
    // https://github.com/Piturnah/gex/issues/13.
    if !env::var("LANG")
        .map(|s| s.starts_with("en"))
        .unwrap_or(true)
    {
        MiniBuffer::push("WARNING: Non-English locale detected. For now, Gex only supports English locale setting.
Set locale to English, e.g.:

        $ LANG=en_GB gex

See https://github.com/Piturnah/gex/issues/13.", MessageType::Error);
    }

    // We are about to start messing with the terminal settings. So let's update the panic hook so
    // that the panic messages will be displayed cleanly.
    let panic = panic::take_hook();
    panic::set_hook(Box::new(move |e| {
        restore_terminal();
        panic(e);
    }));

    crossterm::execute!(stdout(), terminal::EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    terminal::enable_raw_mode().context("failed to put terminal in raw mode")?;
    print!("{}", cursor::Hide);

    // Structure of the event loop
    //
    // 1. Clear the terminal
    // 2. Render status or branch list
    // 3. Render option overlay
    // 4. Render minibuffer messages
    // 5. Wait for event and update state
    //
    loop {
        let (term_width, term_height) =
            terminal::size().context("failed to query terminal dimensions")?;

        print!("{ResetAttributes}");
        match state.view {
            View::Status | View::Command(_) | View::Input(..) => {
                state.status.render(&mut state.renderer)?;
            }
            View::BranchList => state.branch_list.render(&mut state.renderer)?,
        }
        state.renderer.show_and_clear(
            term_width as usize,
            term_height as usize,
            config.options.lookahead_lines,
            config.options.truncate_lines,
        );
        drop(stdout().flush());

        // Display the available subcommands
        if let View::Command(cmd) = state.view {
            use std::fmt::Write;
            let subcmds = cmd.subcommands();
            print!(
                "{}{title:═^term_width$}{}{}{}",
                cursor::MoveTo(0, term_height - 1 - subcmds.len() as u16),
                Clear(ClearType::FromCursorDown),
                subcmds.iter().fold(String::new(), |mut acc, (k, v)| {
                    let _ = write!(
                        acc,
                        "\r\n {}{}{k}{} => {v}",
                        SetForegroundColor(config.colors.key),
                        Attribute::Bold,
                        ResetAttributes
                    );
                    acc
                }),
                SetForegroundColor(config.colors.foreground),
                term_width = term_width as usize,
                title = format!(" {cmd:?} Options "),
            );

            drop(stdout().flush());
        }

        // Draw the current `debug!` window.
        debug_draw!();

        state.minibuffer.pop_message();
        state.minibuffer.render(term_width, term_height)?;

        // Handle input
        //
        // Check what event we get. If we got an event other than a key event, we don't need to
        // handle it so we break. If we got a key event with KeyEventKind::Release, we try again in
        // the loop to avoid re-rendering. If it's a key event without KeyEventKind::Release,
        // handle it and break.
        loop {
            let Event::Key(event) = event::read().context("failed to read a terminal event")?
            else {
                break;
            };
            if event.kind == KeyEventKind::Release {
                continue;
            }

            if !MiniBuffer::is_empty() {
                break;
            }

            match state.view {
                View::Status => match event.code {
                    KeyCode::Down => state.status.down()?,
                    KeyCode::Up => state.status.up()?,
                    KeyCode::Char('s') => {
                        if state.status.cursor
                            < state.status.count_untracked + state.status.count_unstaged
                        {
                            state.status.stage()?;
                            state.status.fetch(&state.repo, &config.options)?;
                        }
                    }
                    KeyCode::Char('S') => {
                        MiniBuffer::push_command_output(&git_process(&["add", "."])?);
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Char('u') => {
                        if state.status.cursor
                            >= state.status.count_untracked + state.status.count_unstaged
                        {
                            state.status.unstage()?;
                            state.status.fetch(&state.repo, &config.options)?;
                        }
                    }
                    KeyCode::Char('U') => {
                        MiniBuffer::push_command_output(&git_process(&["reset"])?);
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Tab | KeyCode::Char(' ') => state.status.expand()?,
                    KeyCode::Char('e') => {
                        state.status.open_editor()?;
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Char('F') => {
                        MiniBuffer::push_command_output(&git_process(&["pull"])?);
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Char('r') => state.status.fetch(&state.repo, &config.options)?,
                    KeyCode::Char(':') => {
                        state.minibuffer.command(true, &mut state.view);
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Char('!') => {
                        state.minibuffer.command(false, &mut state.view);
                        state.status.fetch(&state.repo, &config.options)?;
                    }
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to disable raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    KeyCode::Char(c1) => {
                        if let Some((_, cmd)) =
                            GexCommand::commands().iter().find(|(c2, _)| c1 == *c2)
                        {
                            state.view = View::Command(*cmd);
                        }

                        if c1 == config.keymap.navigation.move_down {
                            state.status.down()?
                        }

                        if c1 == config.keymap.navigation.move_up {
                            state.status.up()?
                        }

                        if c1 == config.keymap.navigation.next_file {
                            state.status.file_down()?
                        }

                        if c1 == config.keymap.navigation.previous_file {
                            state.status.file_up()?
                        }

                        if c1 == config.keymap.navigation.toggle_expand {
                            state.status.expand()?
                        }

                        if c1 == config.keymap.navigation.goto_bottom {
                            state.status.cursor_last()?
                        }

                        if c1 == config.keymap.navigation.goto_top {
                            state.status.cursor_first()?
                        }
                    }
                    _ => {}
                },
                View::BranchList => match event.code {
                    KeyCode::Up => {
                        state.branch_list.cursor = state.branch_list.cursor.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        state.branch_list.cursor = cmp::min(
                            state.branch_list.cursor + 1,
                            state.branch_list.branches.len() - 1,
                        );
                    }
                    KeyCode::Char('g' | 'K') => state.branch_list.cursor = 0,
                    KeyCode::Char('G' | 'J') => {
                        state.branch_list.cursor = state.branch_list.branches.len() - 1;
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        MiniBuffer::push_command_output(&state.branch_list.checkout()?);
                        state.status.fetch(&state.repo, &config.options)?;
                        state.view = View::Status;
                    }
                    KeyCode::Esc => state.view = View::Status,
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to disable raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    KeyCode::Char(c1) => {
                        if c1 == config.keymap.navigation.move_down {
                            state.branch_list.cursor = cmp::min(
                                state.branch_list.cursor + 1,
                                state.branch_list.branches.len() - 1,
                            );
                        }

                        if c1 == config.keymap.navigation.move_up {
                            state.branch_list.cursor = state.branch_list.cursor.saturating_sub(1);
                        }

                        // TODO: g, G, J, K
                    }
                    _ => {}
                },
                View::Command(cmd) => match event.code {
                    KeyCode::Esc => state.view = View::Status,
                    KeyCode::Char('q') => {
                        terminal::disable_raw_mode().context("failed to exit raw mode")?;
                        crossterm::execute!(
                            stdout(),
                            terminal::LeaveAlternateScreen,
                            cursor::Show,
                            cursor::MoveToColumn(0)
                        )
                        .context("failed to leave alternate screen")?;
                        process::exit(0);
                    }
                    KeyCode::Char(c) => cmd.handle_input(c, &mut state, config)?,
                    _ => {}
                },
                View::Input(ref callback, ref return_view) => {
                    // This clone should be very cheap as we should never be constructing a
                    // View::Input with the return view as View::Input.
                    //
                    // NOTE: This all indicates there is probably a better way to represent the
                    // View type, as it never actually needs to be recursive -- then we would also
                    // be able to just #[derive(Copy)].
                    debug_assert!(!matches!(**return_view, View::Input(..)));
                    state.minibuffer.handle_input(
                        event,
                        &Rc::clone(callback),
                        (**return_view).clone(),
                        &mut state.view,
                    )?;
                }
            };
            break;
        }
    }
}

/// Restore the terminal to its original state from before we messed with it.
fn restore_terminal() {
    drop(terminal::disable_raw_mode());
    drop(crossterm::execute!(
        stdout(),
        terminal::LeaveAlternateScreen,
        cursor::Show,
        cursor::MoveToColumn(0)
    ));
}

fn main() -> Result<()> {
    run(&Clargs::parse()).map_err(|e| {
        restore_terminal();
        e
    })
}
