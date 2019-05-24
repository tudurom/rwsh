use super::*;
use crate::parser;
use crate::shell::Context;
use nix::sys::wait;
use std::error::Error;
use std::ops::Deref;

#[derive(Clone, Copy)]
pub enum TaskStatus {
    Wait,
    Success(i32),
}

/// A task defines any operation that the shell might have to schedule
/// and execute to get a final result. The task's behaviour is defined
/// by its [`TaskImpl`](trait.TaskImpl.html). The status of the task
/// is defined by [`TaskStatus`](enum.TaskStatus.html).
///
/// Task can have other nested task, such as functions.
pub struct Task {
    ti: Box<TaskImpl>,
    status: Result<TaskStatus, String>,
}

impl Task {
    pub fn new(ti: Box<TaskImpl>) -> Self {
        Task {
            ti,
            status: Ok(TaskStatus::Wait),
        }
    }

    pub fn run(&mut self, ctx: &mut Context) -> Result<i32, Box<Error>> {
        loop {
            let p = self.poll(ctx)?;
            if let TaskStatus::Success(code) = p {
                ctx.state.last_status = code;
                return Ok(code);
            }
            let stat = wait::waitpid(None, None)?;
            ctx.state.update_process(stat.pid().unwrap(), stat);
        }
    }

    /// Executes the implementation's polling function if still waiting.
    /// Returns the result without executing a second time if not waiting.
    pub fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if let Ok(TaskStatus::Wait) = self.status {
            self.status = self.ti.poll(ctx);
        }
        self.status.clone()
    }

    pub fn new_from_word(word: parser::Word, expand_tilde: bool) -> Self {
        if let parser::RawWord::List(children, double_quoted) = word.borrow().deref() {
            let mut tl = TaskList::new();
            for child in children {
                tl.children.push(Self::new_from_word(
                    child.clone(),
                    !double_quoted && expand_tilde,
                ));
            }
            return Task::new(Box::new(tl));
        }

        let tw = Word::new(word, expand_tilde);
        Task::new(Box::new(tw))
    }

    pub fn new_from_simple_command(c: parser::SimpleCommand) -> Self {
        let mut tl = TaskList::new();
        let sc = c.clone();
        tl.children.push(Self::new_from_word(sc.0.clone(), true));
        for arg in c.1 {
            tl.children.push(Self::new_from_word(arg, true));
        }
        tl.children.push(Task::new(Box::new(Command::new(sc))));

        Task::new(Box::new(tl))
    }

    pub fn new_from_sre_sequence(seq: parser::SRESequence) -> Self {
        Task::new(Box::new(SRESequence::new(seq)))
    }

    pub fn new_from_if(condition: parser::Program, body: parser::Program) -> Self {
        Task::new(Box::new(IfConstruct::new(
                    Self::new_from_command_lists(condition.0),
                    Self::new_from_command_lists(body.0))))
    }

    pub fn new_from_pipeline(p: parser::Pipeline) -> Self {
        let mut tp = Pipeline::new();

        for pi in p.0 {
            tp.children.push(match pi {
                parser::Command::SimpleCommand(c) => Self::new_from_simple_command(c),
                parser::Command::SREProgram(seq) => Self::new_from_sre_sequence(seq),
                parser::Command::BraceGroup(arr) => Self::new_from_command_lists(arr),
                parser::Command::IfConstruct(condition, body) => Self::new_from_if(condition, body),
            });
        }

        Self::new(Box::new(tp))
    }

    pub fn new_from_node(n: parser::Node) -> Self {
        match n {
            parser::Node::Pipeline(p) => Self::new_from_pipeline(p),
        }
    }

    pub fn new_from_command_lists(v: Vec<parser::CommandList>) -> Self {
        let mut tl = TaskList::new();

        for cl in v {
            let child = Self::new_from_node(cl.0);
            tl.children.push(child);
        }

        Self::new(Box::new(tl))
    }
}

/// Defines the behaviour of a [`Task`](struct.Task.html).
pub trait TaskImpl {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String>;
}
