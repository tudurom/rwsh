use crate::util::{BufReadChars, LineReader};
use std::io;
use std::str::Lines;

pub struct DummyLineReader<'a>(pub Lines<'a>);

impl<'a> LineReader for DummyLineReader<'a> {
    fn read_line(&mut self) -> io::Result<Option<String>> {
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
