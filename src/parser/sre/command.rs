use crate::sre::Command;
use crate::util::{BufReadChars, LineReader, ParseError};
use std::iter::Peekable;

pub fn parse_command<R: LineReader>(it: &mut BufReadChars<R>) -> Result<Box<Command>, ParseError> {
    Ok(Box::new(crate::sre::commands::P))
}
