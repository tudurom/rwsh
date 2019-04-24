//! Operations and functionality for [structural regular expressions](http://doc.cat-v.org/bell_labs/structural_regexps/).
//! 
//! We will use the "SRE" abbreviation from now on.
pub mod commands;

use crate::parser::sre::address::ComposedAddress;
use std::collections::LinkedList;
use std::io::{self, Read, Write};

#[derive(Debug)]
/// The buffer holds the text that we are operating on.
///
/// For the moment, it keeps the entire output of the piped command.
pub struct Buffer {
    data: Vec<char>,
}

impl Buffer {
    pub fn new(mut r: impl Read) -> io::Result<Buffer> {
        let mut s = String::new();
        let mut b = Buffer { data: vec![] };
        r.read_to_string(&mut s)?;
        for c in s.chars() {
            b.data.push(c);
        }
        Ok(b)
    }

    /// Returns a new address in this buffer.
    pub fn new_address(&self, l: usize, r: usize) -> Address {
        Address {
            r: Range(l, r),
            buffer: self,
        }
    }
}

#[derive(Copy, Clone)]
/// A simple range. The left part is inclusive, the right part is exclusive.
///
/// An example would be `(3, 10) -> (3, 4, 5, 6, 7, 8, 9)`
pub struct Range(usize, usize);

#[derive(Copy, Clone)]
/// An address is a chunk of a (struct.Buffer.html).
pub struct Address<'a> {
    r: Range,
    buffer: &'a Buffer,
}

/// Defines an interface for text manipulation routines.
pub trait SimpleCommand<'a>: std::fmt::Debug {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>>;
    fn to_tuple(&self) -> (char, LinkedList<String>);
}

#[derive(Debug)]
/// A SRE command that can be applied on a buffer.
pub struct Command<'a> {
    address: ComposedAddress,
    simple: Box<dyn SimpleCommand<'a>>,
}

impl<'a> Command<'a> {
    pub fn new(address: ComposedAddress, simple: Box<dyn SimpleCommand<'a>>) -> Self {
        Command { address, simple }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn open_buffer() {
        let b = super::Buffer::new("xd lol".as_bytes()).unwrap();
        assert_eq!(b.data, "xd lol".chars().collect::<Vec<char>>());
    }
}
