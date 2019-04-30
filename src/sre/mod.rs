//! Operations and functionality for [structural regular expressions](http://doc.cat-v.org/bell_labs/structural_regexps/).
//!
//! We will use the "SRE" abbreviation from now on.
pub mod address;
pub mod commands;

use crate::parser::sre::Command as SRECommand;
use address::Address;
use address::AddressResolveError;
use std::collections::BTreeSet;
use std::collections::LinkedList;
use std::error::Error;
use std::io::{self, Read, Write};
use std::str::FromStr;

#[derive(Debug, Eq)]
pub struct Change {
    pos: usize,
    deleted: usize,
    content: String,
}

impl Change {
    pub fn new(pos: usize, deleted: usize, content: &str) -> Change {
        Change {
            pos,
            deleted,
            content: String::from_str(content).unwrap(),
        }
    }
}

impl Ord for Change {
    fn cmp(&self, rhs: &Change) -> std::cmp::Ordering {
        self.pos.cmp(&rhs.pos)
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, rhs: &Change) -> Option<std::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl PartialEq for Change {
    // a bit of a hack, in order to let the BTreeHash tell us
    // if there are any intersecting changes,
    // we will consider intersecting changes equal
    fn eq(&self, rhs: &Change) -> bool {
        use std::cmp::{max, min};
        let left = max(self.pos, rhs.pos);
        let right = min(self.pos + self.deleted - 1, rhs.pos + rhs.deleted - 1);

        left > right
    }
}

#[derive(Debug)]
/// The buffer holds the text that we are operating on.
///
/// For the moment, it keeps the entire output of the piped command.
pub struct Buffer {
    data: String,
    changes: BTreeSet<Change>,
}

impl Buffer {
    pub fn new(mut r: impl Read) -> io::Result<Buffer> {
        let mut s = String::new();
        r.read_to_string(&mut s)?;
        Ok(Buffer {
            data: s,
            changes: BTreeSet::new(),
        })
    }

    /// Returns a new address in this buffer.
    pub fn new_address(&self, l: usize, r: usize) -> Address {
        Address {
            r: Range(l, r),
            buffer: self,
        }
    }

    /// Returns `true` if the new change didn't intersect any
    pub fn change(&mut self, dot: Range, append: bool, content: &str) -> bool {
        if append {
            self.changes.insert(Change {
                pos: dot.1,
                deleted: 0,
                content: String::from_str(content).unwrap(),
            })
        } else {
            self.changes.insert(Change {
                pos: dot.0,
                deleted: dot.1 - dot.0,
                content: String::from_str(content).unwrap(),
            })
        }
    }

    /// Applies the changes on the buffer.
    ///
    /// The changes must not be intersecting.
    pub fn apply_changes(&mut self, mut dot: Range) -> Range {
        let mut new_data = Vec::<u8>::new();
        let mut last_index: usize = 0;
        let original = dot;
        for c in &self.changes {
            if c.pos < original.0 {
                dot.0 = dot.0 + c.content.len() - c.deleted;
                dot.1 = dot.1 + c.content.len() - c.deleted;
            }
            for i in last_index..c.pos {
                new_data.push(self.data.as_bytes()[i]);
            }
            for b in c.content.bytes() {
                new_data.push(b);
            }
            last_index = c.pos + c.deleted;
        }
        for i in last_index..self.data.len() {
            new_data.push(self.data.as_bytes()[i]);
        }
        self.data = String::from_utf8_lossy(&new_data).to_string();
        self.changes.clear();
        dot
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// A simple range. The left part is inclusive, the right part is exclusive.
///
/// An example would be `(3, 10) -> (3, 4, 5, 6, 7, 8, 9)`
pub struct Range(usize, usize);

/// Defines an interface for text manipulation routines.
pub trait SimpleCommand<'a>: std::fmt::Debug {
    fn execute(&self, w: &mut Write, buffer: &mut Buffer, dot: Range) -> Result<Range, Box<Error>>;
    fn to_tuple(&self) -> (char, LinkedList<String>);
}

#[derive(Debug)]
/// A SRE command that can be applied on a buffer.
pub struct Invocation<'a> {
    address: Range,
    simple: Box<dyn SimpleCommand<'a>>,
}

impl<'a> Invocation<'a> {
    pub fn new(
        parsed: SRECommand,
        buf: &Buffer,
        address: Option<Range>,
    ) -> Result<Invocation<'a>, AddressResolveError> {
        let address = match address {
            Some(x) => x,
            None => Address::new(buf).range(),
        };
        let address = Address::from_range(buf, address)
            .address(parsed.address)?
            .range();
        if address.1 > buf.data.len() {
            panic!(
                "bad range ({}, {}) out of buffer (0, {})",
                address.0,
                address.1,
                buf.data.len()
            );
        }
        Ok(Invocation {
            address,
            simple: match parsed.name {
                'p' => Box::new(commands::P),
                'a' => Box::new(commands::A(parsed.string_args[0].clone())),
                'c' => Box::new(commands::C(parsed.string_args[0].clone())),
                'i' => Box::new(commands::I(parsed.string_args[0].clone())),
                'd' => Box::new(commands::D),

                'x' => Box::new(commands::X(
                    parsed.string_args[0].clone(),
                    parsed.command_args[0].clone(),
                    false,
                )),
                'y' => Box::new(commands::X(
                    parsed.string_args[0].clone(),
                    parsed.command_args[0].clone(),
                    true,
                )),

                'g' => Box::new(commands::Conditional(
                    parsed.string_args[0].clone(),
                    parsed.command_args[0].clone(),
                    false,
                )),
                'v' => Box::new(commands::Conditional(
                    parsed.string_args[0].clone(),
                    parsed.command_args[0].clone(),
                    true,
                )),

                '{' => Box::new(commands::Brace(parsed.command_args)),

                '=' => Box::new(commands::Equals),
                _ => unimplemented!(),
            },
        })
    }

    pub fn execute(self, w: &mut Write, buf: &mut Buffer) -> Result<Range, Box<Error>> {
        self.simple.execute(w, buf, self.address)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn open_buffer() {
        let b = super::Buffer::new("xd lol".as_bytes()).unwrap();
        assert_eq!(b.data, "xd lol");
    }
}
