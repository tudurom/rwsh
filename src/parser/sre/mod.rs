//! Parsing and lexing functions for the SRE sublanguage.
pub mod address;
pub mod command;

use super::skip_whitespace;
use crate::shell::pretty::*;
use crate::util::{BufReadChars, LineReader, ParseError};
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

/// Parses the address and the simple command, returning a complete, ready-to-use command.
pub fn parse_command<R: LineReader>(
    it: &mut BufReadChars<R>,
    brace: bool,
) -> Result<Option<Command>, ParseError> {
    skip_whitespace(it, true);
    let (p, original) = address::Parser::new(it)?;
    let address = match p.parse() {
        Ok(x) => x,
        Err(e) => return Err(it.new_error(e)),
    }
    .unwrap_or_default();
    let simple = match command::parse_command(it, brace)? {
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
    use crate::tests::common::new_dummy_buf;
    #[test]
    fn smoke() {
        let mut buf = new_dummy_buf("/something /a/else/ ,p ,x/Emacs/ /{TM}/d".lines());
        assert_eq!(
            super::parse_command(&mut buf, false).unwrap().unwrap(),
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
            super::parse_command(&mut buf, false).unwrap().unwrap(),
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
            super::parse_command(&mut buf, false).unwrap().unwrap(),
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
