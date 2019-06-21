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
//! Parsing and lexing functions for the SRE sublanguage.
pub mod address;
pub mod command;

use super::Parser;
use crate::shell::pretty::*;
use crate::util::ParseError;
use address::ComposedAddress;

#[derive(Debug, Clone)]
/// A full SRE command, with an address, list of arguments and optional command argument.
/// The command argument is for commands that act as conditionals or loops, such as `x` or `g`.
pub struct Command {
    pub address: ComposedAddress,
    pub name: char,
    pub string_args: Vec<String>,
    pub command_args: Vec<Command>,
    pub original_address: String,
}

impl PrettyPrint for Command {
    fn pretty_print(&self) -> PrettyTree {
        let mut string_args = self
            .string_args
            .iter()
            .map(|s| PrettyTree {
                text: format!("string arg: {}", s),
                children: vec![],
            })
            .collect::<Vec<_>>();
        let mut command_args = self
            .command_args
            .iter()
            .map(|c| PrettyTree {
                text: "command arg".to_owned(),
                children: vec![c.pretty_print()],
            })
            .collect::<Vec<_>>();
        let mut children = Vec::new();
        children.append(&mut string_args);
        children.append(&mut command_args);
        PrettyTree {
            text: format!(
                "{}{}",
                self.name,
                if !self.original_address.is_empty() {
                    format!(" ({})", self.original_address)
                } else {
                    "".to_owned()
                }
            ),
            children,
        }
    }
}

impl PartialEq<Command> for Command {
    fn eq(&self, rhs: &Command) -> bool {
        self.address == rhs.address
            && self.name == rhs.name
            && self.string_args == rhs.string_args
            && self.command_args == rhs.command_args
    }
}

impl Command {
    pub fn new(
        address: ComposedAddress,
        name: char,
        string_args: Vec<String>,
        command_args: Vec<Command>,
        original_address: String,
    ) -> Command {
        Command {
            address,
            name,
            string_args,
            command_args,
            original_address,
        }
    }
}

fn skip_whitespace(p: &mut Parser, break_at_newline: bool) {
    crate::parser::skip_whitespace(&mut p.lexer.borrow_mut().input, break_at_newline);
}

/// Parses the address and the simple command, returning a complete, ready-to-use command.
pub fn parse_command(p: &mut Parser, brace: bool) -> Result<Option<Command>, ParseError> {
    skip_whitespace(p, false);
    let (ap, original) = address::Parser::new(&mut p.lexer.borrow_mut().input)?;
    let address = ap.parse().map_err(|e| p.new_error(e))?.unwrap_or_default();
    let simple = match command::parse_command(p, brace)? {
        Some(c) => c,
        None => return Ok(None),
    };
    Ok(Some(Command {
        address,
        name: simple.name,
        string_args: simple.args,
        command_args: simple.command_args,
        original_address: original,
    }))
}

#[cfg(test)]
mod tests {
    use crate::parser::sre::address::{ComposedAddress, SimpleAddress};
    use crate::parser::Parser;
    use crate::tests::common::new_dummy_buf;
    #[test]
    fn smoke() {
        let mut p = Parser::new(new_dummy_buf(
            "/something /a/else/ ,p ,x/Emacs/ /{TM}/d".lines(),
        ));
        assert_eq!(
            super::parse_command(&mut p, false).unwrap().unwrap(),
            super::Command {
                address: ComposedAddress::new(
                    SimpleAddress::Regex("/something ".to_owned(), false),
                    None,
                    None
                ),
                name: 'a',
                string_args: vec!["else".to_owned()],
                command_args: vec![],
                original_address: String::new(),
            }
        );
        assert_eq!(
            super::parse_command(&mut p, false).unwrap().unwrap(),
            super::Command {
                address: ComposedAddress::new(
                    SimpleAddress::Comma,
                    Some(Box::new(ComposedAddress::new(
                        SimpleAddress::Line(0),
                        None,
                        None
                    ))),
                    Some(Box::new(ComposedAddress::new(
                        SimpleAddress::Dollar,
                        None,
                        None
                    )))
                ),
                name: 'p',
                string_args: vec![],
                command_args: vec![],
                original_address: String::new(),
            }
        );
        assert_eq!(
            super::parse_command(&mut p, false).unwrap().unwrap(),
            super::Command {
                address: ComposedAddress::new(
                    SimpleAddress::Comma,
                    Some(Box::new(ComposedAddress::new(
                        SimpleAddress::Line(0),
                        None,
                        None
                    ))),
                    Some(Box::new(ComposedAddress::new(
                        SimpleAddress::Dollar,
                        None,
                        None
                    )))
                ),
                name: 'x',
                string_args: vec!["Emacs".to_owned()],
                command_args: vec![super::Command {
                    address: ComposedAddress::new(
                        SimpleAddress::Regex("/{TM}".to_owned(), false),
                        None,
                        None,
                    ),
                    name: 'd',
                    string_args: vec![],
                    command_args: vec![],
                    original_address: String::new(),
                }],
                original_address: String::new(),
            }
        );
    }
}
