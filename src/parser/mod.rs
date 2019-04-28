//! The parsers and lexers of the `rwsh` scripting language and its SRE sublanguage.
pub mod lex;
pub mod sre;

use self::lex::{Lexer, Token};
use crate::util::{BufReadChars, LineReader, ParseError};
use sre::Command as SRECommand;
use std::cell::RefCell;
use std::iter::Peekable;

fn skip_whitespace<R: LineReader>(it: &mut BufReadChars<R>) -> usize {
    let mut len: usize = 0;
    while let Some(&c) = it.peek() {
        if !c.is_whitespace() || c == '\n' {
            break;
        }
        len += 1;
        it.next();
    }
    len
}

pub fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}

#[derive(Debug, PartialEq)]
/// A command tuple is made of its name and its arguments.
pub struct Command(pub String, pub Vec<String>);

#[derive(Debug, PartialEq)]
/// A chain of SRE commands.
///
/// Something like `|> ,a/append/ |> 1,2p`.
pub struct SRESequence(pub Vec<SRECommand>);

#[derive(Debug, PartialEq)]
/// A chain of [`Task`s](struct.Task.html) piped together
pub struct Pipeline(pub Vec<Task>);

#[derive(Debug, PartialEq)]
/// A task is something that will be executed in a pipeline in a process,
/// either an external command, or a SRE program.
pub enum Task {
    Command(Command),
    SREProgram(SRESequence),
}

/// Parses the series of [`Token`s](./lex/enum.Token.html) to the AST ([`ParseNode`s](enum.ParseNode.html)).
#[derive(Clone)]
pub struct Parser<R: LineReader> {
    lexer: RefCell<Peekable<Lexer<R>>>,
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
            lexer: RefCell::new(lexer.peekable()),
            error: None,
        }
    }

    fn peek(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().peek().cloned()
    }

    fn next_tok(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().next()
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
        // the grammar is pipeline ::= command pipeline | command
        match self.parse_task() {
            Some(Ok(t)) => {
                let mut v = vec![t];
                match self.peek() {
                    Some(Ok(lex::Token {
                        kind: lex::TokenKind::Newline,
                        ..
                    })) => {
                        self.next_tok();
                    }
                    Some(Ok(
                        ref tok @ lex::Token {
                            kind: lex::TokenKind::Pipe,
                            ..
                        },
                    ))
                    | Some(Ok(
                        ref tok @ lex::Token {
                            kind: lex::TokenKind::Pizza(_),
                            ..
                        },
                    )) => {
                        if let Some(Ok(lex::Token {
                            kind: lex::TokenKind::Pipe,
                            ..
                        })) = self.peek()
                        {
                            self.next_tok();
                        }
                        match self.parse_pipeline() {
                            Some(Ok(Pipeline(ref mut new_v))) => {
                                v.append(new_v);
                            }
                            Some(Err(e)) => {
                                return Some(Err(e.clone()));
                            }
                            None => {
                                return Some(Err(
                                    tok.new_error("expected pipeline, got EOF".to_owned())
                                ));
                            }
                        }
                    }
                    Some(Ok(tok)) => {
                        return Some(Err(
                            tok.new_error(format!("unexpected token {:?}", tok.kind))
                        ));
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

    fn parse_task(&mut self) -> Option<Result<Task, ParseError>> {
        self.skip_space(false);
        match self.peek() {
            Some(Ok(lex::Token {
                kind: lex::TokenKind::Pizza(_),
                ..
            })) => {
                let mut commands = Vec::<SRECommand>::new();
                while let Some(Ok(lex::Token {
                    kind: lex::TokenKind::Pizza(sre),
                    ..
                })) = self.peek()
                {
                    commands.push(sre);
                    self.next_tok();
                    self.skip_space(true);
                }
                Some(Ok(Task::SREProgram(SRESequence(commands))))
            }
            Some(Ok(lex::Token {
                kind: lex::TokenKind::WordString(_, _),
                ..
            })) => self.parse_command().map(|r| r.map(Task::Command)),
            None => None,
            Some(Err(e)) => Some(Err(e)),
            _ => panic!(),
        }
    }

    /// Parses a command
    ///
    /// A command is a chain of word lists (strings)
    fn parse_command(&mut self) -> Option<Result<Command, ParseError>> {
        match self.parse_word_list() {
            Some(Ok(name)) => {
                let mut v: Vec<String> = Vec::new();
                while let Some(r) = self.peek() {
                    match r {
                        Ok(tok) => match tok {
                            lex::Token {
                                kind: lex::TokenKind::WordString(_, _),
                                ..
                            } => match self.parse_word_list() {
                                Some(Ok(wl)) => {
                                    v.push(wl);
                                }
                                Some(Err(e)) => return Some(Err(e)),
                                None => panic!("no WordString"),
                            },
                            lex::Token {
                                kind: lex::TokenKind::Space,
                                ..
                            } => {
                                self.next_tok();
                            }
                            lex::Token {
                                kind: lex::TokenKind::Newline,
                                ..
                            } => {
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
    fn skip_space(&mut self, break_at_newline: bool) -> usize {
        let mut len: usize = 0;
        while let Some(Ok(lex::Token {
            kind: lex::TokenKind::Space,
            len: l,
            ..
        }))
        | Some(Ok(lex::Token {
            kind: lex::TokenKind::Newline,
            len: l,
            ..
        })) = self.peek()
        {
            if break_at_newline && self.peek().unwrap().unwrap().kind == lex::TokenKind::Newline {
                return len;
            }
            len += l;
            self.next_tok();
        }
        len
    }
    fn parse_word_list(&mut self) -> Option<Result<String, ParseError>> {
        let mut r = String::new();
        self.skip_space(false);
        match self.peek() {
            Some(Ok(lex::Token {
                kind: lex::TokenKind::WordString(_, _),
                ..
            })) => {}
            Some(Ok(lex::Token { kind, pos, .. })) => {
                return Some(Err(ParseError {
                    line: pos.0,
                    col: pos.1,
                    message: format!("unexpected token {:?} in word list", kind),
                }));
            }
            _ => {}
        }
        while let Some(Ok(lex::Token {
            kind: lex::TokenKind::WordString(_, s),
            ..
        })) = self.peek()
        {
            r.push_str(&s);
            self.next_tok();
        }
        if let Some(Err(e)) = self.peek() {
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
    use super::{Command, Pipeline, Task};
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
            Task::Command(Command(
                "dmesg".to_owned(),
                vec!["--facility".to_owned(), "daemon".to_owned()],
            )),
            Task::Command(Command("lolcat".to_owned(), vec![])),
            Task::Command(Command("cat".to_owned(), vec!["-v".to_owned()])),
        ])));
        let ok2: Option<Result<Pipeline, ParseError>> = Some(Ok(Pipeline(vec![Task::Command(
            Command("meow".to_owned(), vec![]),
        )])));
        assert_eq!(p.parse_pipeline(), ok1);
        assert_eq!(p.parse_pipeline(), ok2);
    }

    #[test]
    fn parse_task() {
        let s = "|> 2,3a/something/    |> ,p";
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let task = p.parse_task();
        if let Some(Ok(super::Task::SREProgram(seq))) = task {
            assert_eq!(seq.0.len(), 2);
        } else {
            panic!(task);
        }
    }
}
