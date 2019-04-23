pub mod commands;

use crate::parser::sre::address::ComposedAddress;
use std::collections::LinkedList;
use std::io::{self, Read, Write};

#[derive(Debug)]
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

    pub fn new_address(&self, l: usize, r: usize) -> Address {
        Address {
            r: Range(l, r),
            buffer: self,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Range(usize, usize);

#[derive(Copy, Clone)]
pub struct Address<'a> {
    r: Range,
    buffer: &'a Buffer,
}

pub trait SimpleCommand<'a>: std::fmt::Debug {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>>;
    fn to_tuple(&self) -> (char, LinkedList<String>);
}

#[derive(Debug)]
pub struct Command<'a> {
    address: ComposedAddress,
    simple: Box<dyn SimpleCommand<'a>>,
}

impl<'a> PartialEq for Command<'a> {
    fn eq(&self, other: &Command) -> bool {
        self.address == other.address && self.simple.to_tuple() == other.simple.to_tuple()
    }
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
