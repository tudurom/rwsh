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
use super::*;
use crate::parser;
use crate::parser::sre::{Command, CompleteCommand};
use crate::shell::{Context, Process};
use crate::sre::{Buffer, Invocation};
use nix::unistd;
use std::cell::RefCell;
use std::io::{stdin, stdout};
use std::rc::Rc;

pub struct SRESequence {
    ast: parser::SRESequence,
    completed: Vec<CompleteCommand>,
    started: bool,
    process: Option<Rc<RefCell<Process>>>,
}

impl SRESequence {
    pub fn new(ast: parser::SRESequence) -> Self {
        SRESequence {
            ast,
            completed: Vec::new(),
            started: false,
            process: None,
        }
    }

    fn do_exec(&self) {
        let mut prev_address = None;
        let mut buf = Buffer::new(stdin()).unwrap();
        for prog in &self.completed {
            let inv = Invocation::new(prog.clone(), &buf, prev_address).unwrap();
            let mut out = stdout();
            let addr = inv.execute(&mut out, &mut buf).unwrap();
            use std::io::Write;
            out.flush().unwrap();
            prev_address = Some(buf.apply_changes(addr));
        }
        std::process::exit(0);
    }

    fn process_start(&mut self, ctx: &mut Context) -> Result<(), String> {
        if ctx.in_pipe {
            self.do_exec();
            // does not return
        }
        match unistd::fork().map_err(|e| format!("failed to fork: {}", e))? {
            unistd::ForkResult::Child => {
                self.do_exec();
                Ok(())
            }
            unistd::ForkResult::Parent { child: pid, .. } => {
                self.process = Some(ctx.state.new_process(pid));
                Ok(())
            }
        }
    }

    fn complete_command(c: Command) -> CompleteCommand {
        let string_args = c
            .string_args
            .iter()
            .map(|w| super::word::word_to_str(w.clone()))
            .collect::<Vec<_>>();
        let command_args = if !c.command_args.is_empty() {
            c.command_args
                .iter()
                .map(|cmd| Self::complete_command(cmd.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        CompleteCommand {
            address: c.address,
            name: c.name,
            string_args,
            command_args,
            original_address: c.original_address,
        }
    }
}

impl TaskImpl for SRESequence {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        ctx.state.if_condition_ok = None;
        if !self.started {
            self.completed
                .extend(self.ast.0.iter().map(|c| Self::complete_command(c.clone())));
            self.process_start(ctx)?;
            self.started = true;
        }

        self.process.as_ref().unwrap().borrow_mut().poll()
    }
}
