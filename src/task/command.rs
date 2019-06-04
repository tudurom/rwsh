use super::*;
use crate::builtin;
use crate::parser;
use crate::shell::{Context, Process};
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
                if let Err(e) = unistd::execvp(
                    &os2c(OsStr::new(&self.args[0].as_str())),
                    self.args
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

    fn get_args(&mut self, _ctx: &Context) {
        self.args.push(word_to_str(self.cmd.0.clone()));
        for arg in &self.cmd.1 {
            self.args.push(word_to_str(arg.clone()));
        }
    }
}

impl TaskImpl for Command {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        ctx.state.if_condition_ok = None;
        if !self.started {
            self.get_args(ctx);
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

fn word_to_str(w: parser::Word) -> String {
    match w.borrow().deref() {
        parser::RawWord::String(s, _) => s.to_string(),
        parser::RawWord::List(ws, _) => {
            let mut s = String::new();
            for w in ws {
                s.push_str(&word_to_str(w.clone()));
            }
            s
        }
        _ => panic!(),
    }
}

fn os2c(s: &OsStr) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_e| CString::new("<string-with-nul>").unwrap())
}
