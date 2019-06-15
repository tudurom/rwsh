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

pub struct IfConstruct {
    condition: Task,
    body: Task,
}

impl IfConstruct {
    pub fn new(condition: Task, body: Task) -> IfConstruct {
        IfConstruct { condition, body }
    }
}

impl TaskImpl for IfConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let condition_status = self.condition.poll(ctx)?;
        match condition_status {
            TaskStatus::Success(0) => {
                let r = self.body.poll(ctx);
                ctx.state.if_condition_ok = Some(true);
                r
            }
            TaskStatus::Wait => Ok(TaskStatus::Wait),
            _ => {
                ctx.state.if_condition_ok = Some(false);
                Ok(TaskStatus::Success(0))
            }
        }
    }
}

pub struct ElseConstruct {
    body: Task,
    polled: bool,
}

impl ElseConstruct {
    pub fn new(body: Task) -> ElseConstruct {
        ElseConstruct {
            body,
            polled: false,
        }
    }
}

impl TaskImpl for ElseConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if ctx.state.if_condition_ok.is_none() && !self.polled {
            return Err("cannot use else without an if before it".to_owned());
        }
        if self.polled || !ctx.state.if_condition_ok.unwrap() {
            self.polled = true;
            self.body.poll(ctx)
        } else {
            ctx.state.if_condition_ok = None;
            Ok(TaskStatus::Success(ctx.state.last_status))
        }
    }
}
