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

use crate::parser::{Parser, Program, WordParameterBracket};
use crate::task::{Task, TaskStatus};
use crate::util::{BufReadChars, InteractiveLineReader, LineReader};
use nix::sys::wait::WaitStatus;
use nix::unistd::{self, ForkResult, Pid};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::process::exit;
use std::rc::Rc;

pub enum Fork {
    Child,
    Parent(Rc<RefCell<Process>>),
}

#[derive(Clone, Debug)]
pub enum VarValue {
    Array(Vec<String>),
}

impl VarValue {
    pub fn array(&self) -> &Vec<String> {
        match self {
            VarValue::Array(arr) => arr,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Var {
    pub key: String,
    pub value: VarValue,
}

impl Var {
    pub fn new(key: String, value: VarValue) -> Var {
        Var { key, value }
    }
}

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self.value {
                VarValue::Array(arr) => {
                    arr.join(if self.key.ends_with("PATH") { ":" } else { " " })
                }
            }
        )
    }
}

#[derive(Clone, Default)]
/// The config options of the shell.
pub struct Config {
    pub pretty_print: bool,
}

#[derive(Copy, Clone, Debug)]
pub enum Key<'a> {
    Var(&'a str),
    Index(&'a str, usize),
}

impl<'a> Key<'a> {
    pub fn name(&self) -> &str {
        match self {
            Key::Var(name) => name,
            Key::Index(name, _) => name,
        }
    }

    pub fn new(s: &'a str) -> Key<'a> {
        let param = Parser::get_word_parameter(s).unwrap();
        match param.bracket {
            WordParameterBracket::None => Key::Var(&param.name),
            WordParameterBracket::Index(index) => Key::Index(&param.name, index),
        }
    }
}

impl<'a> std::fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Var(name) => write!(f, "{}", name),
            Key::Index(name, index) => write!(f, "{}[{}]", name, index),
        }
    }
}

#[derive(Clone)]
/// The current state of the shell.
pub struct State {
    pub exit: i32,
    pub processes: Vec<Rc<RefCell<Process>>>,
    pub scope: u32,
    pub vars: HashMap<String, Vec<(Var, u32)>>,
    pub last_status: i32,
    pub if_condition_ok: Option<bool>,
    pub config: Config,
    pub process: Option<Rc<RefCell<Process>>>,
    pub parser: Rc<RefCell<Parser>>,

    pub exported_vars: HashMap<String, String>,
    pub computed_exported_vars: Vec<String>,
}

fn read_vars() -> HashMap<String, Var> {
    let mut v = HashMap::new();
    for (key, value) in env::vars() {
        let var = if key.ends_with("PATH") {
            Var::new(
                key,
                VarValue::Array(value.split(':').map(|x| x.to_owned()).collect()),
            )
        } else {
            Var::new(key, VarValue::Array(vec![value]))
        };
        v.insert(var.key.clone(), var);
    }
    v
}

