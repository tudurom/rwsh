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
use crate::shell::Context;
use nix::unistd;
use std::io::{stdin, stdout};
use std::os::unix::io::AsRawFd;

#[derive(Default)]
pub struct Pipeline {
    pub children: Vec<Task>,
    pub started: bool,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            children: vec![],
            started: false,
        }
    }

    fn start(&mut self, ctx: &mut Context) -> Result<(), String> {
        let (dup_stdin, dup_stdout) = if self.children.len() > 1 {
            (
                unistd::dup(stdin().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdin: {}", e))?,
                unistd::dup(stdout().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdout: {}", e))?,
            )
        } else {
            (-1, -1)
        };

        let mut last_stdout = -1;
        let len = self.children.len();
        for (i, child) in self.children.iter_mut().enumerate() {
            if i > 0 {
                unistd::dup2(last_stdout, stdin().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdin: {}", e))?;
                unistd::close(last_stdout).unwrap();
            }

            let new_stdout = if i < len - 1 {
                let (read_pipe, write_pipe) = unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC)
                    .map_err(|e| format!("failed to pipe: {}", e))?;
                last_stdout = read_pipe;
                write_pipe
            } else {
                dup_stdout
            };

            if new_stdout >= 0 {
                unistd::dup2(new_stdout, stdout().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdout: {}", e))?;
                unistd::close(new_stdout).unwrap();
            }

            child.poll(ctx)?;
        }

        if dup_stdin >= 0 {
            unistd::dup2(dup_stdin, stdin().as_raw_fd())
                .map_err(|e| format!("failed to duplicate stdin: {}", e))?;
            unistd::close(dup_stdin).unwrap();
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
        for child in self.children.iter_mut() {
            ret = child.poll(ctx);
            match ret {
                Ok(TaskStatus::Success(_)) => {}
                _ => return ret,
            }
        }

        ret
    }
}
