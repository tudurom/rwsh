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
use regex::RegexSet;
use std::io::{stdin, BufReader, Stdin};

pub struct Match {
    ast: Vec<(parser::Word, parser::Program)>,
    items: Vec<Task>,
    patterns: Vec<String>,
    reader: Option<BufReader<Stdin>>,
    regex_set: Option<RegexSet>,

    initialized: bool,
}

impl Match {
    pub fn new(items: Vec<(parser::Word, parser::Program)>) -> Self {
        Match {
            ast: items,
            items: Vec::new(),
            patterns: Vec::new(),
            initialized: false,
            reader: None,
            regex_set: None,
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        for i in &self.ast {
            /*
            self.items.push(SwitchItem {
                pattern: word_to_str(i.0.clone()),
                body: Task::new_from_command_lists((i.1).0.clone()),
            });
            */
            self.items
                .push(Task::new_from_command_lists((i.1).0.clone()));
        }
        self.items.extend(
            self.ast
                .iter()
                .map(|(_, prog)| Task::new_from_command_lists(prog.0.clone())),
        );
        self.patterns
            .extend(self.ast.iter().map(|(p, _)| word_to_str(p.clone())));
        self.regex_set =
            Some(RegexSet::new(self.patterns.iter()).map_err(|e| format!("regex error: {}", e))?);
        self.reader = Some(BufReader::new(stdin()));
        self.initialized = true;
        Ok(())
    }
}

impl TaskImpl for Match {
    fn poll(&mut self, _ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize()?;
        }
        unimplemented!()
    }
}
