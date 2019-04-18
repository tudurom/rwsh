use crate::sre::Command;
use std::iter::Peekable;

pub fn parse_command<C: Iterator<Item = char>>(
    it: &mut Peekable<C>,
) -> Result<Box<Command>, String> {
    Ok(Box::new(crate::sre::commands::P))
}
