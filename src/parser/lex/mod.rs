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
pub mod sre;

use super::sre::{parse_command, Command};
use super::{escape, skip_whitespace};
use super::{RawWord, Word};
use crate::util::{BufReadChars, LineReader, ParseError};
use bitflags::bitflags;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Nothing,
    /// One or more non-newline whitespace characters.
    Space,
    /// The pipe (`|`) character.
    Pipe,
    /// A structural regular expression pipe (`|>`) and its SRE code
    Pizza(Command),
    Newline,
    /// A sequence of concatenated words.
    Word(Word),
    /// Left brace (`{`)
    LBrace,
    /// Right brace (`}`)
    RBrace,
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    Semicolon,
    DoubleQuote,
    SingleQuote,
    Dollar,

    /// The `end` keyword. Works only in [`END mode`](struct.LexMode.html#associatedconstant.END).
    End,
    /// The slash (`/`) keyword. Works only in [`SLASH mode`](struct.LexMode.html#associatedconstant.SLASH).
    Slash,
}

impl TokenKind {
    pub fn word(self) -> Word {
        if let TokenKind::Word(w) = self {
            w
        } else {
            panic!()
        }
    }
}

#[derive(Clone)]
/// Structure representing a lexical token, together with its position in the file
/// and its size.
pub struct Token {
    pub kind: TokenKind,
    pub pos: (usize, usize),
    pub len: usize,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, w: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(w, "Token({:?})", self.kind)
    }
}

impl Token {
    /// Returns a new [`ParseError`](../../util/struct.ParseError.html) based on the token's position.
    pub fn new_error(&self, message: String) -> ParseError {
        ParseError {
            line: self.pos.0,
            col: self.pos.1,
            message,
        }
    }
}

bitflags! {
    pub struct LexMode: u32 {
        /// Enables the `end` keyword.
        const END   = 0b00000001;
        /// Enables the [`Slash` token](enum.TokenKind.html#variant.Slash).
        const SLASH = 0b00000010;
    }
}

/// Transforms text to a sequence of [`Token`s](enum.Token.html).
#[derive(Clone)]
pub struct Lexer<R: LineReader> {
    pub input: BufReadChars<R>,
    pub mode: LexMode,
    pipe_follows: bool,
    errored: bool,

    #[allow(clippy::option_option)]
    peeked: Option<Option<Result<Token, ParseError>>>,
}

#[macro_export]
macro_rules! tok {
    ($kind:expr, $len:expr, $it:expr) => {
        Token {
            len: $len,
            pos: $it.get_pos(),
            kind: $kind,
        }
    };
}

impl<R: LineReader> Lexer<R> {
    /// Creates a new lexer based on a `char` iterator,
    /// usually a [`BufReadChars`](../../util/struct.BufReadChars.html).
    pub fn new(input: BufReadChars<R>) -> Lexer<R> {
        Lexer {
            input,
            pipe_follows: false,
            errored: false,
            mode: LexMode::empty(),

            peeked: None,
        }
    }

    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next());
        }
        self.peeked.as_ref().unwrap().as_ref()
    }

    pub fn ps2_enter(&mut self, s: String) {
        self.input.ps2_enter(s);
    }

    pub fn ps2_exit(&mut self) {
        self.input.ps2_exit();
    }

    fn read_unquoted_word(&mut self) -> Result<String, ParseError> {
        let c = *self.input.peek().unwrap();
        if is_clear_string_char(c) {
            let mut s = String::new();
            let mut escaping = false;
            while let Some(&c) = self.input.peek() {
                if escaping {
                    s.push(escape(c));
                    escaping = false;
                } else if c == '\\' {
                    escaping = true;
                } else {
                    if !is_clear_string_char(c) || (self.mode.contains(LexMode::SLASH) && c == '/') {
                        break;
                    }
                    s.push(c);
                }
                self.input.next();
            }

            if escaping {
                Err(self
                    .input
                    .new_error("expected character, got EOF".to_owned()))
            } else {
                Ok(s)
            }
        } else {
            Err(self
                .input
                .new_error(format!("unexpected character '{}'", c)))
        }
    }
}

