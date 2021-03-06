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
//! Lexing routines.
pub mod sre;

use super::{escape, skip_whitespace};
use crate::util::{BufReadChars, NullReader, ParseError};
use bitflags::bitflags;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Nothing,
    /// One or more non-newline whitespace characters.
    Space,
    /// The pipe (`|`) character.
    Pipe,
    /// A structural regular expression pipe (`|>`) and its SRE code
    Pizza,
    Newline,
    /// A sequence of concatenated words.
    Word(String),
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
    /// '&'
    Ampersand,
    /// `||`
    Or,
    /// `&&`
    And,
}

impl TokenKind {
    pub fn word(self) -> String {
        if let TokenKind::Word(s) = self {
            s
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
    /// Enables and disables certain tokens, based on the current parser state.
    pub struct LexMode: u32 {
        /// Enables the `end` keyword.
        const END   = 0b0000_0001;
        /// Enables the [`Slash` token](enum.TokenKind.html#variant.Slash).
        const SLASH = 0b0000_0010;
    }
}

/// Transforms text to a sequence of [`Token`s](enum.Token.html).
pub struct Lexer {
    pub input: BufReadChars,
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

impl Lexer {
    /// Creates a new lexer based on a `char` iterator,
    /// usually a [`BufReadChars`](../../util/struct.BufReadChars.html).
    pub fn new(input: BufReadChars) -> Lexer {
        Lexer {
            input,
            pipe_follows: false,
            errored: false,
            mode: LexMode::empty(),

            peeked: None,
        }
    }

    /// Reset the lexer to clean state. Used after encountering an error in interractive mode.
    pub fn reload(&mut self) {
        self.mode = LexMode::empty();
        self.pipe_follows = false;
        self.errored = false;
        self.peeked = None;
        self.input.ps2_clear();
        self.input.refresh();
    }

    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next());
        }
        self.peeked.as_ref().unwrap().as_ref()
    }

    /// Put a new parsing context on the stack to show in the prompt.
    pub fn ps2_enter(&mut self, s: String) {
        self.input.ps2_enter(s);
    }

    /// Remove the current parsing context from the stack.
    pub fn ps2_exit(&mut self) {
        self.input.ps2_exit();
    }

    /// Switch the input source to null.
    pub fn blindfold(&mut self) {
        self.input = BufReadChars::new(Box::new(NullReader::new()));
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
                    if !is_clear_string_char(c) || (self.mode.contains(LexMode::SLASH) && c == '/')
                    {
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

// KEEP SORTED
static SYMBOLS: &'static [(char, TokenKind)] = &[
    ('"', TokenKind::DoubleQuote),
    ('$', TokenKind::Dollar),
    ('\'', TokenKind::SingleQuote),
    ('(', TokenKind::LParen),
    (')', TokenKind::RParen),
    (';', TokenKind::Semicolon),
    ('{', TokenKind::LBrace),
    ('}', TokenKind::RBrace),
    ('🍕', TokenKind::Pizza),
];

fn find_symbol(c: char) -> Option<TokenKind> {
    SYMBOLS
        .binary_search_by(|probe| probe.0.cmp(&c))
        .ok()
        .map(|i| SYMBOLS[i].1.clone())
}

impl Iterator for Lexer {
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
                    Some(Ok(tok!(TokenKind::Pizza, 2, self.input)))
                } else if let Some('|') = self.input.peek() {
                    self.input.next();
                    Some(Ok(tok!(TokenKind::Or, 2, self.input)))
                } else {
                    Some(Ok(tok!(TokenKind::Pipe, 1, self.input)))
                }
            } else if c == '&' {
                self.input.next();
                if let Some('&') = self.input.peek() {
                    self.input.next();
                    Some(Ok(tok!(TokenKind::And, 2, self.input)))
                } else {
                    Some(Ok(tok!(TokenKind::Ampersand, 1, self.input)))
                }
            } else if c == '\n' {
                self.input.next();
                Some(Ok(tok!(TokenKind::Newline, 0, self.input)))
            } else if let Some(kind) = find_symbol(c) {
                self.input.next();
                Some(Ok(tok!(kind, 1, self.input)))
            } else if c == '/' && self.mode.contains(LexMode::SLASH) {
                self.input.next();
                Some(Ok(tok!(TokenKind::Slash, 1, self.input)))
            } else if c.is_whitespace() {
                let len = skip_whitespace(&mut self.input, true);
                Some(Ok(tok!(TokenKind::Space, len, self.input)))
            } else {
                match self.read_unquoted_word() {
                    Ok(s) => {
                        if self.mode.contains(LexMode::END) && s == "end" {
                            Some(Ok(tok!(TokenKind::End, 3, self.input)))
                        } else {
                            Some(Ok(tok!(TokenKind::Word(s), s.len(), self.input)))
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
        || c == '['
        || c == ']'
        || ((is_clear_string_char(c) && !is_special_char(c))
            && !c.is_ascii_punctuation()
            && c != '\n')
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
        use super::LexMode;
        use super::TokenKind::*;
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
        assert_eq!(lex.next(), Some(Ok(tok!(Word("end".to_owned())))));
        lex.mode.insert(LexMode::END);
        assert_eq!(lex.next(), Some(Ok(tok!(Space))));
        assert_eq!(lex.next(), Some(Ok(tok!(End))));
    }

    #[test]
    fn slash_mode() {
        use super::LexMode;
        use super::TokenKind::*;
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
        assert_eq!(lex.next(), Some(Ok(tok!(Word("something".to_owned())))));
        assert_eq!(lex.next(), Some(Ok(tok!(Slash))));
        lex.mode.remove(LexMode::SLASH);
        assert_eq!(lex.next(), Some(Ok(tok!(Word("/".to_owned())))));
    }

    #[test]
    fn lex() {
        let s = "test | {cat\nmeow}())} |>\n";
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
            Ok(tok!(Word("test".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(LBrace)),
            Ok(tok!(Word("cat".to_owned()))),
            Ok(tok!(Newline)),
            Ok(tok!(Word("meow".to_owned()))),
            Ok(tok!(RBrace)),
            Ok(tok!(LParen)),
            Ok(tok!(RParen)),
            Ok(tok!(RParen)),
            Ok(tok!(RBrace)),
            Ok(tok!(Space)),
            Ok(tok!(Pizza)),
            Ok(tok!(Newline)),
        ];
        let l = super::Lexer::new(buf);
        assert_eq!(l.collect::<Vec<_>>(), ok);
    }

    /*
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
    */
}
