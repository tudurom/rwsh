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
use super::*;
use crate::parser;
use crate::shell::Context;
use super::word::word_to_str;
use regex::Regex;

struct SwitchItem {
    pattern: String,
    body: Task,
}

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
    items: Vec<SwitchItem>,

    initialized: bool,
    index: ItemIndex,
}

impl Switch {
    pub fn new(condition: parser::Word, items: Vec<(parser::Word, parser::Program)>) -> Self {
        Switch {
            ast: (condition, items),
            to_match: String::new(),
            items: Vec::new(),
            initialized: false,
            index: ItemIndex::Unknown,
        }
    }

    fn initialize(&mut self) {
        self.to_match = word_to_str(self.ast.0.clone());
        for i in &self.ast.1 {
            self.items.push(SwitchItem {
                pattern: word_to_str(i.0.clone()),
                body: Task::new_from_command_lists((i.1).0.clone()),
            });
        }
        self.initialized = true;
    }
}

impl TaskImpl for Switch {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.initialized {
            self.initialize();
        }
        if let ItemIndex::Unknown = self.index {
            for (i, item) in self.items.iter().enumerate() {
                let re = Regex::new(&item.pattern).map_err(|e| format!("regex error: {}", e))?;
                if re.is_match(&self.to_match) {
                    self.index = ItemIndex::Index(i);
                    break;
                }
            }
            if let ItemIndex::Unknown = self.index {
                self.index = ItemIndex::None;
            }
        }
        if let ItemIndex::None = self.index {
            Ok(TaskStatus::Success(0))
        } else {
            self.items[self.index.index()].body.poll(ctx)
        }
    }
}
