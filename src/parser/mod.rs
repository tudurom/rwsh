//! The parsers and lexers of the `rwsh` scripting language and its SRE sublanguage.
pub mod lex;
pub mod misc;
pub mod sre;

use self::lex::{Lexer, Token};
use crate::util::{BufReadChars, LineReader, ParseError};
use sre::Command as SRECommand;
use std::cell::RefCell;
use std::rc::Rc;

fn skip_whitespace<R: LineReader>(it: &mut BufReadChars<R>, skip_newlines: bool) -> usize {
    let mut len: usize = 0;
    while let Some(&c) = it.peek() {
        if !c.is_whitespace() || (c == '\n' && !skip_newlines) {
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

#[derive(Clone, PartialEq, Debug)]
pub struct WordParameter {
    pub name: String,
}

pub type Word = Rc<RefCell<RawWord>>;

pub fn naked_word(mut w: Word) -> RawWord {
    Rc::make_mut(&mut w).clone().into_inner()
}

#[derive(Clone, PartialEq, Debug)]
/// The multiple ways of representing a string in the shell.
pub enum RawWord {
    /// An unquoted or single-quoted string.
    /// Second field is `true` if single-quoted.
    String(String, bool),

    /// A parameter expansion expression, such as
    /// `$VAR` or `${VAR}`.
    Parameter(WordParameter),

    /// An unqoted or double-quoted list of words.
    List(Vec<Word>, bool),
}

impl Into<Word> for RawWord {
    fn into(self) -> Word {
        Rc::new(RefCell::new(self))
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A command tuple is made of its name and its arguments.
pub struct SimpleCommand(pub Word, pub Vec<Word>);

#[derive(Debug, PartialEq, Clone)]
/// A chain of SRE commands.
///
/// Something like `|> ,a/append/ |> 1,2p`.
pub struct SRESequence(pub Vec<SRECommand>);

#[derive(Debug, PartialEq, Clone)]
/// A chain of [`Command`s](enum.Command.html) piped together
pub struct Pipeline(pub Vec<Command>);

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Pipeline(Pipeline),
}

#[derive(Debug, PartialEq, Clone)]
pub struct CommandList(pub Node);

#[derive(Debug)]
pub struct Program(pub Vec<CommandList>);

#[derive(Debug, PartialEq, Clone)]
/// A command can be a simple command, a brace group or a control structure.
pub enum Command {
    /// A simple command is the most basic command, like `man 2 ptrace`.
    SimpleCommand(SimpleCommand),
    /// A SRE program is code after the pizza operator.
    SREProgram(SRESequence),
    /// A brace group is code enclosed in brackets.
    BraceGroup(Vec<CommandList>),
}

/// Parses the series of [`Token`s](./lex/enum.Token.html) to the AST ([`ParseNode`s](enum.ParseNode.html)).
#[derive(Clone)]
pub struct Parser<R: LineReader> {
    lexer: RefCell<Lexer<R>>,
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
            lexer: RefCell::new(lexer),
            error: None,
        }
    }

    fn peek(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().peek().cloned()
    }

    fn next_tok(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().next()
    }

    fn parse_program(&mut self) -> Option<Result<Program, ParseError>> {
        match self.parse_command_list() {
            Some(Ok(cl)) => Some(Ok(Program(vec![cl]))),
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
        }
    }

    fn parse_command_list(&mut self) -> Option<Result<CommandList, ParseError>> {
        match self.parse_pipeline() {
            Some(Ok(p)) => Some(Ok(CommandList(Node::Pipeline(p)))),
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
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
        // the grammar is pipeline ::= command pipeline | command
        match self.parse_command() {
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
                        if tok.kind == lex::TokenKind::Pipe {
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
                    Some(Ok(ref tok)) =>
                    {
                        return Some(Err(
                            tok.new_error(format!("unexpected token {:?}", tok.kind))
                        ));
                    }
                    Some(Err(e)) => {
                        return Some(Err(e.clone()));
                    }
                    _ => {}
                }
                Some(Ok(Pipeline(v)))
            }
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
        }
    }

    fn parse_command(&mut self) -> Option<Result<Command, ParseError>> {
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
                Some(Ok(Command::SREProgram(SRESequence(commands))))
            }
            Some(Ok(lex::Token {
                kind: lex::TokenKind::LBrace,
                ..
            })) => {
                self.next_tok();
                self.lexer.borrow_mut().ps2_enter("brace".to_owned());
                let mut lists = Vec::<CommandList>::new();
                while let Some(Ok(tok)) = self.peek() {
                    if tok.kind == lex::TokenKind::RBrace {
                        self.next_tok();
                        break;
                    }
                    match self.parse_command_list().unwrap() {
                        Ok(cl) => lists.push(cl),
                        Err(e) => return Some(Err(e)),
                    }
                }
                self.lexer.borrow_mut().ps2_exit();
                self.skip_space(true);
                Some(Ok(Command::BraceGroup(lists)))
            }
            Some(Ok(lex::Token {
                kind: lex::TokenKind::RBrace,
                ..
            })) => None,
            Some(Ok(lex::Token {
                kind: lex::TokenKind::Word(_),
                ..
            })) => self
                .parse_simple_command()
                .map(|r| r.map(Command::SimpleCommand)),
            None => None,
            Some(Err(e)) => Some(Err(e)),
            _ => panic!(),
        }
    }

    /// Parses a command
    ///
    /// A command is a chain of word lists (strings)
    fn parse_simple_command(&mut self) -> Option<Result<SimpleCommand, ParseError>> {
        match self.parse_word_list() {
            Some(Ok(name)) => {
                let mut v: Vec<Word> = Vec::new();
                while let Some(r) = self.peek() {
                    match r {
                        Ok(tok) => match tok {
                            lex::Token {
                                kind: lex::TokenKind::Word(_),
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
                Some(Ok(SimpleCommand(name, v)))
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
    fn parse_word_list(&mut self) -> Option<Result<Word, ParseError>> {
        self.skip_space(false);
        let mut v = Vec::new();
        while let Some(Ok(lex::Token {
            kind: lex::TokenKind::Word(word),
            ..
        })) = self.peek()
        {
            v.push(word);
            self.next_tok();
        }
        if let Some(Err(e)) = self.peek() {
            Some(Err(e.clone()))
        } else if v.is_empty() {
            None
        } else {
            Some(Ok(RawWord::List(v, false).into()))
        }
    }
}

impl<R: LineReader> Iterator for Parser<R> {
    type Item = Result<Program, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return None;
        }
        self.parse_program()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::tests::common::new_dummy_buf;
    use super::{SimpleCommand,ParseError,RawWord,Pipeline,Command};
    use std::cell::RefCell;
    use std::rc::Rc;

    macro_rules! word {
        ($str:expr) => {word!($str, false)};
        ($str:expr, $quote:expr) => {
            Rc::new(RefCell::new(
                    RawWord::List(vec![
                        Rc::new(RefCell::new(RawWord::String($str, $quote)))
                    ], false)
            ))
        }
    }
    #[test]
    fn parse_simple_command() {
        let s = "echo 'Hello, world!'\nextra command";
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let ok1: Option<Result<SimpleCommand, ParseError>> = Some(Ok(SimpleCommand(
            word!("echo".to_owned()),
            vec![
                word!("Hello, world!".to_owned(), true),
            ],
        )));
        let ok2: Option<Result<SimpleCommand, ParseError>> =
            Some(Ok(SimpleCommand(word!("extra".to_owned()), vec![word!("command".to_owned())])));
        assert_eq!(p.parse_simple_command(), ok1);
        assert_eq!(p.parse_simple_command(), ok2);
    }

    #[test]
    fn parse_pipeline() {
        let s = "   dmesg --facility daemon| lolcat |   cat -v  \n\nmeow\n"; // useless use of cat!
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let ok1: Option<Result<Pipeline, ParseError>> = Some(Ok(Pipeline(vec![
            Command::SimpleCommand(SimpleCommand(
                word!("dmesg".to_owned()),
                vec![word!("--facility".to_owned()), word!("daemon".to_owned())],
            )),
            Command::SimpleCommand(SimpleCommand(word!("lolcat".to_owned()), vec![])),
            Command::SimpleCommand(SimpleCommand(word!("cat".to_owned()), vec![word!("-v".to_owned())])),
        ])));
        let ok2: Option<Result<Pipeline, ParseError>> = Some(Ok(Pipeline(vec![Command::SimpleCommand(
            SimpleCommand(word!("meow".to_owned()), vec![]),
        )])));
        assert_eq!(p.parse_pipeline(), ok1);
        assert_eq!(p.parse_pipeline(), ok2);
    }

    #[test]
    fn parse_sre_command() {
        let s = "|> 2,3a/something/    |> ,p";
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let task = p.parse_command();
        if let Some(Ok(super::Command::SREProgram(seq))) = task {
            assert_eq!(seq.0.len(), 2);
        } else {
            println!("{:#?}", task);
            panic!();
        }
    }

    #[test]
    fn iterator() {
        let p = super::Parser::new(new_dummy_buf("dmesg |> 2,3p\necho 'All ok'\n".lines()));
        let progs = p.collect::<Vec<_>>();
        assert_eq!(progs.len(), 2);
    }
}
