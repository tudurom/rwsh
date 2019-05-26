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

fn check_condition_symbol(
    x: Option<Result<Token, ParseError>>,
    ch: char,
    kind: lex::TokenKind,
    construct: &'static str,
    kw_tok: Token,
) -> Result<(), ParseError> {
    match x {
        Some(Err(e)) => return Err(e),
        Some(Ok(ref tok @ Token { .. })) if tok.kind != kind => Err(tok.new_error(format!(
            "expected '{}' in {} condition, got {:?}",
            ch, construct, tok.kind
        ))),
        None => Err(kw_tok.new_error(format!(
            "expected '{}' in {} condition, got EOF",
            ch, construct
        ))),
        _ => Ok(()),
    }
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

    /// A command substitution
    Command(Program),
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

#[derive(Debug, Clone, PartialEq)]
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
    /// An if construct. First is the condition, second is the body.
    IfConstruct(Program, Program),
    /// An else construct. The tuple contains the body.
    ElseConstruct(Program),
}

fn can_start_word(kind: &lex::TokenKind) -> bool {
    if let lex::TokenKind::Word(_)
    | lex::TokenKind::SingleQuote
    | lex::TokenKind::DoubleQuote
    | lex::TokenKind::Dollar = kind
    {
        true
    } else {
        false
    }
}

