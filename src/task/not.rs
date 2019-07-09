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

pub struct Not {
    task: Task,
}

impl Not {
    pub fn new(task: Task) -> Not {
        Not { task }
    }
}

impl TaskImpl for Not {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        match self.task.poll(ctx)? {
            TaskStatus::Wait => Ok(TaskStatus::Wait),
            TaskStatus::Success(i) => Ok(TaskStatus::Success(if i == 0 { 1 } else { 0 })),
        }
    }
}
