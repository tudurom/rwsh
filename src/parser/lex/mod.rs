pub mod sre;

use super::sre::{parse_command, Command};
use super::{escape, skip_whitespace};
use super::{RawWord, Word};
use crate::util::{BufReadChars, LineReader, ParseError};

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
    ///
    /// The first tuple element is the quote type (`"` or `'`),
    /// or `\0` if none.
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

/// Transforms text to a sequence of [`Token`s](enum.Token.html).
#[derive(Clone)]
pub struct Lexer<R: LineReader> {
    pub input: BufReadChars<R>,
    pipe_follows: bool,
    errored: bool,

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
                    if !is_clear_string_char(c) {
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
            } else if c.is_whitespace() {
                let len = skip_whitespace(&mut self.input, false);
                Some(Ok(tok!(TokenKind::Space, len, self.input)))
            } else {
                match self.read_unquoted_word() {
                    Ok(s) => Some(Ok(tok!(
                        TokenKind::Word(RawWord::String(s, false).into()),
                        s.len(),
                        self.input
                    ))),
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

    /*
    #[test]
    fn lex() {
        use crate::parser::sre::{
            address::{ComposedAddress, SimpleAddress},
            Command,
        };
        let s =
            "echo this\\ is\\ a test\". ignore \"'this 'please | cat\nmeow |> a/pizza/ | lolcat";
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

        let ok: Vec<Result<Token, ParseError>> = vec![
            Ok(tok!(WordString('\u{0}', "echo".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "this is a".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "test".to_owned()))),
            Ok(tok!(WordString('\"', ". ignore ".to_owned()))),
            Ok(tok!(WordString('\'', "this ".to_owned()))),
            Ok(tok!(WordString('\u{0}', "please".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "cat".to_owned()))),
            Ok(tok!(Newline)),
            Ok(tok!(WordString('\u{0}', "meow".to_owned()))),
            Ok(tok!(Space)),
            Ok(tok!(Pizza(Command::new(
                ComposedAddress::new(SimpleAddress::Dot, None, None),
                'a',
                vec!["pizza".to_owned()],
                vec![],
            )))),
            Ok(tok!(Space)),
            Ok(tok!(Pipe)),
            Ok(tok!(Space)),
            Ok(tok!(WordString('\u{0}', "lolcat".to_owned()))),
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
            Ok(tok!(WordString(
                '\u{0}',
                "long_unimplemented_stuff".to_owned()
            ))),
            Ok(tok!(Space)),
            Err(ParseError {
                message: "unexpected character &".to_owned(),
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
