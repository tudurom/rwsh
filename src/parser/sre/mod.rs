pub mod address;
pub mod command;

use super::skip_whitespace;
use crate::sre::Command;
use crate::util::{BufReadChars, LineReader, ParseError};

pub fn parse_command<'a, R: LineReader>(it: &mut BufReadChars<R>) -> Result<Command, ParseError> {
    skip_whitespace(it);
    let address = match address::Parser::new(it)?.parse() {
        Ok(x) => x,
        Err(e) => return Err(it.new_error(e)),
    }
    .unwrap_or_default();
    let simple = command::parse_simple_command(it)?;
    Ok(Command::new(address, simple))
}

#[cfg(test)]
mod tests {
    use crate::tests::common::new_dummy_buf;
    #[test]
    fn smoke() {
        let mut buf = new_dummy_buf("/something /a/else/ ,p".lines());
        // HACK: this is the easiest method i found to compare the damn trait objects
        assert_eq!(
            format!("{:#?}", super::parse_command(&mut buf)),
            "Ok(
    Command {
        address: ComposedAddress {
            simple: Regex(
                \"/something \",
                false
            ),
            left: None,
            next: None
        },
        simple: A(
            \"else\"
        )
    }
)"
        );
        assert_eq!(
            format!("{:#?}", super::parse_command(&mut buf)),
            "Ok(
    Command {
        address: ComposedAddress {
            simple: Comma,
            left: Some(
                ComposedAddress {
                    simple: Line(
                        0
                    ),
                    left: None,
                    next: None
                }
            ),
            next: Some(
                ComposedAddress {
                    simple: Dollar,
                    left: None,
                    next: None
                }
            )
        },
        simple: P
    }
)"
        );
    }
}
