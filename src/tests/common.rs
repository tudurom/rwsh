//! Convenient functions and types for tests.
use crate::parser::sre::address::{ComposedAddress, Parser};
use crate::sre::Buffer;
use crate::util::{BufReadChars, LineReader};
use std::error::Error;
use std::str::Lines;

#[derive(Clone)]
pub struct DummyLineReader<'a>(pub Lines<'a>);

impl<'a> LineReader for DummyLineReader<'a> {
    fn read_line(&mut self) -> Result<Option<String>, Box<Error>> {
        match self.0.next() {
            Some(s) => {
                let mut s = String::from(s);
                s.push('\n');
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }
}

pub fn new_dummy_buf(l: Lines) -> BufReadChars<DummyLineReader> {
    BufReadChars::new(DummyLineReader(l))
}

pub fn new_composed_address(addr: &'static str) -> ComposedAddress {
    let mut buf = new_dummy_buf(addr.lines());
    Parser::new(&mut buf).unwrap().parse().unwrap().unwrap()
}

pub fn new_buffer(text: &'static str) -> Buffer {
    Buffer::new(text.as_bytes()).unwrap()
}
