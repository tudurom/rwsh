use super::*;
use crate::shell::Context;

pub struct WhileConstruct {
    condition: Task,
    body: Task,
}

impl WhileConstruct {
    pub fn new(condition: Task, body: Task) -> WhileConstruct {
        WhileConstruct { condition, body }
    }
}

impl TaskImpl for WhileConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        unimplemented!()
    }
}
