use hyprland::dispatch::DispatchType;
use hyprland::dispatch::{self, Dispatch};
use hyprland::shared::HyprDataActiveOptional;

use std::borrow::BorrowMut;
use std::process::Command;

use clap::{Parser, Subcommand, ValueEnum};

mod ssht;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Change tmux pane or WM window seamlessly
  TmuxFocus { direction: Direction },
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum Direction {
  Up,
  Down,
  Left,
  Right,
}

/// Attempts to change tmux focus in the specified direction, returns true on focus change
fn tmux_move(direction: Direction) -> bool {
  let dir_str = format!("{:?}", direction).to_lowercase();
  let tmux_dir = format!("-{}", dir_str.chars().next().unwrap().to_uppercase()); // -U -D -L -R
  let output = Command::new("tmux")
    .args([
      "display-message",
      "-p",
      &format!(
        "#{{pane_at_{}}}",
        match direction {
          Direction::Up => "top",
          Direction::Down => "bottom",
          _ => &dir_str,
        }
      ),
    ])
    .output()
    .unwrap()
    .stdout;
  let has_pane_in_direction = output[0] == b'0';
  if has_pane_in_direction {
    Command::new("tmux").args(["select-pane", &tmux_dir]).status().unwrap();
    return true;
  }
  false
}

// inspired by https://github.com/intrntbrn/awesomewm-vim-tmux-navigator
fn tmux_focus(direction: Direction) {
  let hdirection = match direction {
    Direction::Up => dispatch::Direction::Up,
    Direction::Down => dispatch::Direction::Down,
    Direction::Left => dispatch::Direction::Left,
    Direction::Right => dispatch::Direction::Right,
  };

  let mut win = match hyprland::data::Client::get_active().unwrap() {
    Some(win) => win,
    None => {
      Dispatch::call(DispatchType::MoveFocus(hdirection)).unwrap();
      return;
    }
  };

  let win = win.borrow_mut();

  if win.title.starts_with("tmux") && tmux_move(direction) {
    return;
  }

  if ssht::ssh_tmux_move(win.pid as u32, direction) {
    return;
  }

  Dispatch::call(DispatchType::MoveFocus(hdirection)).unwrap();
}

fn main() {
  let cli = Cli::parse();
  match cli.command {
    Commands::TmuxFocus { direction } => tmux_focus(direction),
  }
}
