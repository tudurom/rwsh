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
    Parser::new(&mut buf).unwrap().0.parse().unwrap().unwrap()
}

pub fn new_buffer(text: &'static str) -> Buffer {
    Buffer::new(text.as_bytes()).unwrap()
}
