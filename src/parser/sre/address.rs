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
use crate::parser::lex::sre::{lex_address, Token};
use crate::util::{BufReadChars, ParseError};
use std::cell::RefCell;
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq)]
/// A simple address, by the definition from sam(1).
/// These are the basis of the compound addresses.
pub enum SimpleAddress {
    Nothing,
    Char(usize),
    Line(usize),
    Regex(String, bool),
    Dot,
    Plus,
    Minus,
    Comma,
    Semicolon,
    Dollar,
}

impl Default for SimpleAddress {
    fn default() -> Self {
        SimpleAddress::Nothing
    }
}

#[derive(Debug, Clone, Default)]
/// The internal representation of a compound address.
///
/// This is basically a very weird double-linked list.
/// To get away from Rust's bitey teeth, we are doing some
/// manual management of the list's nodes.
/// `left` and `right` are indices in a vec with all our nodes.
///
/// This structure will be transformed in the end in a [`ComposedAddress`](struct.ComposedAddress.html).
struct Address {
    simple: SimpleAddress,
    left: Option<usize>,
    next: Option<usize>,
}

impl Address {
    pub fn new() -> Address {
        Address {
            simple: SimpleAddress::Nothing,
            left: None,
            next: None,
        }
    }
}

#[derive(Default)]
/// This holds all the [`Address`es](struct.Address.html) that we use to build the final
/// [`ComposedAddress`](struct.ComposedAddress.html).
struct AddressSet {
    vec: RefCell<Vec<Address>>,
}

#[derive(Debug, PartialEq, Clone)]
/// A ready-to-use structural regular expression address.
pub struct ComposedAddress {
    pub simple: SimpleAddress,
    pub left: Option<Box<ComposedAddress>>,
    pub next: Option<Box<ComposedAddress>>,
}

impl Default for ComposedAddress {
    fn default() -> Self {
        ComposedAddress {
            simple: SimpleAddress::Dot,
            left: None,
            next: None,
        }
    }
}

impl ComposedAddress {
    pub fn new(
        simple: SimpleAddress,
        left: Option<Box<ComposedAddress>>,
        next: Option<Box<ComposedAddress>>,
    ) -> Self {
        ComposedAddress { simple, left, next }
    }
}

impl AddressSet {
    fn new() -> AddressSet {
        AddressSet {
            vec: RefCell::new(vec![]),
        }
    }

    fn add(&self, addr: Address) -> usize {
        self.vec.borrow_mut().push(addr);
        self.vec.borrow().len() - 1
    }

    fn replace(&self, i: usize, addr: Address) {
        self.vec.borrow_mut()[i] = addr;
    }

    fn get(&self, i: usize) -> Address {
        self.vec.borrow()[i].clone()
    }

    fn compose(&self, i: usize) -> ComposedAddress {
        let addr = self.get(i);
        ComposedAddress {
            simple: addr.simple,
            left: addr
                .left
                .and_then(|left| Some(Box::new(self.compose(left)))),
            next: addr
                .next
                .and_then(|next| Some(Box::new(self.compose(next)))),
        }
    }
}

fn is_low_precedence(sa: &SimpleAddress) -> bool {
    if let SimpleAddress::Comma | SimpleAddress::Semicolon = sa {
        true
    } else {
        false
    }
}

fn is_high_precedence(sa: &SimpleAddress) -> bool {
    if let SimpleAddress::Plus | SimpleAddress::Minus = sa {
        true
    } else {
        false
    }
}

pub struct Parser<I: Iterator<Item = Token>> {
    tokens: RefCell<Peekable<I>>,
    addr_set: AddressSet,
}

