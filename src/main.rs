use hyprland::dispatch::DispatchType;
use hyprland::dispatch::{self, Dispatch};
use hyprland::shared::HyprDataActiveOptional;
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response};

use std::borrow::BorrowMut;
use std::env;
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
fn tmux_focus_hyprland(direction: Direction) {
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

fn tmux_focus_niri(direction: Direction) {
  let action = match direction {
    Direction::Up => Action::FocusWindowUp {},
    Direction::Down => Action::FocusWindowDown {},
    Direction::Left => Action::FocusColumnLeft {},
    Direction::Right => Action::FocusColumnRight {},
  };

  let mut socket = Socket::connect().unwrap();
  let reply = socket.send(Request::FocusedWindow).unwrap();
  let res = reply.unwrap();
  let Response::FocusedWindow(win) = res else { panic!() };

  let win = match win {
    Some(win) => win,
    None => {
      socket.send(Request::Action(action)).unwrap().unwrap();
      return;
    }
  };

  if win.title.unwrap_or_default().starts_with("tmux") && tmux_move(direction) {
    return;
  }

  if let Some(pid) = win.pid
    && ssht::ssh_tmux_move(pid as u32, direction)
  {
    return;
  }

  socket.send(Request::Action(action)).unwrap().unwrap();
}

enum Wm {
  Hyprland,
  Niri,
}

fn main() {
  use Wm::*;
  let wm = match env::var("XDG_CURRENT_DESKTOP").unwrap().as_str() {
    "Hyprland" => Hyprland,
    "niri" => Niri,
    wm => panic!("Unknown WM {wm}"),
  };
  let cli = Cli::parse();
  match (wm, cli.command) {
    (Hyprland, Commands::TmuxFocus { direction }) => tmux_focus_hyprland(direction),
    (Niri, Commands::TmuxFocus { direction }) => tmux_focus_niri(direction),
  }
}
