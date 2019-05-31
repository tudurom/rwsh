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
            Ok(TaskStatus::Success(0))
        }
    }
}