impl Parser<IntoIter<Token>> {
    pub fn new(it: &mut BufReadChars) -> Result<(Parser<IntoIter<Token>>, String), ParseError> {
        let (tokens, original) = lex_address(it)?;
        let it = tokens.into_iter();
        Ok((
            Parser {
                tokens: RefCell::new(it.peekable()),
                addr_set: AddressSet::new(),
            },
            original,
        ))
    }
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn parse(&self) -> Result<Option<ComposedAddress>, String> {
        match self.do_parse() {
            Err(e) => Err(e),
            Ok(None) => Ok(None),
            Ok(Some(f)) => Ok(Some(self.addr_set.compose(f))),
        }
    }
    fn do_parse(&self) -> Result<Option<usize>, String> {
        let mut addr = Address::new();

        addr.left = self.parse_simple_address();
        {
            let mut tokens = self.tokens.borrow_mut();
            addr.simple = match tokens.peek() {
                Some(Token::Comma) => SimpleAddress::Comma,
                Some(Token::Semicolon) => SimpleAddress::Semicolon,
                _ => return Ok(addr.left.and_then(|l| Some(self.fill_defaults(l)))),
            };
            tokens.next();
        }
        addr.next = self.do_parse()?;
        if addr.next.is_some()
            && is_low_precedence(&self.addr_set.get(addr.next.unwrap()).simple)
            && self.addr_set.get(addr.next.unwrap()).left.is_none()
        {
            Err("Eaddress".to_owned())
        } else {
            Ok(Some(self.fill_defaults(self.addr_set.add(addr))))
        }
    }
    fn parse_simple_address(&self) -> Option<usize> {
        let mut addr = Address::new();
        {
            let mut tokens = self.tokens.borrow_mut();
            let tok = tokens.peek();
            addr.simple = match tok {
                Some(Token::CharAddress(n)) => SimpleAddress::Char(*n),
                Some(Token::LineAddr(n)) => SimpleAddress::Line(*n),
                Some(Token::Regexp(re)) => SimpleAddress::Regex(re.clone(), false),
                Some(Token::BackwardsRegexp(re)) => SimpleAddress::Regex(re.clone(), true),
                Some(Token::Dot) => SimpleAddress::Dot,
                Some(Token::Dollar) => SimpleAddress::Dollar,
                Some(Token::Plus) => SimpleAddress::Plus,
                Some(Token::Minus) => SimpleAddress::Minus,
                _ => return None,
            };
            tokens.next();
        }

        addr.next = self.parse_simple_address();
        if addr.next.is_some()
            && !is_high_precedence(&self.addr_set.get(addr.next.unwrap()).simple)
            && !is_high_precedence(&addr.simple)
        {
            addr.next = Some(self.addr_set.add(Address {
                simple: SimpleAddress::Plus,
                next: addr.next,
                left: None,
            }));
        }
        Some(self.addr_set.add(addr))
    }
    fn fill_defaults(&self, mut i: usize) -> usize {
        let mut cur = i;
        let mut init = true;
        loop {
            let mut real_cur = self.addr_set.get(cur);
            if is_high_precedence(&real_cur.simple) {
                /*
                    A high precedende compound is of the form
                        a1+a2
                    or
                        a1-a2
                */

                // if a1 is missing, we put the dot
                if init {
                    i = self.addr_set.add(Address {
                        simple: SimpleAddress::Dot,
                        next: Some(i),
                        left: None,
                    });
                }
                // if a2 is missing, we put the address to one line
                // so it will either add a line, or subtract a line
                if real_cur.next.is_none()
                    || is_high_precedence(&self.addr_set.get(real_cur.next.unwrap()).simple)
                {
                    real_cur.next = Some(self.addr_set.add(Address {
                        simple: SimpleAddress::Line(1),
                        next: real_cur.next,
                        left: None,
                    }));
                }
            } else if is_low_precedence(&real_cur.simple) {
                /*
                    A low precedence compound is of the form
                        a1,a2
                    or
                        a1;a2
                */

                real_cur.left = real_cur.left.and_then(|l| Some(self.fill_defaults(l)));
                // if a1 is missing, we put the null line
                if real_cur.left.is_none() {
                    real_cur.left = Some(self.addr_set.add(Address {
                        simple: SimpleAddress::Line(0),
                        left: None,
                        next: None,
                    }));
                }
                // if a2 is missing, we put the end of the file (dollar)
                if real_cur.next.is_none()
                    || is_low_precedence(&self.addr_set.get(real_cur.next.unwrap()).simple)
                {
                    real_cur.next = Some(self.addr_set.add(Address {
                        simple: SimpleAddress::Dollar,
                        next: real_cur.next,
                        left: None,
                    }));
                }
            }
            self.addr_set.replace(cur, real_cur.clone());
            cur = match real_cur.next {
                Some(c) => c,
                None => break,
            };
            init = false;
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use super::ComposedAddress;
    use super::SimpleAddress::*;
    use crate::tests::common::new_dummy_buf;
    #[test]
    fn simple_address() {
        let s = "-0+";
        let mut buf = new_dummy_buf(s.lines());
        let p = super::Parser::new(&mut buf).unwrap().0;
        let x = p.parse_simple_address().unwrap();
        assert_eq!(
            p.addr_set.compose(x),
            ComposedAddress {
                simple: Minus,
                left: None,
                next: Some(Box::new(ComposedAddress {
                    simple: Line(0),
                    left: None,
                    next: Some(Box::new(ComposedAddress {
                        simple: Plus,
                        left: None,
                        next: None,
                    })),
                })),
            }
        );
    }

    #[test]
    fn address() {
        let s = "-0+,+0-";
        let ok: Result<Option<ComposedAddress>, String> = Ok(Some(ComposedAddress {
            simple: Comma,
            left: Some(Box::new(ComposedAddress {
                simple: Dot,
                left: None,
                next: Some(Box::new(ComposedAddress {
                    simple: Minus,
                    left: None,
                    next: Some(Box::new(ComposedAddress {
                        simple: Line(0),
                        left: None,
                        next: Some(Box::new(ComposedAddress {
                            simple: Plus,
                            left: None,
                            next: Some(Box::new(ComposedAddress {
                                simple: Line(1),
                                left: None,
                                next: None,
                            })),
                        })),
                    })),
                })),
            })),
            next: Some(Box::new(ComposedAddress {
                simple: Dot,
                left: None,
                next: Some(Box::new(ComposedAddress {
                    simple: Plus,
                    left: None,
                    next: Some(Box::new(ComposedAddress {
                        simple: Line(0),
                        left: None,
                        next: Some(Box::new(ComposedAddress {
                            simple: Minus,
                            left: None,
                            next: Some(Box::new(ComposedAddress {
                                simple: Line(1),
                                left: None,
                                next: None,
                            })),
                        })),
                    })),
                })),
            })),
        }));
        let mut buf = new_dummy_buf(s.lines());
        let p = super::Parser::new(&mut buf).unwrap().0;
        assert_eq!(p.parse(), ok);
    }
}
