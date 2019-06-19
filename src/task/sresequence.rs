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
use crate::shell::{Context, Process};
use crate::sre::{Buffer, Invocation};
use nix::unistd;
use std::cell::RefCell;
use std::io::{stdin, stdout};
use std::rc::Rc;

pub struct SRESequence {
    seq: parser::SRESequence,
    started: bool,
    process: Option<Rc<RefCell<Process>>>,
}

impl SRESequence {
    pub fn new(seq: parser::SRESequence) -> Self {
        SRESequence {
            seq,
            started: false,
            process: None,
        }
    }

    fn do_exec(&self) {
        let mut prev_address = None;
        let mut buf = Buffer::new(stdin()).unwrap();
        for prog in &self.seq.0 {
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
}

impl TaskImpl for SRESequence {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        ctx.state.if_condition_ok = None;
        if !self.started {
            self.process_start(ctx)?;
            self.started = true;
        }

        self.process.as_ref().unwrap().borrow_mut().poll()
    }
}
