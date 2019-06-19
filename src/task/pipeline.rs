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
use crate::shell::{Context, Fork, Process};
use nix::unistd;
use std::cell::RefCell;
use std::io::{stdin, stdout};
use std::os::unix::io::AsRawFd;
use std::process::exit;
use std::rc::Rc;

#[derive(Default)]
pub struct Pipeline {
    pub children: Vec<Task>,
    pub started: bool,

    processes: Vec<Rc<RefCell<Process>>>,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            children: vec![],
            started: false,
            processes: Vec::new(),
        }
    }

    fn start(&mut self, ctx: &mut Context) -> Result<(), String> {
        let mut last_stdout = -1;
        let len = self.children.len();
        let mut read_pipe = -1;
        let mut write_pipe;
        for (i, child) in self.children.iter_mut().enumerate() {
            write_pipe = -1;
            if i < len - 1 {
                let (r, w) = unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC).map_err(|e| {
                    if last_stdout != -1 {
                        unistd::close(last_stdout).unwrap();
                    }
                    format!("failed to pipe: {}", e)
                })?;
                read_pipe = r;
                write_pipe = w;
            }
            ctx.in_pipe = true;
            match ctx
                .state
                .fork()
                .map_err(|e| format!("couldn't fork: {}", e))?
            {
                Fork::Child => {
                    if write_pipe >= 0 {
                        unistd::close(read_pipe).unwrap();
                    }
                    if last_stdout > 0 {
                        unistd::dup2(last_stdout, stdin().as_raw_fd()).unwrap();
                        unistd::close(last_stdout).unwrap();
                    }
                    if write_pipe > 1 {
                        unistd::dup2(write_pipe, stdout().as_raw_fd()).unwrap();
                        unistd::close(write_pipe).unwrap();
                    }

                    match child.run(ctx) {
                        Ok(x) => exit(x),
                        Err(e) => eprintln!("error in pipe: {}", e),
                    }
                }
                Fork::Parent(proc) => self.processes.push(proc),
            }
            ctx.in_pipe = false;
            // child.poll(ctx)?;
            if last_stdout >= 0 {
                unistd::close(last_stdout).unwrap();
            }
            last_stdout = read_pipe;
            if write_pipe != -1 {
                unistd::close(write_pipe).unwrap();
            }
        }

        Ok(())
    }
}

impl TaskImpl for Pipeline {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.started {
            self.start(ctx)?;
            self.started = true;
        }

        let mut ret = Ok(TaskStatus::Success(0));
        for child in self.processes.iter_mut() {
            ret = child.borrow_mut().poll();
            match ret {
                Ok(TaskStatus::Success(_)) => {}
                _ => return ret,
            }
        }

        ret
    }
}
