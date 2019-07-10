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
use crate::parser::BinOpType;
use crate::shell::Context;

pub struct BinOp {
    typ: BinOpType,
    left: Task,
    right: Task,
}

impl BinOp {
    pub fn new(typ: BinOpType, left: Task, right: Task) -> BinOp {
        BinOp { typ, left, right }
    }
}

impl TaskImpl for BinOp {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let left_status = match self.left.poll(ctx)? {
            TaskStatus::Wait => return Ok(TaskStatus::Wait),
            TaskStatus::Success(i) => i,
        };

        match self.typ {
            BinOpType::And => {
                if left_status != 0 {
                    return Ok(TaskStatus::Success(left_status));
                }
            }
            BinOpType::Or => {
                if left_status == 0 {
                    return Ok(TaskStatus::Success(0));
                }
            }
        }

        self.right.poll(ctx)
    }
}
