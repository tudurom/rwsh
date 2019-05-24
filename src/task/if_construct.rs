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
            TaskStatus::Success(0) => self.body.poll(ctx),
            TaskStatus::Wait => Ok(TaskStatus::Wait),
            _ => Ok(TaskStatus::Success(0)),
        }
    }
}
