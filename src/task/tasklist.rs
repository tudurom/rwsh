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
