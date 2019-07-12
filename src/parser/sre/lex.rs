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
use crate::util::{BufReadChars, LineReader, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    CharAddress(usize),
    LineAddr(usize),
    Regexp(String),
    BackwardsRegexp(String),

    Dot,
    Plus,
    Minus,
    Comma,
    Semicolon,
    Dollar,
}

pub fn lex_address<R: LineReader>(it: &mut BufReadChars<R>) -> Result<Vec<Token>, ParseError> {
    let mut v: Vec<Token> = Vec::new();
    while let Some(&c) = it.peek() {
        if c == '\n' || c == '|' {
            // we either reached \n as a separator, or the pizza operator
            break;
        } else if c.is_whitespace() {
            scan_space(it);
        } else if c == '#' {
            v.push(scan_address(it, true));
        } else if c.is_digit(10) {
            v.push(scan_address(it, false));
        } else if c == '/' {
            v.push(scan_regexp(it, false)?);
        } else if c == '?' {
            v.push(scan_regexp(it, true)?);
        } else if c == '.' {
            v.push(Token::Dot);
            it.next();
        } else if c == '+' {
            v.push(Token::Plus);
            it.next();
        } else if c == '-' {
            v.push(Token::Minus);
            it.next();
        } else if c == ',' {
            v.push(Token::Comma);
            it.next();
        } else if c == ';' {
            v.push(Token::Semicolon);
            it.next();
        } else if c == '$' {
            v.push(Token::Dollar);
            it.next();
        } else {
            break;
        }
    }

    Ok(v)
}

fn scan_space<R: LineReader>(it: &mut BufReadChars<R>) {
    while let Some(&c) = it.peek() {
        if c.is_whitespace() {
            it.next();
        } else {
            break;
        }
    }
}

fn scan_address<R: LineReader>(it: &mut BufReadChars<R>, is_char: bool) -> Token {
    if is_char {
        it.next(); // eat #
    }

    let mut num: usize = 0;
    let mut init = true;
    while let Some(&c) = it.peek() {
        if c.is_digit(10) {
            num = num * 10 + c.to_digit(10).unwrap() as usize;
            init = false;
        } else {
            break;
        }
        it.next();
    }

    if init {
        num = 1;
    }
    if is_char {
        Token::CharAddress(num)
    } else {
        Token::LineAddr(num)
    }
}

fn scan_regexp<R: LineReader>(
    it: &mut BufReadChars<R>,
    reverse: bool,
) -> Result<Token, ParseError> {
    let mut s = String::new();
    let delimiter = if reverse { '?' } else { '/' };

    s.push(it.next().unwrap());
    let closed = {
        let (new_s, closed) = crate::parser::misc::read_regexp(it, delimiter);
        s.push_str(&new_s);
        closed
    };

    if closed {
        it.next();
        if reverse {
            Ok(Token::BackwardsRegexp(s))
        } else {
            Ok(Token::Regexp(s))
        }
    } else {
        Err(it.new_error("unclosed regex".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::Token::*;
    use crate::tests::common::new_dummy_buf;

    #[test]
    fn regexp() {
        let s = "/lm(a[o-z]\\\\))/?xd(lol)?";
        let mut buf = new_dummy_buf(s.lines());
        assert_eq!(
            super::scan_regexp(&mut buf, false),
            Ok(Regexp("/lm(a[o-z]\\\\))".to_owned()))
        );
        assert_eq!(
            super::scan_regexp(&mut buf, true),
            Ok(BackwardsRegexp("?xd(lol)".to_owned()))
        );
    }

    #[test]
    fn address() {
        let s = "420#69";
        let mut buf = new_dummy_buf(s.lines());
        assert_eq!(super::scan_address(&mut buf, false), LineAddr(420));
        assert_eq!(super::scan_address(&mut buf, true), CharAddress(69));
    }

    #[test]
    fn space() {
        let s = "   \t\t   xy";
        let mut buf = new_dummy_buf(s.lines());
        super::scan_space(&mut buf);
        assert_eq!(buf.peek(), Some(&'x'));
    }

    #[test]
    fn address_lex() {
        let mut buf = new_dummy_buf("-0+,+320-d".lines());
        assert_eq!(
            Ok(vec![
                Minus,
                LineAddr(0),
                Plus,
                Comma,
                Plus,
                LineAddr(320),
                Minus,
            ],),
            super::lex_address(&mut buf)
        );
    }
}