/// Parses the series of [`Token`s](./lex/enum.Token.html) to the AST ([`ParseNode`s](enum.ParseNode.html)).
#[derive(Clone)]
pub struct Parser<R: LineReader> {
    lexer: RefCell<Lexer<R>>,
    error: Option<String>,
    brace_group_level: u32,
    subshell_level: u32,
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
            brace_group_level: 0,
            subshell_level: 0,
        }
    }

    fn peek(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().peek().cloned()
    }

    fn next_tok(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().next()
    }

    fn parse_program(&mut self, absorb_newline: bool) -> Option<Result<Program, ParseError>> {
        let mut v = Vec::new();
        self.skip_space(true);

        let mut got_eof = true;
        while let Some(p) = self.peek() {
            got_eof = false;
            if let Err(e) = p {
                return Some(Err(e));
            }
            let kind = p.unwrap().kind;
            match kind {
                lex::TokenKind::Newline => {
                    if absorb_newline {
                        self.next_tok();
                    }
                    break;
                }
                lex::TokenKind::Semicolon => {
                    self.next_tok();
                    break;
                }
                lex::TokenKind::Word(_) | lex::TokenKind::LBrace => {}
                _ => break,
            }
            match self.parse_command_list() {
                None => break,
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(cl)) => v.push(cl),
            }
            self.skip_space(true);
        }
        if got_eof && v.is_empty() {
            None
        } else {
            Some(Ok(Program(v)))
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
                self.lexer.borrow_mut().ps2_enter("pipe".to_owned());
                let mut v = vec![t];
                match self.peek() {
                    Some(Ok(lex::Token {
                        kind: lex::TokenKind::Newline,
                        ..
                    })) => {}
                    Some(Ok(lex::Token {
                        kind: lex::TokenKind::Semicolon,
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
                                self.lexer.borrow_mut().ps2_exit();
                                return Some(Err(e.clone()));
                            }
                            None => {
                                self.lexer.borrow_mut().ps2_exit();
                                return Some(Err(
                                    tok.new_error("expected pipeline, got EOF".to_owned())
                                ));
                            }
                        }
                    }
                    Some(Err(e)) => {
                        self.lexer.borrow_mut().ps2_exit();
                        return Some(Err(e.clone()));
                    }
                    _ => {}
                }
                self.lexer.borrow_mut().ps2_exit();
                Some(Ok(Pipeline(v)))
            }
            Some(Err(e)) => Some(Err(e.clone())),
            None => None,
        }
    }

    fn parse_if(&mut self) -> Option<Result<Command, ParseError>> {
        let if_tok = self.next_tok().unwrap().unwrap(); // if keyword
        self.lexer.borrow_mut().ps2_enter("if".to_owned());

        self.skip_space(false);
        let lparen = self.next_tok(); // (
        if let Err(e) = check_condition_symbol(
            lparen.clone(),
            '(',
            lex::TokenKind::LParen,
            "if",
            if_tok.clone(),
        ) {
            return Some(Err(e));
        }
        let prog = match self.parse_program(false) {
            None => {
                return Some(Err(lparen
                    .unwrap()
                    .unwrap()
                    .new_error("expected if condition, got EOF".to_owned())))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(p)) => {
                if p.0.is_empty() {
                    return Some(Err(lparen
                        .unwrap()
                        .unwrap()
                        .new_error("expected if condition".to_owned())));
                }
                p
            }
        };
        let rparen = self.next_tok(); // )
        if let Err(e) =
            check_condition_symbol(rparen.clone(), ')', lex::TokenKind::RParen, "if", if_tok)
        {
            return Some(Err(e));
        }
        let body = match self.parse_program(false) {
            None => {
                return Some(Err(rparen
                    .unwrap()
                    .unwrap()
                    .new_error("expected if body, got EOF".to_owned())))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(b)) => b,
        };
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::IfConstruct(prog, body)))
    }

    fn parse_else(&mut self) -> Option<Result<Command, ParseError>> {
        let else_tok = self.next_tok().unwrap().unwrap(); // else keyword
        self.lexer.borrow_mut().ps2_enter("else".to_owned());

        let body = match self.parse_program(false) {
            None => {
                return Some(Err(
                    else_tok.new_error("expected else body, got EOF".to_owned())
                ))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(b)) => b,
        };
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::ElseConstruct(body)))
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
                let mut last = self.next_tok().unwrap().unwrap();
                self.brace_group_level += 1;
                self.lexer.borrow_mut().ps2_enter("brace".to_owned());
                let mut lists = Vec::<CommandList>::new();
                while let Some(Ok(tok)) = self.peek() {
                    last = tok;
                    match self.parse_command_list() {
                        None => break,
                        Some(Ok(cl)) => lists.push(cl),
                        Some(Err(e)) => return Some(Err(e)),
                    }
                }
                match self.next_tok() {
                    Some(Ok(Token { .. })) => {}
                    Some(Err(e)) => return Some(Err(e)),
                    None => return Some(Err(last.new_error("expected '}', got EOF".to_owned()))),
                }

                self.lexer.borrow_mut().ps2_exit();
                self.skip_space(true);
                Some(Ok(Command::BraceGroup(lists)))
            }
            Some(Ok(lex::Token {
                kind: lex::TokenKind::RBrace,
                ..
            })) if self.brace_group_level > 0 => {
                self.brace_group_level -= 1;
                None
            }
            Some(Ok(lex::Token {
                kind: lex::TokenKind::Word(w),
                ..
            })) => {
                use std::ops::Deref;
                if let RawWord::String(s, false) = w.borrow().deref() {
                    match s.as_ref() {
                        "if" => return self.parse_if(),
                        "else" => return self.parse_else(),
                        _ => {}
                    }
                }
                self.parse_simple_command()
                    .map(|r| r.map(Command::SimpleCommand))
            }
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(x)) => Some(Err(x.new_error(format!("unexpected token {:?}", x)))),
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
                            lex::Token { ref kind, .. } if can_start_word(kind) => {
                                match self.parse_word_list() {
                                    Some(Ok(wl)) => {
                                        v.push(wl);
                                    }
                                    Some(Err(e)) => return Some(Err(e)),
                                    None => panic!("no WordString"),
                                }
                            }
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
        while let Some(Ok(lex::Token { kind, .. })) = self.peek() {
            match kind {
                lex::TokenKind::Word(word) => {
                    v.push(word);
                    self.next_tok();
                }
                lex::TokenKind::SingleQuote => {
                    let word = match self.parse_word_single_quoted() {
                        Ok(w) => w,
                        Err(e) => return Some(Err(e)),
                    };
                    v.push(word)
                }
                lex::TokenKind::DoubleQuote => {
                    let word = match self.parse_word_double_quoted() {
                        Ok(w) => w,
                        Err(e) => return Some(Err(e)),
                    };
                    v.push(word)
                }
                lex::TokenKind::Dollar => {
                    let word = match self.parse_word_parameter() {
                        Ok(w) => w,
                        Err(e) => return Some(Err(e)),
                    };
                    v.push(word);
                }
                _ => break,
            }
        }
        if let Some(Err(e)) = self.peek() {
            Some(Err(e.clone()))
        } else if v.is_empty() {
            None
        } else {
            Some(Ok(RawWord::List(v, false).into()))
        }
    }

    fn parse_word_single_quoted(&mut self) -> Result<Word, ParseError> {
        self.next_tok();
        Ok(self
            .lexer
            .borrow_mut()
            .read_word_string(lex::WordStringReadMode::SingleQuoted)?
            .0)
    }

    fn parse_word_double_quoted(&mut self) -> Result<Word, ParseError> {
        self.next_tok();
        self.lexer.borrow_mut().read_double_quoted()
    }

    fn parse_word_parameter(&mut self) -> Result<Word, ParseError> {
        self.next_tok();
        let param = self.lexer.borrow_mut().read_word_parameter()?.0;
        Ok(RawWord::Parameter(param).into())
    }
}

