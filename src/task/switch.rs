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
use crate::shell::Context;
use regex::RegexSet;

enum ItemIndex {
    Unknown,
    Index(usize),
    None,
}

impl ItemIndex {
    fn index(&self) -> usize {
        if let ItemIndex::Index(i) = self {
            *i
        } else {
            panic!()
        }
    }
}

pub struct Switch {
    ast: (parser::Word, Vec<(parser::Word, parser::Program)>),
    to_match: String,
    items: Vec<Task>,
    patterns: Vec<String>,
    regex_set: Option<RegexSet>,

    initialized: bool,
    index: ItemIndex,
}

impl Switch {
    pub fn new(condition: parser::Word, items: Vec<(parser::Word, parser::Program)>) -> Self {
        Switch {
            ast: (condition, items),
            to_match: String::new(),
            items: Vec::new(),
            patterns: Vec::new(),
            initialized: false,
            index: ItemIndex::Unknown,
            regex_set: None,
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.to_match = word_to_str(self.ast.0.clone());
        self.items.extend(
            self.ast
                .1
                .iter()
                .map(|(_, prog)| Task::new_from_command_lists(prog.0.clone())),
        );
        self.patterns
            .extend(self.ast.1.iter().map(|(p, _)| word_to_str(p.clone())));
        self.regex_set =
            Some(RegexSet::new(self.patterns.iter()).map_err(|e| format!("regex error: {}", e))?);
        self.initialized = true;
        Ok(())
    }
}

impl TaskImpl for Switch {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize()?;
        }
        if let ItemIndex::Unknown = self.index {
            let matches = self.regex_set.as_ref().unwrap().matches(&self.to_match);
            match matches.into_iter().next() {
                None => self.index = ItemIndex::None,
                Some(i) => self.index = ItemIndex::Index(i),
            }
        }
        if let ItemIndex::None = self.index {
            Ok(TaskStatus::Success(0))
        } else {
            self.items[self.index.index()].poll(ctx)
        }
    }
}
