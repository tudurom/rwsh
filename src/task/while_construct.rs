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
use crate::parser::Program;
use crate::shell::Context;

pub struct WhileConstruct {
    condition: Program,
    body: Program,
    condition_task: Task,
    body_task: Task,
    last_body_status: Result<TaskStatus, String>,
}

impl WhileConstruct {
    pub fn new(condition: Program, body: Program) -> WhileConstruct {
        let c = condition.clone();
        let b = body.clone();
        WhileConstruct {
            condition,
            body,
            condition_task: Task::new_from_command_lists(c.0),
            body_task: Task::new_from_command_lists(b.0),
            last_body_status: Ok(TaskStatus::Wait),
        }
    }
}

impl TaskImpl for WhileConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        while ctx.state.exit == -1 {
            let condition_status = self.condition_task.poll(ctx)?;
            match condition_status {
                TaskStatus::Wait => return Ok(TaskStatus::Wait),
                TaskStatus::Success(i) if i != 0 => return self.last_body_status.clone(),
                _ => {}
            }

            let body_status = self.body_task.poll(ctx)?;
            if let TaskStatus::Wait = body_status {
                return Ok(TaskStatus::Wait);
            }
            self.last_body_status = Ok(body_status);
            self.condition_task = Task::new_from_command_lists(self.condition.clone().0);
            self.body_task = Task::new_from_command_lists(self.body.clone().0);
        }

        Ok(TaskStatus::Success(ctx.state.exit))
    }
}