impl State {
    pub fn new(config: Config, parser: Rc<RefCell<Parser>>) -> State {
        let vars = read_vars();
        let mut s = State {
            exit: -1,
            last_status: 0,
            processes: Vec::new(),
            scope: 0,
            vars: vars
                .iter()
                .map(|(k, v)| (k.clone(), vec![(v.clone(), 0)]))
                .collect(),
            if_condition_ok: None,
            config,
            process: None,
            parser,

            exported_vars: vars
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
            computed_exported_vars: Vec::new(),
        };
        s.compute_exported_vars();
        s
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

    fn compute_exported_vars(&mut self) {
        self.computed_exported_vars = self
            .exported_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
    }

    /// Puts a value in the exported set.
    /// Values in the exported set must be explicitly set.
    /// Assigning a new value to a variable that has the same name
    /// as the exported variable does not change the exported variable's value.
    /// They are two, independent variables.
    pub fn export_var(&mut self, key: String, value: String) {
        self.exported_vars.insert(key, value);
        self.compute_exported_vars();
    }

    /// Removes a value from the exported set.
    /// If there's a normal variable with the same name, it will remain available.
    pub fn unexport_var(&mut self, key: &str) {
        self.exported_vars.remove(key);
        self.compute_exported_vars()
    }

    /// Sets a variable's name. The variable is created in the current scope if it doesn't exit,
    /// or if `create_new` is true. If `create_new` is `false`, and the variable's scope is a parent
    /// of the current scope, the variable's value will be set to `value`.
    /// If `create_new` is `true` and the variable already exists in the scope, it will not be created again either.
    /// Only its value will change.
    pub fn set_var(&mut self, key: Key, mut value: Var, mut create_new: bool) {
        if !self.vars.contains_key(key.name()) {
            self.vars.insert(key.name().to_owned(), Vec::new());
        }
        let v = self.vars.get_mut(key.name()).unwrap();

        create_new = create_new || v.is_empty();
        // don't put variables with the same scope on the stack
        if !v.is_empty() && v.last().unwrap().1 == self.scope {
            create_new = false;
        }
        let current = if create_new {
            Var::new(key.name().to_owned(), VarValue::Array(vec![]))
        } else {
            v.last().unwrap().clone().0
        };
        match current.value {
            VarValue::Array(arr) => {
                if let Key::Index(_, index) = key {
                    let mut new_value = arr.clone();
                    if index >= new_value.len() {
                        new_value.resize(index + 1, String::new());
                    }
                    new_value[index] = value.value.array().get(0).cloned().unwrap_or_default();

                    value = Var::new(key.name().to_owned(), VarValue::Array(new_value))
                };
                if create_new {
                    v.push((value, self.scope));
                } else {
                    let scope = v.last().unwrap().1;
                    *v.last_mut().unwrap() = (value, scope);
                }
            }
        }
    }

    /// Removes the variable. If the variable was created in a scope other than the root,
    /// it is only masked: a new variable with the same name but in the current scope
    /// is created with a null value.
    pub fn remove_var(&mut self, key: &str) {
        if self.scope == 0 {
            self.vars.remove(key);
        } else if self.vars.contains_key(key) {
            let v = self.vars.get_mut(key).unwrap();
            assert!(v.last().unwrap().1 <= self.scope);
            self.set_var(
                Key::Var(key),
                Var::new(key.to_owned(), VarValue::Array(vec![])),
                true,
            );
        }
    }

    pub fn get_var(&self, key: Key) -> Option<Var> {
        let name = key.name();
        let var = self.vars.get(name).map(|vec| vec.last().unwrap().0.clone());
        match key {
            Key::Var(_) => var.or_else(|| {
                self.exported_vars
                    .get(name)
                    .cloned()
                    .map(|ex| Var::new(name.to_owned(), VarValue::Array(vec![ex])))
            }),
            Key::Index(_, index) => match var {
                Some(var) => match var.value {
                    VarValue::Array(arr) => arr.get(index).cloned().map(|el| {
                        Var::new(format!("{}[{}]", name, index), VarValue::Array(vec![el]))
                    }),
                },
                None => None,
            },
        }
    }

    pub fn fork(&mut self) -> Result<Fork, Box<Error>> {
        let fr = unistd::fork()?;
        match fr {
            ForkResult::Child => {
                // Get rid of opened files.
                // This should be only the current script, if any.
                self.parser.borrow_mut().blindfold();
                Ok(Fork::Child)
            }
            ForkResult::Parent { child: pid, .. } => {
                self.process = Some(self.new_process(pid));
                Ok(Fork::Parent(self.process.as_ref().unwrap().clone()))
            }
        }
    }

    pub fn begin_scope(&mut self) {
        self.scope += 1;
    }

    pub fn end_scope(&mut self) {
        let s = self.scope;
        let mut to_remove = Vec::new();
        for (k, vec) in &mut self.vars {
            vec.retain(|(_, scope)| {
                assert!(*scope <= s);
                *scope < s
            });
            if vec.is_empty() {
                to_remove.push(k.clone());
            }
        }
        for t in to_remove {
            self.vars.remove(&t);
        }
        self.scope -= 1;
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
    pub in_pipe: bool,
}

impl<'a> Context<'a> {
    pub fn get_parameter_value(&self, key: Key) -> Option<Var> {
        match key.name() {
            "" => Some(Var::new(
                "".to_owned(),
                VarValue::Array(vec!["$".to_owned()]),
            )),
            "?" => Some(Var::new(
                "?".to_owned(),
                VarValue::Array(vec![self.state.last_status.to_string()]),
            )),
            _ => self.state.get_var(key),
        }
    }
}

/// The shell engine with its internal state.
///
/// Use it with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html) to get an interactive shell.
pub struct Shell {
    p: Rc<RefCell<Parser>>,
    state: State,
    interactive: bool,
}

impl Shell {
    /// Create a new `Shell` with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html).
    pub fn new_interactive(config: Config) -> Shell {
        Self::new(Box::new(InteractiveLineReader::new()), config, true)
    }

    /// Returns a new `Shell` with the given [`LineReader`](../util/trait.LineReader.html).
    pub fn new(r: Box<LineReader>, config: Config, interactive: bool) -> Shell {
        let buf = BufReadChars::new(r);
        let p = Rc::new(RefCell::new(Parser::new(buf)));
        Shell {
            p: p.clone(),
            state: State::new(config, p.clone()),
            interactive,
        }
    }

    /// Start the REPL.
    pub fn run(&mut self) {
        self.install_signal_handlers();
        while self.state.exit == -1 {
            let t = match self.p.borrow_mut().by_ref().next() {
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
                            // TODO: break on error
                            // if (status.0 != 0 && break_on_error_flag) {
                            //     self.state.exit = status.0
                            // }
                        }
                        Err(error) => eprintln!("{}", error),
                    }
                }
            } else if let Err(e) = t {
                eprintln!("{}", e);
                if !self.interactive {
                    exit(1);
                }
                self.p.borrow_mut().reload();
            }
        }
        exit(self.state.exit);
    }

    fn install_signal_handlers(&self) {
        // nothing yet
    }
}

pub fn run_program(p: Program, state: &mut State) -> Result<(i32, Context), Box<Error>> {
    let mut task = Task::new_from_command_lists(p.0, false);
    let mut ctx = Context {
        state,
        in_pipe: false,
    };
    let r = task.run(&mut ctx)?;
    Ok((r, ctx))
}
