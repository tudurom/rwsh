/* Copyright (C) 2019 Tudor-Ioan Roman
 *
 * This file is part of the Really Weird Shell, also known as RWSH.
 *
 * RWSH is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * RWSH is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with RWSH. If not, see <http://www.gnu.org/licenses/>.
 */
pub mod pretty;

use crate::parser::{Parser, Program};
use crate::task::{Task, TaskStatus};
use crate::util::{BufReadChars, InteractiveLineReader, LineReader};
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::process::exit;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Var {
    String(String),
}

impl std::string::ToString for Var {
    fn to_string(&self) -> String {
        match self {
            Var::String(s) => s.clone(),
        }
    }
}

#[derive(Clone, Default)]
/// The config options of the shell.
pub struct Config {
    pub pretty_print: bool,
}

#[derive(Clone, Default)]
/// The current state of the shell.
pub struct State {
    pub exit: i32,
    pub processes: Vec<Rc<RefCell<Process>>>,
    pub vars: HashMap<String, Var>,
    pub last_status: i32,
    pub if_condition_ok: Option<bool>,
    pub config: Config,
}

fn read_vars() -> HashMap<String, Var> {
    let mut v = HashMap::new();
    for (key, value) in env::vars() {
        v.insert(key, Var::String(value));
    }
    v
}

impl State {
    pub fn new(config: Config) -> State {
        State {
            exit: -1,
            last_status: 0,
            processes: Vec::new(),
            vars: read_vars(),
            if_condition_ok: None,
            config,
        }
    }

    pub fn new_process(&mut self, pid: Pid) -> Rc<RefCell<Process>> {
        let p = Process {
            pid,
            terminated: false,
            stat: WaitStatus::StillAlive,
        };
        self.processes.push(Rc::new(RefCell::new(p)));
        self.processes.last().unwrap().clone()
    }

    pub fn update_process(&mut self, pid: Pid, stat: WaitStatus) {
        let p = self.processes.iter().find(|p| p.borrow().pid == pid);
        if p.is_none() {
            eprintln!("rwsh: warning: tried to update lost process {}", pid);
            return; // poor process got lost
        }
        let p = p.unwrap();

        match stat {
            WaitStatus::Exited(_, _) | WaitStatus::Signaled(_, _, _) => {
                let mut p = p.borrow_mut();
                p.terminated = true;
                p.stat = stat;
            }
            _ => {}
        }
    }

    pub fn set_var(&mut self, key: String, value: Var) {
        match &value {
            Var::String(s) => {
                if s.is_empty() && self.vars.contains_key(&key) {
                    self.vars.remove(&key);
                } else {
                    self.vars.insert(key, value);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Process {
    pub pid: Pid,
    pub terminated: bool,
    pub stat: WaitStatus,
}

impl Process {
    pub fn poll(&mut self) -> Result<TaskStatus, String> {
        if !self.terminated {
            Ok(TaskStatus::Wait)
        } else {
            match self.stat {
                WaitStatus::Exited(_, code) => Ok(TaskStatus::Success(code)),
                WaitStatus::Signaled(_, sig, _) => Ok(TaskStatus::Success(
                    128 + unsafe { std::mem::transmute::<nix::sys::signal::Signal, i32>(sig) },
                )),
                _ => panic!(),
            }
        }
    }
}

/// A context holds state information and per-job information.
/// It is guaranteed to be shared across all members of a job.
pub struct Context<'a> {
    pub state: &'a mut State,
}

impl<'a> Context<'a> {
    pub fn get_parameter_value(&self, name: &str) -> Option<String> {
        match name {
            "" => Some("$".to_owned()),
            "?" => Some(self.state.last_status.to_string()),
            _ => self.state.vars.get(name).map(ToString::to_string),
        }
    }
}

/// The shell engine with its internal state.
///
/// Use it with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html) to get an interactive shell.
pub struct Shell {
    p: Parser,
    state: State,
}

impl Shell {
    /// Create a new `Shell` with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html).
    pub fn new_interactive(config: Config) -> Shell {
        Self::new(Box::new(InteractiveLineReader::new()), config)
    }

    /// Returns a new `Shell` with the given [`LineReader`](../util/trait.LineReader.html).
    pub fn new(r: Box<LineReader>, config: Config) -> Shell {
        let buf = BufReadChars::new(r);
        let p = Parser::new(buf);
        Shell {
            p,
            state: State::new(config),
        }
    }

    /// Start the REPL.
    pub fn run(&mut self) {
        self.install_signal_handlers();
        while self.state.exit == -1 {
            let t = match self.p.by_ref().next() {
                None => {
                    self.state.exit = self.state.last_status;
                    break;
                }
                Some(t) => t,
            };
            if let Ok(p) = t {
                if self.state.config.pretty_print {
                    use pretty::PrettyPrint;
                    p.pretty_print().print()
                } else {
                    if p.0.is_empty() {
                        continue;
                    }
                    match run_program(p, &mut self.state) {
                        Ok(_status) => {
                            // TODO: break on exit
                            // if (status.0 != 0 && break_on_exit_flag) {
                            //     self.state.exit = status.0
                            // }
                        }
                        Err(error) => eprintln!("{}", error),
                    }
                }
            } else if let Err(e) = t {
                eprintln!("{}", e);
                exit(1);
            }
        }
        exit(self.state.exit);
    }

    fn install_signal_handlers(&self) {
        // nothing yet
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new_interactive(Default::default())
    }
}

pub fn run_program(p: Program, state: &mut State) -> Result<(i32, Context), Box<Error>> {
    let mut task = Task::new_from_command_lists(p.0);
    let mut ctx = Context { state };
    let r = task.run(&mut ctx)?;
    Ok((r, ctx))
}