impl<R: LineReader> Iterator for Lexer<R> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.peeked.take() {
            return v;
        }
        if self.errored {
            return None;
        }
        if self.pipe_follows {
            let peek = self.input.peek();
            if let Some('|') | Some('\n') | Some('}') | None = peek {
                self.pipe_follows = false;
            } else if peek.is_some() && peek.unwrap().is_whitespace() {

            } else {
                self.errored = true;
                return Some(Err(self
                    .input
                    .new_error("expected pipe, pizza or newline".to_owned())));
            }
        }
        let r = if let Some(&the_c) = self.input.peek() {
            self.input.ps2_enter("".to_owned());
            let c = if the_c == '#' {
                self.input.next();
                while let Some(&some_c) = self.input.peek() {
                    if some_c == '\n' {
                        break;
                    }
                    self.input.next();
                }
                if let Some(&new_c) = self.input.peek() {
                    new_c
                } else {
                    return None;
                }
            } else {
                the_c
            };
            if c == '|' {
                self.input.next();
                if let Some('>') = self.input.peek() {
                    self.input.next();
                    self.input.ps2_enter("pizza".to_owned());
                    let r = match parse_command(&mut self.input, false) {
                        Ok(Some(sre)) => {
                            self.pipe_follows = true;
                            Some(Ok(tok!(TokenKind::Pizza(sre), 1, self.input)))
                        }
                        Ok(None) => panic!(),
                        Err(e) => {
                            self.errored = true;
                            Some(Err(e))
                        }
                    };
                    self.input.ps2_exit();
                    r
                } else {
                    Some(Ok(tok!(TokenKind::Pipe, 1, self.input)))
                }
            } else if c == '\n' {
                self.input.next();
                Some(Ok(tok!(TokenKind::Newline, 0, self.input)))
            } else if c == '{' {
                self.input.next();
                Some(Ok(tok!(TokenKind::LBrace, 1, self.input)))
            } else if c == '}' {
                self.input.next();
                Some(Ok(tok!(TokenKind::RBrace, 1, self.input)))
            } else if c == '(' {
                self.input.next();
                Some(Ok(tok!(TokenKind::LParen, 1, self.input)))
            } else if c == ')' {
                self.input.next();
                Some(Ok(tok!(TokenKind::RParen, 1, self.input)))
            } else if c == ';' {
                self.input.next();
                Some(Ok(tok!(TokenKind::Semicolon, 1, self.input)))
            } else if c == '\'' {
                self.input.next();
                Some(Ok(tok!(TokenKind::SingleQuote, 1, self.input)))
            } else if c == '"' {
                self.input.next();
                Some(Ok(tok!(TokenKind::DoubleQuote, 1, self.input)))
            } else if c == '$' {
                self.input.next();
                Some(Ok(tok!(TokenKind::Dollar, 1, self.input)))
            } else if c == '/' && self.mode.contains(LexMode::SLASH) {
                self.input.next();
                Some(Ok(tok!(TokenKind::Slash, 1, self.input)))
            } else if c.is_whitespace() {
                let len = skip_whitespace(&mut self.input, false);
                Some(Ok(tok!(TokenKind::Space, len, self.input)))
            } else {
                match self.read_unquoted_word() {
                    Ok(s) => {
                        if self.mode.contains(LexMode::END) && s == "end" {
                            Some(Ok(tok!(TokenKind::End, 3, self.input)))
                        } else {
                            Some(Ok(tok!(
                                TokenKind::Word(RawWord::String(s, false).into()),
                                s.len(),
                                self.input
                            )))
                        }
                    }
                    Err(e) => {
                        self.errored = true;
                        Some(Err(e))
                    }
                }
            }
        } else {
            None
        };
        self.input.ps2_exit();
        r
    }
}

fn is_special_char(c: char) -> bool {
    c == '|'
        || c == '\''
        || c == '\"'
        || c == '&'
        || c == '$'
        || c == '{'
        || c == '}'
        || c == '('
        || c == ')'
        || c == ';'
}

pub fn is_clear_string_char(c: char) -> bool {
    !(c.is_control() || c.is_whitespace() || is_special_char(c))
}

pub fn is_parameter_char(c: char) -> bool {
    //c.is_alphanumeric() || c == '_'
    c == '_'
        || c == '?'
        || ((is_clear_string_char(c) && !is_special_char(c)) && !c.is_ascii_punctuation())
}

#[cfg(test)]
mod tests {
    use super::Token;
    use crate::tests::common::new_dummy_buf;
    use crate::util::ParseError;

    impl PartialEq<Token> for Token {
        fn eq(&self, other: &Token) -> bool {
            self.kind == other.kind
        }
    }

    impl PartialEq<crate::parser::Word> for crate::parser::RawWord {
        fn eq(&self, other: &crate::parser::Word) -> bool {
            &crate::parser::naked_word(other.clone()) == self
        }
    }

