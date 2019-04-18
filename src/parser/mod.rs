pub mod lex;
pub mod sre;

use self::lex::Lexer;
use crate::util::{BufReadChars, LineReader, ParseError};
use std::iter::Peekable;

/// A command tuple is made of its name and its arguments.
#[derive(Debug, PartialEq)]
pub struct Command(pub String, pub Vec<String>);

/*
pub enum Task {
    Command(Command),
    SREProgram(SREProgram),
}
*/

/// A chain of [`Command`s](struct.Command.html) piped together
#[derive(Debug, PartialEq)]
pub struct Pipeline(pub Vec<Command>);

/// Parses the series of [`Token`s](./lex/enum.Token.html) to the AST ([`ParseNode`s](enum.ParseNode.html)).
pub struct Parser<R: LineReader> {
    lexer: Peekable<Lexer<R>>,
    error: Option<String>,
}

impl<R: LineReader> Parser<R> {
    pub fn new(r: BufReadChars<R>) -> Parser<R> {
        let l = Lexer::new(r);
        Self::from_lexer(l)
    }

    /// Creates a new parser from a [`Lexer`](./lex/struct.Lexer.html).
    pub fn from_lexer(lexer: Lexer<R>) -> Parser<R> {
        Parser {
            lexer: lexer.peekable(),
            error: None,
        }
    }

    /// Parses a pipeline.
    ///
    /// A pipeline is a chain of piped commands
    ///
    /// # Example
    ///
    /// `dmesg | lolcat`
    ///
    /// Here, there are two commands, `dmesg` and `lolcat`, piped together.
    fn parse_pipeline(&mut self) -> Option<Result<Pipeline, ParseError>> {
        self.skip_space();
        // the grammar is pipeline ::= command pipeline | command
        match self.parse_command() {
            Some(Ok(c)) => {
                let mut v: Vec<Command> = vec![c];
                match self.lexer.peek() {
                    Some(Ok(lex::Token{kind: lex::TokenKind::Newline, ..})) => {
                        self.lexer.next();
                    }
                    Some(Ok(lex::Token{kind: lex::TokenKind::Pipe, ..})) => {
                        self.lexer.next();
                        match self.parse_pipeline() {
                            Some(Ok(Pipeline(ref mut new_v))) => {
                                v.append(new_v);
                            }
                            Some(Err(e)) => {
                                return Some(Err(e.clone()));
                            }
                            None => {
                                return Some(Err(ParseError::mute_error("expected pipeline, got EOF".to_owned())));
                            }
                        }
                    }
                    Some(Ok(t)) => {
                        return Some(Err(ParseError::mute_error(format!("unexpected token {:?}", t))));
                    }
                    Some(Err(e)) => {
                        return Some(Err(e.clone()));
                    }
                    None => {}
                }
                Some(Ok(Pipeline(v)))
            }
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
        }
    }

    /// Parses a command
    ///
    /// A command is a chain of word lists (strings)
    fn parse_command(&mut self) -> Option<Result<Command, ParseError>> {
        match self.parse_word_list() {
            Some(Ok(name)) => {
                let mut v: Vec<String> = Vec::new();
                while let Some(r) = self.lexer.peek() {
                    match r {
                        Ok(tok) => match tok {
                            lex::Token{kind: lex::TokenKind::WordString(_, _), ..} => match self.parse_word_list() {
                                Some(Ok(wl)) => {
                                    v.push(wl);
                                }
                                Some(Err(e)) => return Some(Err(e)),
                                None => panic!("no WordString"),
                            },
                            lex::Token{kind: lex::TokenKind::Space, ..} => {
                                self.lexer.next();
                            }
                            lex::Token{kind: lex::TokenKind::Newline, ..} => {
                                break;
                            }
                            _ => {
                                break;
                            }
                        },
                        Err(e) => {
                            return Some(Err(e.clone()));
                        }
                    }
                }
                Some(Ok(Command(name, v)))
            }
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
        }
    }
    fn skip_space(&mut self) -> usize {
        let mut len: usize = 0;
        while let Some(Ok(lex::Token{kind: lex::TokenKind::Space, len: l, ..})) | Some(Ok(lex::Token{kind: lex::TokenKind::Newline, len: l, ..})) = self.lexer.peek() {
            len += l;
            self.lexer.next();
        }
        len
    }
    fn parse_word_list(&mut self) -> Option<Result<String, ParseError>> {
        let mut r = String::new();
        self.skip_space();
        match self.lexer.peek() {
            Some(Ok(lex::Token{kind: lex::TokenKind::WordString(_, _), ..})) => {}
            Some(Ok(tok)) => return Some(Err(ParseError::mute_error(format!("unexpected token {:?} in word list", tok)))),
            _ => {}
        }
        while let Some(Ok(lex::Token{kind: lex::TokenKind::WordString(_, s), ..})) = self.lexer.peek() {
            r.push_str(s);
            self.lexer.next();
        }
        if let Some(Err(e)) = self.lexer.peek() {
            Some(Err(e.clone()))
        } else if r.is_empty() {
            None
        } else {
            Some(Ok(r))
        }
    }
}

impl<R: LineReader> Iterator for Parser<R> {
    type Item = Result<Pipeline, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return None;
        }
        self.parse_pipeline()
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Command, Pipeline};
    use crate::tests::common::new_dummy_buf;
    use crate::util::ParseError;

    #[test]
    fn parse_simple_command() {
        let s = "echo Hello,\\ w\"or\"ld\\! This is a 't''e''s''t'.\n\nextra command\n\n\n";
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let ok1: Option<Result<Command, ParseError>> = Some(Ok(Command(
            "echo".to_owned(),
            vec![
                "Hello, world!".to_owned(),
                "This".to_owned(),
                "is".to_owned(),
                "a".to_owned(),
                "test.".to_owned(),
            ],
        )));
        let ok2: Option<Result<Command, ParseError>> =
            Some(Ok(Command("extra".to_owned(), vec!["command".to_owned()])));
        assert_eq!(p.parse_command(), ok1);
        assert_eq!(p.parse_command(), ok2);
    }

    #[test]
    fn parse_pipeline() {
        let s = "   dmesg --facility daemon| lolcat |   cat -v  \n\nmeow\n"; // useless use of cat!
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let ok1: Option<Result<Pipeline, ParseError>> = Some(Ok(Pipeline(vec![
            Command(
                "dmesg".to_owned(),
                vec!["--facility".to_owned(), "daemon".to_owned()],
            ),
            Command("lolcat".to_owned(), vec![]),
            Command("cat".to_owned(), vec!["-v".to_owned()]),
        ])));
        let ok2: Option<Result<Pipeline, ParseError>> =
            Some(Ok(Pipeline(vec![Command("meow".to_owned(), vec![])])));
        assert_eq!(p.parse_pipeline(), ok1);
        assert_eq!(p.parse_pipeline(), ok2);
    }
}
