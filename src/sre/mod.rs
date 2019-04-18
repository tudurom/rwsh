pub mod commands;

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

pub trait Command<'a> {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn open_buffer() {
        let b = super::Buffer::new("xd lol".as_bytes()).unwrap();
        assert_eq!(b.data, "xd lol".chars().collect::<Vec<char>>());
    }
}
