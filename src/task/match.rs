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
use regex::Regex;
use std::io::{stdin, BufRead, BufReader, ErrorKind, Stdin};

struct MatchItem {
    regex: Regex,
    offset: usize,
    task: Task,
    to_exec: usize,
    prog: parser::Program,
}

pub struct Match {
    ast: Vec<(parser::Word, parser::Program)>,
    reader: Option<BufReader<Stdin>>,
    items: Vec<MatchItem>,

    initialized: bool,
    finished: bool,
    buf: Vec<u8>,
    last_body_status: Result<TaskStatus, String>,
}

impl Match {
    pub fn new(items: Vec<(parser::Word, parser::Program)>) -> Self {
        Match {
            ast: items,
            items: Vec::new(),
            reader: None,

            initialized: false,
            finished: false,
            buf: Vec::new(),
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
                MatchItem {
                    regex,
                    offset: 0,
                    task,
                    to_exec: 0,
                    prog: self.ast[i].1.clone(),
                }
            })
            .collect();
        self.reader = Some(BufReader::new(stdin()));
        self.initialized = true;
        Ok(())
    }
}

impl TaskImpl for Match {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize()?;
        }
        let stdin = stdin();
        while ctx.state.exit == -1 {
            match self.items.iter_mut().find(|item| item.to_exec > 0) {
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
                    self.buf.extend_from_slice(available);
                    for item in self.items.iter_mut() {
                        let s = String::from_utf8_lossy(&self.buf[item.offset..]);
                        let mut to_add = 0;
                        for m in item.regex.find_iter(&s) {
                            item.to_exec += 1;
                            to_add = m.end();
                        }
                        item.offset += to_add;
                    }
                    handle.consume(len);
                    return Ok(TaskStatus::Wait);
                }
                Some(item) => {
                    let body_status = item.task.poll(ctx)?;
                    if let TaskStatus::Wait = body_status {
                        return Ok(TaskStatus::Wait);
                    } else {
                        self.last_body_status = Ok(body_status);
                        item.task = Task::new_from_command_lists(item.prog.0.clone(), false);
                        item.to_exec -= 1;
                    }
                }
            }
        }

        Ok(TaskStatus::Success(ctx.state.exit))
    }
}
