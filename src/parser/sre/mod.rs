//! Parsing and lexing functions for the SRE sublanguage.
pub mod address;
pub mod command;

use super::skip_whitespace;
use crate::util::{BufReadChars, LineReader, ParseError};
use address::ComposedAddress;

#[derive(Debug, PartialEq, Clone)]
/// A full SRE command, with an address, list of arguments and optional command argument.
/// The command argument is for commands that act as conditionals or loops, such as `x` or `g`.
pub struct Command {
    pub address: ComposedAddress,
    pub name: char,
    pub string_args: Vec<String>,
    pub command_arg: Option<Box<Command>>,
}

impl Command {
    pub fn new(
        address: ComposedAddress,
        name: char,
        string_args: Vec<String>,
        command_arg: Option<Box<Command>>,
    ) -> Command {
        Command {
            address,
            name,
            string_args,
            command_arg,
        }
    }
}

/// Parses the address and the simple command, returning a complete, ready-to-use command.
pub fn parse_command<R: LineReader>(it: &mut BufReadChars<R>) -> Result<Command, ParseError> {
    skip_whitespace(it);
    let address = match address::Parser::new(it)?.parse() {
        Ok(x) => x,
        Err(e) => return Err(it.new_error(e)),
    }
    .unwrap_or_default();
    let simple = command::parse_command(it)?;
    Ok(Command {
        address,
        name: simple.name,
        string_args: simple.args,
        command_arg: simple.command_arg,
    })
}

#[cfg(test)]
mod tests {
    use crate::parser::sre::address::{ComposedAddress, SimpleAddress};
    use crate::tests::common::new_dummy_buf;
    #[test]
    fn smoke() {
        let mut buf = new_dummy_buf("/something /a/else/ ,p ,x/Emacs/ /{TM}/d".lines());
        assert_eq!(
            super::parse_command(&mut buf).unwrap(),
            super::Command {
                address: ComposedAddress::new(
                    SimpleAddress::Regex("/something ".to_owned(), false),
                    None,
                    None
                ),
                name: 'a',
                string_args: vec!["else".to_owned()],
                command_arg: None,
            }
        );
        assert_eq!(
            super::parse_command(&mut buf).unwrap(),
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
                command_arg: None,
            }
        );
        assert_eq!(
            super::parse_command(&mut buf).unwrap(),
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
                command_arg: Some(Box::new(super::Command {
                    address: ComposedAddress::new(
                        SimpleAddress::Regex("/{TM}".to_owned(), false),
                        None,
                        None
                    ),
                    name: 'd',
                    string_args: vec![],
                    command_arg: None
                }))
            }
        );
    }
}