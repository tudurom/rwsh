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
mod binop;
mod command;
mod if_construct;
mod match_construct;
mod not;
mod pipeline;
mod sresequence;
mod switch_construct;
mod tasklist;
mod while_construct;
mod word;
pub use binop::BinOp;
pub use command::Command;
pub use if_construct::{ElseConstruct, IfConstruct};
pub use match_construct::MatchConstruct;
pub use not::Not;
pub use pipeline::Pipeline;
pub use sresequence::SRESequence;
pub use switch_construct::SwitchConstruct;
pub use tasklist::TaskList;
pub use while_construct::WhileConstruct;
pub use word::Word;

use crate::parser;
use crate::shell::Context;
use nix::sys::wait;
use std::error::Error;
use std::ops::Deref;

#[derive(Clone, Copy, Debug)]
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
            match wait::waitpid(None, None) {
                Err(e) => {
                    if let Some(nix::errno::Errno::ECHILD) = e.as_errno() {
                        ctx.state.last_status = 0;
                        //return Ok(0);
                        continue;
                    } else {
                        return Err(Box::new(e));
                    }
                }
                Ok(stat) => ctx.state.update_process(stat.pid().unwrap(), stat),
            }
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

    pub fn new_from_word(word: parser::Word, expand_tilde: bool, is_pattern: bool) -> Self {
        if let parser::RawWord::List(children, double_quoted) = word.borrow().deref() {
            let mut tl = TaskList::new(false);
            for child in children {
                tl.children.push(Self::new_from_word(
                    child.clone(),
                    !double_quoted && expand_tilde,
                    false,
                ));
            }
            return Task::new(Box::new(tl));
        } else if let parser::RawWord::Pattern(children) = word.borrow().deref() {
            let mut tl = TaskList::new(false);
            for child in children {
                tl.children
                    .push(Self::new_from_word(child.clone(), expand_tilde, true));
            }
            return Task::new(Box::new(tl));
        }

        let tw = Word::new(word, expand_tilde, is_pattern);
        Task::new(Box::new(tw))
    }

    pub fn new_from_simple_command(c: parser::SimpleCommand) -> Self {
        let mut tl = TaskList::new(false);
        let sc = c.with_deep_copied_word();
        tl.children
            .push(Self::new_from_word(sc.0.clone(), true, false));
        for arg in &sc.1 {
            tl.children
                .push(Self::new_from_word(arg.clone(), true, false));
        }
        tl.children.push(Task::new(Box::new(Command::new(sc))));

        Task::new(Box::new(tl))
    }

    pub fn new_from_sre_sequence(seq: parser::SRESequence, top_level: bool) -> Self {
        let mut tl = TaskList::new(true);
        for sre in &seq.0 {
            tl.children.extend(sre.string_args.iter().map(|arg| {
                if let parser::RawWord::Parameter(_) = arg.borrow().deref() {
                    Self::new_from_word(arg.clone(), false, true)
                } else {
                    Self::new_from_word(arg.clone(), false, false)
                }
            }));
            if !sre.command_args.is_empty() {
                tl.children.push(Self::new_from_sre_sequence(
                    parser::SRESequence(sre.command_args.clone()),
                    false,
                ));
            }
        }
        if top_level {
            tl.children.push(Task::new(Box::new(SRESequence::new(seq))));
        }

        Task::new(Box::new(tl))
    }

    pub fn new_from_if(condition: parser::Program, body: parser::Program) -> Self {
        Task::new(Box::new(IfConstruct::new(
            Self::new_from_command_lists(condition.0, true),
            Self::new_from_command_lists(body.0, true),
        )))
    }

    pub fn new_from_else(body: parser::Program) -> Self {
        Task::new(Box::new(ElseConstruct::new(Self::new_from_command_lists(
            body.0, true,
        ))))
    }

    pub fn new_from_while(condition: parser::Program, body: parser::Program) -> Self {
        Task::new(Box::new(WhileConstruct::new(condition, body)))
    }

    pub fn new_from_switch(
        to_match: parser::Word,
        items: Vec<(parser::Word, parser::Program)>,
    ) -> Self {
        let mut tl = TaskList::new(true);
        tl.children
            .push(Self::new_from_word(to_match.clone(), false, true));
        for item in &items {
            tl.children
                .push(Self::new_from_word(item.0.clone(), false, true));
        }
        tl.children
            .push(Task::new(Box::new(SwitchConstruct::new(to_match, items))));

        Task::new(Box::new(tl))
    }

    pub fn new_from_match(items: Vec<(parser::Word, parser::Program)>) -> Self {
        let mut tl = TaskList::new(true);
        for item in &items {
            tl.children
                .push(Self::new_from_word(item.0.clone(), false, true));
        }
        tl.children
            .push(Task::new(Box::new(MatchConstruct::new(items))));
        Task::new(Box::new(tl))
    }

    pub fn new_from_not(prog: parser::Program) -> Self {
        Task::new(Box::new(Not::new(Self::new_from_command_lists(
            prog.0, false,
        ))))
    }

    pub fn new_from_command(pi: parser::Command) -> Self {
        match pi {
            parser::Command::SimpleCommand(c) => Self::new_from_simple_command(c),
            parser::Command::SREProgram(seq) => Self::new_from_sre_sequence(seq, true),
            parser::Command::BraceGroup(arr) => Self::new_from_command_lists(arr, true),
            parser::Command::IfConstruct(condition, body) => Self::new_from_if(condition, body),
            parser::Command::ElseConstruct(body) => Self::new_from_else(body),
            parser::Command::WhileConstruct(condition, body) => {
                Self::new_from_while(condition, body)
            }
            parser::Command::SwitchConstruct(to_match, items) => {
                Self::new_from_switch(to_match, items)
            }
            parser::Command::MatchConstruct(items) => Self::new_from_match(items),
            parser::Command::NotConstruct(prog) => Self::new_from_not(prog),
        }
    }

    pub fn new_from_pipeline(p: parser::Pipeline) -> Self {
        if p.0.len() == 1 {
            return Self::new_from_command(p.0[0].clone());
        }
        let mut tp = Pipeline::new();

        for pi in p.0 {
            tp.children.push(Self::new_from_command(pi));
        }

        Self::new(Box::new(tp))
    }

    pub fn new_from_binop(typ: parser::BinOpType, left: parser::Node, right: parser::Node) -> Self {
        Self::new(Box::new(BinOp::new(
            typ,
            Self::new_from_node(left),
            Self::new_from_node(right),
        )))
    }

    pub fn new_from_node(n: parser::Node) -> Self {
        match n {
            parser::Node::Pipeline(p) => Self::new_from_pipeline(p),
            parser::Node::BinOp(typ, left, right) => Self::new_from_binop(typ, *left, *right),
        }
    }

    pub fn new_from_command_lists(v: Vec<parser::CommandList>, has_scope: bool) -> Self {
        let mut tl = TaskList::new(has_scope);

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
