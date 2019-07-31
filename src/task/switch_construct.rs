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
use crate::shell::{Context, Key, Var, VarValue};
use regex::{Regex, RegexSet};
use std::collections::HashMap;

struct ExecContext {
    index: usize,
    int_captures: Vec<String>,
    string_captures: HashMap<String, String>,
    started: bool,
    finished: bool,
}

enum ItemIndex {
    Unknown,
    Index(ExecContext),
    None,
}

impl ItemIndex {
    fn index(&mut self) -> &mut ExecContext {
        if let ItemIndex::Index(ref mut ctx) = self {
            ctx
        } else {
            panic!()
        }
    }
}

pub struct SwitchConstruct {
    ast: (parser::Word, Vec<(parser::Word, parser::Program)>),
    to_match: String,
    items: Vec<Task>,
    patterns: Vec<String>,
    regex_set: Option<RegexSet>,
    regexes: Vec<Regex>,
    named_capture_groups: Vec<Vec<String>>,

    initialized: bool,
    index: ItemIndex,
}

impl SwitchConstruct {
    pub fn new(condition: parser::Word, items: Vec<(parser::Word, parser::Program)>) -> Self {
        SwitchConstruct {
            ast: (condition, items),
            to_match: String::new(),
            items: Vec::new(),
            patterns: Vec::new(),
            initialized: false,
            index: ItemIndex::Unknown,
            regex_set: None,
            regexes: Vec::new(),
            named_capture_groups: Vec::new(),
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.to_match = word_to_str(self.ast.0.clone());

        let len = self.ast.1.len();
        self.items.reserve(len);
        self.patterns.reserve(len);
        self.regexes.reserve(len);
        self.named_capture_groups.reserve(len);

        self.items.extend(
            self.ast
                .1
                .iter()
                .map(|(_, prog)| Task::new_from_command_lists(prog.0.clone(), false)),
        );
        self.patterns
            .extend(self.ast.1.iter().map(|(p, _)| word_to_str(p.clone())));
        self.regex_set =
            Some(RegexSet::new(self.patterns.iter()).map_err(|e| format!("regex error: {}", e))?);
        self.regexes
            .extend(self.patterns.iter().map(|p| Regex::new(p).unwrap()));
        self.named_capture_groups
            .extend(self.regexes.iter().map(|re| {
                re.capture_names()
                    .filter(|v| v.is_some())
                    .map(|v| v.unwrap().to_owned())
                    .collect()
            }));
        self.initialized = true;
        Ok(())
    }
}

impl TaskImpl for SwitchConstruct {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize()?;
        }
        if let ItemIndex::Unknown = self.index {
            let matches = self.regex_set.as_ref().unwrap().matches(&self.to_match);
            match matches.into_iter().next() {
                None => self.index = ItemIndex::None,
                Some(i) => {
                    let cap = self.regexes[i].captures(&self.to_match).unwrap();
                    self.index = ItemIndex::Index(ExecContext {
                        index: i,
                        int_captures: cap
                            .iter()
                            .map(|x| x.map_or(String::new(), |val| val.as_str().to_owned()))
                            .collect(),
                        string_captures: self.named_capture_groups[i]
                            .iter()
                            .map(|name| {
                                (
                                    name.clone(),
                                    cap.name(name)
                                        .map_or(String::new(), |val| val.as_str().to_owned()),
                                )
                            })
                            .collect(),
                        started: false,
                        finished: false,
                    });
                }
            }
        }
        if let ItemIndex::None = self.index {
            Ok(TaskStatus::Success(0))
        } else {
            let cur = self.index.index();
            if !cur.started {
                for (i, val) in cur.int_captures.iter().enumerate() {
                    ctx.state.set_var(
                        Key::Var(&i.to_string()),
                        Var::new(i.to_string(), VarValue::Array(vec![val.clone()])),
                        true,
                    );
                }
                for (name, val) in cur.string_captures.iter() {
                    ctx.state.set_var(
                        Key::Var(name),
                        Var::new(name.clone(), VarValue::Array(vec![val.clone()])),
                        true,
                    );
                }
                cur.started = true;
            }
            let body_status = self.items[cur.index].poll(ctx)?;
            if let TaskStatus::Wait = body_status {
                Ok(TaskStatus::Wait)
            } else {
                if !cur.finished {
                    for i in 0..cur.int_captures.len() {
                        ctx.state.remove_var(&i.to_string());
                    }
                    for name in cur.string_captures.keys() {
                        ctx.state.remove_var(name);
                    }
                    cur.finished = true;
                }
                Ok(body_status)
            }
        }
    }
}
