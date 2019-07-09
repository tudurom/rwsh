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
use super::word::word_to_str;
use super::*;
use crate::builtin;
use crate::parser;
use crate::shell::{Context, Process};
use glob;
use nix::unistd;
use std::cell::RefCell;
use std::ffi::{CString, OsStr};
use std::ops::Deref;
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;

pub struct Command {
    cmd: parser::SimpleCommand,
    started: bool,
    args: Vec<String>,
    t: CommandType,
    process: Option<Rc<RefCell<Process>>>,
}

pub enum CommandType {
    Process,
    Builtin,
}

impl Command {
    pub fn new(cmd: parser::SimpleCommand) -> Self {
        Command {
            cmd,
            started: false,
            args: Vec::new(),
            t: CommandType::Process,
            process: None,
        }
    }

    fn process_start(&mut self, ctx: &mut Context) -> Result<(), String> {
        match unistd::fork().map_err(|e| format!("failed to fork: {}", e))? {
            unistd::ForkResult::Child => {
                if let Err(e) = unistd::execvpe(
                    &os2c(OsStr::new(&self.args[0].as_str())),
                    self.args
                        .iter()
                        .map(|a| os2c(OsStr::new(&a)))
                        .collect::<Vec<CString>>()
                        .as_slice(),
                    ctx.state
                        .computed_exported_vars
                        .iter()
                        .map(|a| os2c(OsStr::new(&a)))
                        .collect::<Vec<CString>>()
                        .as_slice(),
                ) {
                    eprintln!("{}: {}", self.args[0], e);
                    std::process::exit(127);
                }
                Ok(())
            }
            unistd::ForkResult::Parent { child: pid, .. } => {
                self.process = Some(ctx.state.new_process(pid));
                Ok(())
            }
        }
    }

    fn process_poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.started {
            self.process_start(ctx)?;
            self.started = true;
        }

        self.process.as_ref().unwrap().borrow_mut().poll()
    }

    fn builtin_poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        assert!(!self.started);
        let b = crate::builtin::get_builtin(&self.args[0]).unwrap();
        Ok(TaskStatus::Success((b.func)(
            ctx,
            self.args.iter().map(|s| &**s).collect::<Vec<&str>>(),
        )))
    }

    fn get_args(&mut self, _ctx: &Context) -> Result<(), String> {
        self.args.push(word_to_str(self.cmd.0.clone()));
        for word_list in &self.cmd.1 {
            let words = if let parser::RawWord::List(words, false) = word_list.borrow().deref() {
                words.clone()
            } else {
                panic!();
            };
            let mut should_glob = false;
            for word in &words {
                if let parser::RawWord::String(s, false) = word.borrow().deref() {
                    for c in s.chars() {
                        should_glob = should_glob || (c == '*' || c == '?' || c == '[');
                    }
                }
            }
            if should_glob {
                let mut original = String::new();
                for word in &words {
                    if let parser::RawWord::String(s, false) = word.borrow().deref() {
                        original.push_str(&s);
                    } else {
                        original.push_str(&glob::Pattern::escape(&word_to_str(word.clone())));
                    }
                }
                self.args.extend(
                    glob::glob(&original)
                        .map_err(|e| format!("{} ({})", e, &original))?
                        .filter_map(Result::ok)
                        .map(|p| String::from(p.to_str().unwrap())),
                );
            } else {
                self.args.push(word_to_str(word_list.clone()));
            }
        }

        Ok(())
    }
}

impl TaskImpl for Command {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        ctx.state.if_condition_ok = None;
        if !self.started {
            self.get_args(ctx)?;
            self.t = if builtin::get_builtin(&self.args[0]).is_some() {
                CommandType::Builtin
            } else {
                CommandType::Process
            };
        }

        match self.t {
            CommandType::Process => self.process_poll(ctx),
            CommandType::Builtin => self.builtin_poll(ctx),
        }
    }
}

fn os2c(s: &OsStr) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_e| CString::new("<string-with-nul>").unwrap())
}
