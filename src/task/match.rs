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

enum MatchState {
    Searching,
    Executing(usize),
    Finished,
}

pub struct Match {
    ast: Vec<(parser::Word, parser::Program)>,
    items: Vec<Task>,
    patterns: Vec<String>,
    reader: Option<BufReader<Stdin>>,
    regexes: Vec<Regex>,

    initialized: bool,
    state: MatchState,
    buf: Vec<u8>,
    last_body_status: Result<TaskStatus, String>,
}

impl Match {
    pub fn new(items: Vec<(parser::Word, parser::Program)>) -> Self {
        Match {
            ast: items,
            items: Vec::new(),
            patterns: Vec::new(),
            reader: None,
            regexes: Vec::new(),

            initialized: false,
            state: MatchState::Searching,
            buf: Vec::new(),
            last_body_status: Ok(TaskStatus::Wait),
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.items.extend(
            self.ast
                .iter()
                .map(|(_, prog)| Task::new_from_command_lists(prog.0.clone())),
        );
        self.patterns
            .extend(self.ast.iter().map(|(p, _)| word_to_str(p.clone())));
        self.regexes
            .extend(self.patterns.iter().map(|p| Regex::new(p).unwrap()));
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
        while ctx.state.exit == -1 {
            if let MatchState::Searching = self.state {
                let stdin = stdin();
                let mut handle = stdin.lock();
                loop {
                    let available = match handle.fill_buf() {
                        Ok(n) => n,
                        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) => return Err(format!("{}", e)),
                    };
                    if available.is_empty() {
                        self.state = MatchState::Finished;
                        break;
                    }

                    let init_len = self.buf.len();
                    self.buf.extend_from_slice(available);
                    let s = String::from_utf8_lossy(self.buf.as_slice());
                    let mut found = false;
                    let mut used = available.len();
                    for (i, r) in self.regexes.iter().enumerate() {
                        if let Some(m) = r.find(&s) {
                            self.state = MatchState::Executing(i);
                            found = true;
                            used = m.end() - init_len;
                            break;
                        }
                    }
                    handle.consume(used);
                    if found {
                        self.buf.clear();
                        break;
                    }
                }
            }

            match &self.state {
                MatchState::Executing(i) => {
                    let body_status = self.items[*i].poll(ctx)?;
                    if let TaskStatus::Wait = body_status {
                        return Ok(TaskStatus::Wait)
                    } else {
                        self.last_body_status = Ok(body_status);
                        self.items[*i] = Task::new_from_command_lists(self.ast[*i].1.clone().0);
                        self.state = MatchState::Searching;
                    }
                }
                MatchState::Finished => return self.last_body_status.clone(),
                _ => {}
            }
        }
        Ok(TaskStatus::Success(ctx.state.exit))
    }
}
