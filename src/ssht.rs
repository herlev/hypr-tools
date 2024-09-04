use core::panic;
use std::{
  io::{Read, Write},
  os::unix::net::UnixStream,
  path::{Path, PathBuf},
  str::FromStr,
};

use crate::Direction;

fn get_parent_pid(pid: u32) -> Option<u32> {
  let path = format!("/proc/{pid}/stat");
  let ppid = std::fs::read_to_string(path)
    .unwrap()
    .split(' ')
    .nth(3)
    .unwrap()
    .parse()
    .unwrap();

  match ppid {
    0 => None,
    _ => Some(ppid),
  }
}

fn get_parents_pids(mut pid: u32) -> Vec<u32> {
  let mut parent_pids = vec![];
  while let Some(parent_pid) = get_parent_pid(pid) {
    parent_pids.push(parent_pid);
    pid = parent_pid;
  }
  parent_pids
}

fn get_pid(path: &Path) -> Option<u32> {
  let s: Vec<_> = path.file_name()?.to_str()?.split('.').collect();
  if s.len() != 2 {
    return None;
  }
  s[0].parse().ok()
}

fn get_all_ssht_pids() -> Vec<u32> {
  std::fs::read_dir("/tmp/ssht/")
    .unwrap()
    .filter_map(|f| f.ok())
    .filter_map(|f| get_pid(&f.path()))
    .collect()
}

pub fn get_ssht_pid_from_ppid(ppid: u32) -> Option<u32> {
  let p: Vec<_> = get_all_ssht_pids()
    .into_iter()
    .map(|pid| (pid, get_parents_pids(pid)))
    .collect();

  let p: Vec<_> = p.into_iter().filter(|(_pid, ppids)| ppids.contains(&ppid)).collect();

  match p.len() {
    0 => None,
    1 => Some(p[0].0),
    _ => panic!("Multiple ssht processes with same parent"),
  }
}

fn move_direction(pid: u32, direction: Direction) {
  let dir_str = format!("{:?}", direction).to_lowercase();
  let socket = PathBuf::from_str(&format!("/tmp/ssht/{pid}.sock")).unwrap();
  let mut stream = UnixStream::connect(socket).unwrap();
  stream.write_all(format!("move_pane {dir_str}").as_bytes()).unwrap();
  let mut res = String::new();
  stream.read_to_string(&mut res).unwrap();
}

fn has_pane_in_direction(pid: u32, direction: Direction) -> bool {
  let dir_str = format!("{:?}", direction).to_lowercase();
  let socket = PathBuf::from_str(&format!("/tmp/ssht/{pid}.sock")).unwrap();
  let mut stream = UnixStream::connect(socket).unwrap();
  stream.write_all(format!("has_pane {dir_str}").as_bytes()).unwrap();
  let mut res = String::new();
  stream.read_to_string(&mut res).unwrap();
  match res.as_str() {
    "true" => true,
    "false" => false,
    _ => panic!("got unexpected response from ssht socket: {:?}", res.bytes()),
  }
}

pub fn ssh_tmux_move(current_window_id: u32, direction: Direction) -> bool {
  if let Some(pid) = get_ssht_pid_from_ppid(current_window_id) {
    if has_pane_in_direction(pid, direction) {
      move_direction(pid, direction);
      return true;
    }
  }
  false
}