    #[test]
    fn read_word_no_quotes() {
        let s = "hell_o nice \\-meme😀 test";
        let _result = ["hell_o", "nice", "-meme😀", "test"];
        let mut result = _result.iter().peekable();
        let mut lex = super::Lexer::new(new_dummy_buf(s.lines()));
        loop {
            if lex.input.peek().is_none() {
                break;
            }
            let x = lex.read_unquoted_word();
            let correct = result.next().map(|x| String::from(*x));
            if correct.is_none() && x.is_ok() {
                panic!("still getting results: {:?}", x);
            } else if x.is_err() {
                panic!(x.err().unwrap());
            }
            assert_eq!(x.ok(), correct);
            lex.next(); // skip space
        }
        assert_eq!(result.peek(), None);
    }

    #[test]
    fn end_mode() {
        use super::TokenKind::*;
        use super::LexMode;
        let mut lex = super::Lexer::new(new_dummy_buf("end end".lines()));
        macro_rules! tok {
            ($kind:expr) => {
                super::Token {
                    kind: $kind,
                    len: 0,
                    pos: (0, 0),
                }
            };
        }
        assert_eq!(lex.next(), Some(Ok(tok!(Word(
                            super::RawWord::String("end".to_owned(), false).into())))));
        lex.mode.insert(LexMode::END);
        assert_eq!(lex.next(), Some(Ok(tok!(Space))));
        assert_eq!(lex.next(), Some(Ok(tok!(End))));
    }

    #[test]
    fn slash_mode() {
        use super::TokenKind::*;
        use super::LexMode;
        let mut lex = super::Lexer::new(new_dummy_buf("/something//".lines()));
        macro_rules! tok {
            ($kind:expr) => {
                super::Token {
                    kind: $kind,
                    len: 0,
                    pos: (0, 0),
                }
            };
        }
        lex.mode.insert(LexMode::SLASH);
        assert_eq!(lex.next(), Some(Ok(tok!(Slash))));
        assert_eq!(lex.next(), Some(Ok(tok!(Word(
                            super::RawWord::String("something".to_owned(), false).into())))));
        assert_eq!(lex.next(), Some(Ok(tok!(Slash))));
        lex.mode.remove(LexMode::SLASH);
        assert_eq!(lex.next(), Some(Ok(tok!(Word(
                            super::RawWord::String("/".to_owned(), false).into())))));
    }

    #[test]
    fn lex() {
        use crate::parser::sre::{
            address::{ComposedAddress, SimpleAddress},
            Command,
        };
        let s = "test | {cat\nmeow}())} |> a/pizza/ | lolcat";
        let buf = new_dummy_buf(s.lines());
        macro_rules! tok {
            ($kind:expr) => {
                super::Token {
                    kind: $kind,
                    len: 0,
                    pos: buf.get_pos(),
                }
            };
        }

        use super::TokenKind::*;
        let ok: Vec<Result<Token, ParseError>> = vec![
            Ok(tok!(Word(
                super::RawWord::String("test".to_owned(), false).into()
            ))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(LBrace)),
            Ok(tok!(Word(
                super::RawWord::String("cat".to_owned(), false).into()
            ))),
            Ok(tok!(Newline)),
            Ok(tok!(Word(
                super::RawWord::String("meow".to_owned(), false).into()
            ))),
            Ok(tok!(RBrace)),
            Ok(tok!(LParen)),
            Ok(tok!(RParen)),
            Ok(tok!(RParen)),
            Ok(tok!(RBrace)),
            Ok(tok!(Space)),
            Ok(tok!(Pizza(Command::new(
                ComposedAddress::new(SimpleAddress::Dot, None, None),
                'a',
                vec!["pizza".to_owned()],
                vec![],
                String::new(),
            )))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(Word(
                super::RawWord::String("lolcat".to_owned(), false).into()
            ))),
            Ok(tok!(Newline)),
        ];
        let l = super::Lexer::new(buf);
        assert_eq!(l.collect::<Vec<_>>(), ok);
    }

    #[test]
    fn lex_err() {
        let s = "long_unimplemented_stuff & | cat";
        let buf = new_dummy_buf(s.lines());
        macro_rules! tok {
            ($kind:expr) => {
                super::Token {
                    kind: $kind,
                    len: 0,
                    pos: buf.get_pos(),
                }
            };
        }
        let ok: Vec<Result<super::Token, ParseError>> = vec![
            Ok(tok!(super::TokenKind::Word(
                super::RawWord::String("long_unimplemented_stuff".to_owned(), false).into()
            ))),
            Ok(tok!(super::TokenKind::Space)),
            Err(ParseError {
                message: "unexpected character '&'".to_owned(),
                line: 0,
                col: 0,
            }),
        ];
        let mut l = super::Lexer::new(buf).peekable();
        let mut result: Vec<Result<Token, ParseError>> = Vec::new();
        while let Some(x) = l.peek() {
            result.push(x.clone());
            if let Err(_) = x {
                break;
            }
            l.next();
        }
        assert_eq!(result, ok);
    }
}
