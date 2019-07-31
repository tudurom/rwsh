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
use super::word::word_to_str;
use super::*;
use crate::parser;
use crate::shell::{Key, Var, VarValue};
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::io::{stdin, BufRead, BufReader, ErrorKind, Stdin};

struct ExecContext {
    int_captures: Vec<String>,
    string_captures: HashMap<String, String>,
}

struct MatchItem {
    regex: Regex,
    offset: usize,
    task: Task,
    to_exec: VecDeque<ExecContext>,
    prog: parser::Program,
    named_capture_groups: Vec<String>,
    started: bool,
}

pub struct MatchConstruct {
    ast: Vec<(parser::Word, parser::Program)>,
    reader: Option<BufReader<Stdin>>,
    items: Vec<MatchItem>,

    initialized: bool,
    finished: bool,
    buf: String,
    last_body_status: Result<TaskStatus, String>,
}

impl MatchConstruct {
    pub fn new(items: Vec<(parser::Word, parser::Program)>) -> Self {
        MatchConstruct {
            ast: items,
            items: Vec::new(),
            reader: None,

            initialized: false,
            finished: false,
            buf: String::new(),
            last_body_status: Ok(TaskStatus::Wait),
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.items = self
            .ast
            .iter()
            .enumerate()
            .map(|(i, (pattern, prog))| {
                let pattern = word_to_str(pattern.clone());
                let task = Task::new_from_command_lists(prog.0.clone(), false);
                let regex = Regex::new(&pattern).unwrap();
                let named_capture_groups = regex
                    .capture_names()
                    .filter(|v| v.is_some())
                    .map(|v| v.unwrap().to_owned())
                    .collect();
                MatchItem {
                    regex,
                    offset: 0,
                    task,
                    to_exec: VecDeque::new(),
                    prog: self.ast[i].1.clone(),
                    named_capture_groups,
                    started: false,
                }
            })
            .collect();
        self.reader = Some(BufReader::new(stdin()));
        self.initialized = true;
        Ok(())
    }
}

impl TaskImpl for MatchConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize()?;
        }
        let stdin = stdin();
        while ctx.state.exit == -1 {
            match self.items.iter_mut().find(|item| !item.to_exec.is_empty()) {
                None => {
                    if self.finished {
                        return self.last_body_status.clone();
                    }
                    let mut handle = stdin.lock();
                    let available = match handle.fill_buf() {
                        Ok(n) => n,
                        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Err(format!("{}", e)),
                    };
                    if available.is_empty() {
                        self.finished = true;
                        continue;
                    }
                    let len = available.len();
                    self.buf.push_str(&String::from_utf8_lossy(available));
                    for item in self.items.iter_mut() {
                        let s = &self.buf[item.offset..];
                        let mut to_add = 0;
                        for m in item.regex.captures_iter(&s) {
                            item.to_exec.push_back(ExecContext {
                                int_captures: m
                                    .iter()
                                    .map(|x| x.map_or(String::new(), |val| val.as_str().to_owned()))
                                    .collect(),
                                string_captures: item
                                    .named_capture_groups
                                    .iter()
                                    .map(|name| {
                                        (
                                            name.clone(),
                                            m.name(name).map_or(String::new(), |val| {
                                                val.as_str().to_owned()
                                            }),
                                        )
                                    })
                                    .collect(),
                            });
                            to_add = m.get(0).unwrap().end();
                        }
                        item.offset += to_add;
                    }
                    handle.consume(len);
                    return Ok(TaskStatus::Wait);
                }
                Some(item) => {
                    if !item.started {
                        for (i, val) in item
                            .to_exec
                            .front()
                            .as_ref()
                            .unwrap()
                            .int_captures
                            .iter()
                            .enumerate()
                        {
                            ctx.state.set_var(
                                Key::Var(&i.to_string()),
                                Var::new(i.to_string(), VarValue::Array(vec![val.clone()])),
                                true,
                            );
                        }
                        for (name, val) in item
                            .to_exec
                            .front()
                            .as_ref()
                            .unwrap()
                            .string_captures
                            .iter()
                        {
                            ctx.state.set_var(
                                Key::Var(name),
                                Var::new(name.clone(), VarValue::Array(vec![val.clone()])),
                                true,
                            );
                        }
                        item.started = true;
                    }
                    let body_status = item.task.poll(ctx)?;
                    if let TaskStatus::Wait = body_status {
                        return Ok(TaskStatus::Wait);
                    } else {
                        self.last_body_status = Ok(body_status);
                        item.task = Task::new_from_command_lists(item.prog.0.clone(), false);
                        for i in 0..item.to_exec.front().as_ref().unwrap().int_captures.len() {
                            ctx.state.remove_var(&i.to_string());
                        }
                        for name in item
                            .to_exec
                            .front()
                            .as_ref()
                            .unwrap()
                            .string_captures
                            .keys()
                        {
                            ctx.state.remove_var(name);
                        }
                        item.started = false;
                        item.to_exec.pop_front();
                    }
                }
            }
        }

        Ok(TaskStatus::Success(ctx.state.exit))
    }
}