impl<R: LineReader> Iterator for Parser<R> {
    type Item = Result<Program, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return None;
        }
        self.parse_program(true)
    }
}

#[cfg(test)]
pub mod tests {
    use super::{Command, ParseError, Pipeline, RawWord, SimpleCommand};
    use crate::tests::common::new_dummy_buf;
    use std::cell::RefCell;
    use std::rc::Rc;

    macro_rules! word {
        ($str:expr) => {
            word!($str, false)
        };
        ($str:expr, $quote:expr) => {
            Rc::new(RefCell::new(RawWord::List(
                vec![Rc::new(RefCell::new(RawWord::String($str, $quote)))],
                false,
            )))
        };
    }
    #[test]
    fn parse_simple_command() {
        let s = "echo 'Hello, world!'\nextra command";
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        let ok1: Option<Result<SimpleCommand, ParseError>> = Some(Ok(SimpleCommand(
            word!("echo".to_owned()),
            vec![word!("Hello, world!".to_owned(), true)],
        )));
        let ok2: Option<Result<SimpleCommand, ParseError>> = Some(Ok(SimpleCommand(
            word!("extra".to_owned()),
            vec![word!("command".to_owned())],
        )));
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
            Command::SimpleCommand(SimpleCommand(
                word!("cat".to_owned()),
                vec![word!("-v".to_owned())],
            )),
        ])));
        let ok2: Option<Result<Pipeline, ParseError>> =
            Some(Ok(Pipeline(vec![Command::SimpleCommand(SimpleCommand(
                word!("meow".to_owned()),
                vec![],
            ))])));
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

    #[test]
    fn read_word() {
        use crate::parser::{RawWord, WordParameter};
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut p = super::Parser::new(new_dummy_buf("\"Hello, my name is $NAME!\"".lines()));
        let r = p.parse_word_list().unwrap().unwrap();
        // i'm not proud of this
        use std::ops::Deref;
        assert_eq!(
            &RawWord::List(
                vec![RawWord::List(
                    vec![
                        Rc::new(RefCell::new(RawWord::String(
                            "Hello, my name is ".to_owned(),
                            false
                        ))),
                        Rc::new(RefCell::new(RawWord::Parameter(WordParameter {
                            name: "NAME".to_owned()
                        }))),
                        Rc::new(RefCell::new(RawWord::String("!".to_owned(), false)))
                    ],
                    true
                )
                .into()],
                false
            ),
            r.borrow().deref()
        );
    }

    #[test]
    fn read_word_error() {
        let mut p = super::Parser::new(new_dummy_buf("\"not finished".lines()));
        assert!(p.parse_word_list().unwrap().is_err());
    }
}
