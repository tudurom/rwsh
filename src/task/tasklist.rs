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

#[derive(Default)]
pub struct TaskList {
    pub children: Vec<Task>,
    pub current: usize,
}

impl TaskList {
    pub fn new() -> TaskList {
        TaskList {
            children: vec![],
            current: 0,
        }
    }
}

impl TaskImpl for TaskList {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let mut ret = Ok(TaskStatus::Wait);
        if self.children.len() == 0 {
            return Ok(TaskStatus::Success(0));
        }
        while self.current < self.children.len() {
            let child = &mut self.children[self.current];

            ret = child.poll(ctx);
            match ret {
                Ok(TaskStatus::Success(_)) => {}
                _ => return ret,
            }

            self.current += 1;
        }

        ret
    }
}
