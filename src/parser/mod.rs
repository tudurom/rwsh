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
//! The parsers and lexers of the `rwsh` scripting language and its SRE sublanguage.
pub mod lex;
pub mod misc;
pub mod sre;

use self::lex::{LexMode, Lexer, Token};
use crate::shell::pretty::*;
use crate::util::{BufReadChars, ParseError};
use result::ResultOptionExt;
use sre::Command as SRECommand;
use std::cell::RefCell;
use std::rc::Rc;

pub enum WordStringReadMode {
    Unqoted,
    SingleQuoted,
    DoubleQuoted,
    Parameter,
    Pattern,
}

fn skip_whitespace(it: &mut BufReadChars, skip_newlines: bool) -> usize {
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
        Some(Err(e)) => Err(e),
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

    /// A regex pattern.
    Pattern(Vec<Word>),
}

impl Into<Word> for RawWord {
    fn into(self) -> Word {
        Rc::new(RefCell::new(self))
    }
}

/// Returns a `Rc<RefCell<RawWord>>` (aka a `Word`) with the raw word cloned
/// and its component words cloned like so recursively.
pub fn deep_clone_word(w: &Word) -> Word {
    use std::ops::Deref;
    match w.borrow().deref() {
        RawWord::String(s, b) => RawWord::String(s.clone(), *b),
        RawWord::Parameter(wp) => RawWord::Parameter(wp.clone()),
        RawWord::List(ws, b) => RawWord::List(ws.iter().map(deep_clone_word).collect(), *b),
        RawWord::Command(prog) => RawWord::Command(prog.clone()),
        RawWord::Pattern(s) => RawWord::Pattern(s.clone()),
    }
    .into()
}

impl PrettyPrint for RawWord {
    fn pretty_print(&self) -> PrettyTree {
        match self {
            RawWord::String(s, quoted) => PrettyTree {
                text: format!(
                    "word string{} {}",
                    if *quoted { " (quoted)" } else { "" },
                    s
                ),
                children: vec![],
            },
            RawWord::Parameter(param) => PrettyTree {
                text: "word parameter".to_owned(),
                children: vec![PrettyTree {
                    text: format!("name: {}", param.name),
                    children: vec![],
                }],
            },
            RawWord::List(words, quoted) => PrettyTree {
                text: format!("word list{}", if *quoted { " (quoted)" } else { "" }),
                children: words
                    .iter()
                    .map(|w| naked_word(w.clone()).pretty_print())
                    .collect(),
            },
            RawWord::Command(prog) => PrettyTree {
                text: "word command".to_owned(),
                children: vec![prog.pretty_print()],
            },
            RawWord::Pattern(words) => PrettyTree {
                text: "word pattern".to_owned(),
                children: words
                    .iter()
                    .map(|w| naked_word(w.clone()).pretty_print())
                    .collect(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A command tuple is made of its name and its arguments.
pub struct SimpleCommand(pub Word, pub Vec<Word>);

impl SimpleCommand {
    pub fn with_deep_copied_word(&self) -> SimpleCommand {
        SimpleCommand(
            deep_clone_word(&self.0),
            self.1.iter().map(deep_clone_word).collect(),
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A chain of SRE commands.
///
/// Something like `|> ,a/append/ |> 1,2p`.
pub struct SRESequence(pub Vec<SRECommand>);

impl PrettyPrint for SRESequence {
    fn pretty_print(&self) -> PrettyTree {
        PrettyTree {
            text: "SRE sequence".to_owned(),
            children: self.0.iter().map(|c| c.pretty_print()).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A chain of [`Command`s](enum.Command.html) piped together
pub struct Pipeline(pub Vec<Command>);

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Pipeline(Pipeline),
}

#[derive(Debug, PartialEq, Clone)]
pub struct CommandList(pub Node);

impl PrettyPrint for CommandList {
    fn pretty_print(&self) -> PrettyTree {
        match &self.0 {
            Node::Pipeline(p) => PrettyTree {
                text: "command list - pipeline".to_owned(),
                children: p.0.iter().map(|c| c.pretty_print()).collect(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program(pub Vec<CommandList>);

impl PrettyPrint for Program {
    fn pretty_print(&self) -> PrettyTree {
        PrettyTree {
            text: "program".to_owned(),
            children: self.0.iter().map(|c| c.pretty_print()).collect(),
        }
    }
}

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
    /// Like `IfConstruct`, first is the condition, second is the body.
    WhileConstruct(Program, Program),
    /// A switch construct, runs code based on the first pattern that matches.
    /// The first is the word to be matched, second is a list of patterns.
    /// A pattern has a `Word` that is the pattern, and a program, that is the code.
    SwitchConstruct(Word, Vec<(Word, Program)>),
    /// A match construct. It runs code *for each match* of *every pattern* in the text.
    /// The text is given by `stdin`.
    MatchConstruct(Vec<(Word, Program)>),
}

impl PrettyPrint for Command {
    fn pretty_print(&self) -> PrettyTree {
        match self {
            Command::SimpleCommand(sc) => PrettyTree {
                text: "simple command".to_owned(),
                children: vec![
                    PrettyTree {
                        text: "name".to_owned(),
                        children: vec![naked_word(sc.0.clone()).pretty_print()],
                    },
                    PrettyTree {
                        text: "args".to_owned(),
                        children: sc
                            .1
                            .iter()
                            .map(|w| naked_word(w.clone()).pretty_print())
                            .collect(),
                    },
                ],
            },
            Command::SREProgram(seq) => seq.pretty_print(),
            Command::BraceGroup(cls) => PrettyTree {
                text: "brace group".to_owned(),
                children: cls.iter().map(|cl| cl.pretty_print()).collect(),
            },
            Command::IfConstruct(condition, body) => PrettyTree {
                text: "if construct".to_owned(),
                children: vec![
                    PrettyTree {
                        text: "condition - program".to_owned(),
                        children: condition.pretty_print().children,
                    },
                    PrettyTree {
                        text: "body - program".to_owned(),
                        children: body.pretty_print().children,
                    },
                ],
            },
            Command::ElseConstruct(body) => PrettyTree {
                text: "else construct - program".to_owned(),
                children: body.pretty_print().children,
            },
            Command::WhileConstruct(condition, body) => PrettyTree {
                text: "while construct".to_owned(),
                children: vec![
                    PrettyTree {
                        text: "condition - program".to_owned(),
                        children: condition.pretty_print().children,
                    },
                    PrettyTree {
                        text: "body - program".to_owned(),
                        children: body.pretty_print().children,
                    },
                ],
            },
            Command::SwitchConstruct(to_match, patterns) => PrettyTree {
                text: "switch construct".to_owned(),
                children: vec![
                    PrettyTree {
                        text: "word".to_owned(),
                        children: vec![to_match.borrow().pretty_print()],
                    },
                    PrettyTree {
                        text: "items".to_owned(),
                        children: patterns
                            .iter()
                            .map(|(w, prog)| PrettyTree {
                                text: "item".to_owned(),
                                children: vec![
                                    PrettyTree {
                                        text: "pattern".to_owned(),
                                        children: vec![w.borrow().pretty_print()],
                                    },
                                    PrettyTree {
                                        text: "body".to_owned(),
                                        children: prog.pretty_print().children,
                                    },
                                ],
                            })
                            .collect(),
                    },
                ],
            },
            Command::MatchConstruct(patterns) => PrettyTree {
                text: "match construct".to_owned(),
                children: patterns
                    .iter()
                    .map(|(w, prog)| PrettyTree {
                        text: "item".to_owned(),
                        children: vec![
                            PrettyTree {
                                text: "pattern".to_owned(),
                                children: vec![w.borrow().pretty_print()],
                            },
                            PrettyTree {
                                text: "body".to_owned(),
                                children: prog.pretty_print().children,
                            },
                        ],
                    })
                    .collect(),
            },
        }
    }
}

/// Parses the series of [`Token`s](./lex/enum.Token.html) to the AST ([`ParseNode`s](enum.ParseNode.html)).
pub struct Parser {
    lexer: RefCell<Lexer>,
    error: Option<String>,
    brace_group_level: u32,
}

impl Parser {
    pub fn new(r: BufReadChars) -> Parser {
        let l = Lexer::new(r);
        Self::from_lexer(l)
    }

    /// Creates a new parser from a [`Lexer`](./lex/struct.Lexer.html).
    pub fn from_lexer(lexer: Lexer) -> Parser {
        Parser {
            lexer: RefCell::new(lexer),
            error: None,
            brace_group_level: 0,
        }
    }

    fn peek(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().peek().cloned()
    }

    fn peek_char(&self) -> Option<char> {
        self.lexer.borrow_mut().input.peek().cloned()
    }

    fn next_tok(&self) -> Option<Result<Token, ParseError>> {
        self.lexer.borrow_mut().next()
    }

    fn parse_program(&mut self, top_level: bool) -> Option<Result<Program, ParseError>> {
        let mut v = Vec::new();
        self.skip_space(true);

        let mut got_eof = true;
        while let Some(p) = self.peek() {
            got_eof = false;
            if let Err(e) = p {
                return Some(Err(e));
            }
            let p = p.unwrap();
            match p.kind {
                lex::TokenKind::Newline => {
                    if top_level {
                        self.next_tok();
                    }
                    break;
                }
                lex::TokenKind::Semicolon => {
                    self.next_tok();
                    break;
                }
                ref kind if can_start_word(kind) => {}
                lex::TokenKind::LBrace => {}
                lex::TokenKind::Pizza(_) => {}
                _ if top_level => {
                    return Some(Err(p.new_error(format!("unexpected token {:?}", p))))
                }
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
                self.skip_space(true);
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
        let lparen = lparen.unwrap().unwrap();
        self.skip_space(false);
        let condition = match self.parse_program(false) {
            None => {
                return Some(Err(
                    lparen.new_error("expected if condition, got EOF".to_owned())
                ))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(p)) => {
                assert!(!p.0.is_empty());
                p
            }
        };
        self.skip_space(false);
        let rparen = self.next_tok(); // )
        if let Err(e) =
            check_condition_symbol(rparen.clone(), ')', lex::TokenKind::RParen, "if", if_tok)
        {
            return Some(Err(e));
        }
        let rparen = rparen.unwrap().unwrap();
        self.lexer.borrow_mut().ps2_exit();
        self.lexer.borrow_mut().ps2_enter("then".to_owned());
        self.skip_space(false);
        let body = match self.parse_program(false) {
            None => return Some(Err(rparen.new_error("expected if body, got EOF".to_owned()))),
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(b)) => b,
        };
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::IfConstruct(condition, body)))
    }

    fn parse_else(&mut self) -> Option<Result<Command, ParseError>> {
        let else_tok = self.next_tok().unwrap().unwrap(); // else keyword
        self.lexer.borrow_mut().ps2_enter("else".to_owned());
        self.skip_space(false);

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

    fn parse_while(&mut self) -> Option<Result<Command, ParseError>> {
        let while_tok = self.next_tok().unwrap().unwrap(); // while keyword
        self.lexer.borrow_mut().ps2_enter("while".to_owned());

        self.skip_space(false);
        let lparen = self.next_tok(); // (
        if let Err(e) = check_condition_symbol(
            lparen.clone(),
            '(',
            lex::TokenKind::LParen,
            "while",
            while_tok.clone(),
        ) {
            return Some(Err(e));
        }
        let lparen = lparen.unwrap().unwrap();
        self.skip_space(false);
        let condition = match self.parse_program(false) {
            None => {
                return Some(Err(
                    lparen.new_error("expected while condition, got EOF".to_owned())
                ))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(p)) => {
                if p.0.is_empty() {
                    return Some(Err(lparen.new_error("expected while condition".to_owned())));
                }
                p
            }
        };
        self.skip_space(false);
        let rparen = self.next_tok(); // )
        if let Err(e) = check_condition_symbol(
            rparen.clone(),
            ')',
            lex::TokenKind::RParen,
            "while",
            while_tok,
        ) {
            return Some(Err(e));
        }
        let rparen = rparen.unwrap().unwrap();
        self.lexer.borrow_mut().ps2_exit();
        self.lexer.borrow_mut().ps2_enter("do".to_owned());
        self.skip_space(false);
        let body = match self.parse_program(false) {
            None => {
                return Some(Err(
                    rparen.new_error("expected while body, got EOF".to_owned())
                ))
            }
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(b)) => b,
        };
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::WhileConstruct(condition, body)))
    }

    fn parse_switch(&mut self) -> Option<Result<Command, ParseError>> {
        let switch_tok = self.next_tok().unwrap().unwrap(); // switch keyword
        let mut v = Vec::new();
        self.lexer.borrow_mut().ps2_enter("switch".to_owned());

        self.skip_space(false);
        let to_match = match self.parse_word_list() {
            Some(Err(e)) => return Some(Err(e)),
            Some(Ok(w)) => w,
            None => {
                return Some(Err(
                    switch_tok.new_error("expected switch matchee, got EOF".to_owned())
                ))
            }
        };
        loop {
            self.lexer
                .borrow_mut()
                .mode
                .insert(LexMode::END | LexMode::SLASH);
            self.skip_space(false);
            match self.peek() {
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(ref p)) if p.kind == lex::TokenKind::Slash => {}
                Some(Ok(ref p)) if p.kind == lex::TokenKind::End => {
                    self.next_tok();
                    self.lexer
                        .borrow_mut()
                        .mode
                        .remove(LexMode::END | LexMode::SLASH);
                    break;
                }
                Some(Ok(_)) => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected switch pattern".to_owned())))
                }
                None => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected switch pattern, got EOF".to_owned())))
                }
            }
            self.lexer.borrow_mut().mode.remove(LexMode::END);
            let pattern = match self.parse_switch_pattern() {
                Err(e) => return Some(Err(e)),
                Ok(p) => p,
            };
            self.skip_space(false);
            self.lexer.borrow_mut().mode.remove(LexMode::SLASH);
            let prog = match self.parse_program(false) {
                None => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected pattern body, got EOF".to_owned())))
                }
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(p)) => {
                    assert!(!p.0.is_empty());
                    p
                }
            };
            v.push((pattern, prog));
        }
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::SwitchConstruct(to_match, v)))
    }

    fn parse_switch_pattern(&mut self) -> Result<Word, ParseError> {
        self.next_tok(); // /
        let mut v = Vec::new();
        let mut closed = false;

        while let Some(c) = self.peek_char() {
            if c == '/' {
                closed = true;
                self.lexer.borrow_mut().input.next();
                break;
            }
            let w = if c == '$' {
                self.parse_word_dollar()?
            } else {
                self.parse_word_string(WordStringReadMode::Pattern)?.0
            };
            v.push(w);
        }
        if !closed {
            Err(self
                .lexer
                .borrow_mut()
                .input
                .new_error("expected '/', got EOF".to_owned()))
        } else {
            Ok(RawWord::Pattern(v).into())
        }
    }

    fn parse_match(&mut self) -> Option<Result<Command, ParseError>> {
        self.next_tok().unwrap().unwrap(); // match keyword
        let mut v = Vec::new();
        self.lexer.borrow_mut().ps2_enter("match".to_owned());

        loop {
            self.lexer
                .borrow_mut()
                .mode
                .insert(LexMode::END | LexMode::SLASH);
            self.skip_space(false);
            match self.peek() {
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(ref p)) if p.kind == lex::TokenKind::Slash => {}
                Some(Ok(ref p)) if p.kind == lex::TokenKind::End => {
                    self.next_tok();
                    self.lexer
                        .borrow_mut()
                        .mode
                        .remove(LexMode::END | LexMode::SLASH);
                    break;
                }
                Some(Ok(_)) => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected match pattern".to_owned())))
                }
                None => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected match pattern, got EOF".to_owned())))
                }
            }
            self.lexer.borrow_mut().mode.remove(LexMode::END);
            let pattern = match self.parse_switch_pattern() {
                Err(e) => return Some(Err(e)),
                Ok(p) => p,
            };
            self.skip_space(false);
            self.lexer.borrow_mut().mode.remove(LexMode::SLASH);
            let prog = match self.parse_program(false) {
                None => {
                    return Some(Err(self
                        .lexer
                        .borrow()
                        .input
                        .new_error("expected pattern body, got EOF".to_owned())))
                }
                Some(Err(e)) => return Some(Err(e)),
                Some(Ok(p)) => {
                    assert!(!p.0.is_empty());
                    p
                }
            };
            v.push((pattern, prog));
        }
        self.lexer.borrow_mut().ps2_exit();
        Some(Ok(Command::MatchConstruct(v)))
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
                        "while" => return self.parse_while(),
                        "switch" => return self.parse_switch(),
                        "match" => return self.parse_match(),
                        _ => {}
                    }
                }
                self.parse_simple_command()
                    .map(|r| r.map(Command::SimpleCommand))
            }
            Some(Ok(lex::Token { ref kind, .. })) if can_start_word(kind) => self
                .parse_simple_command()
                .map(|r| r.map(Command::SimpleCommand)),
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
                    let word = match self.parse_word_dollar() {
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

    // Becase of the nature of shell strings, this part (parse_word_*) is extremely "evil".
    // These functions operate on chars instead of tokens, but are part of the parser because
    // they require some parsing (such as command substitution).
    pub fn parse_word_string(
        &mut self,
        mode: WordStringReadMode,
    ) -> Result<(Word, usize), ParseError> {
        let mut s = String::new();
        let mut escaping = false;
        if let WordStringReadMode::SingleQuoted = mode {
            //self.input.next(); // skip quote
        }

        let input = &mut self.lexer.borrow_mut().input;
        while let Some(&c) = input.peek() {
            if escaping {
                s.push(escape(c));
                escaping = false;
            } else if c == '\\' {
                if let WordStringReadMode::Pattern = mode {
                    input.next();
                    match input.peek() {
                        Some('\\') => s.push_str("\\\\"),
                        Some('/') => s.push('/'),
                        Some(&x) => {
                            s.push('\\');
                            s.push(x);
                        }
                        None => {}
                    }
                    input.next();
                    continue;
                } else {
                    escaping = true;
                }
            } else {
                match mode {
                    WordStringReadMode::Unqoted => {
                        if !lex::is_clear_string_char(c) {
                            break;
                        }
                    }
                    WordStringReadMode::SingleQuoted => {
                        if c == '\'' {
                            input.next();
                            break;
                        }
                    }
                    WordStringReadMode::DoubleQuoted => {
                        if c == '$' || c == '"' {
                            break;
                        }
                    }
                    WordStringReadMode::Parameter => {
                        if !lex::is_parameter_char(c) {
                            break;
                        }
                    }
                    WordStringReadMode::Pattern => {
                        if c == '/' {
                            break;
                        }
                    }
                }
                s.push(c);
            }
            input.next();
        }

        if escaping {
            Err(input.new_error("expected character, got EOF".to_owned()))
        } else {
            let single_quote = if let WordStringReadMode::SingleQuoted = mode {
                true
            } else {
                false
            };
            let len = s.len();
            Ok((RawWord::String(s, single_quote).into(), len))
        }
    }

    fn parse_word_single_quoted(&mut self) -> Result<Word, ParseError> {
        self.next_tok();
        Ok(self.parse_word_string(WordStringReadMode::SingleQuoted)?.0)
    }

    fn parse_word_double_quoted(&mut self) -> Result<Word, ParseError> {
        self.next_tok(); // "
        let mut v = Vec::new();
        let mut closed = false;

        while let Some(c) = self.peek_char() {
            if c == '"' {
                closed = true;
                self.lexer.borrow_mut().input.next();
                break;
            }
            let w = if c == '$' {
                self.parse_word_dollar()?
            } else {
                self.parse_word_string(WordStringReadMode::DoubleQuoted)?.0
            };
            v.push(w);
        }
        if !closed {
            Err(self
                .lexer
                .borrow_mut()
                .input
                .new_error("expected '\"', got EOF".to_owned()))
        } else {
            Ok(RawWord::List(v, true).into())
        }
    }

    fn parse_word_dollar(&mut self) -> Result<Word, ParseError> {
        self.next_tok(); // $

        let peek = self.peek_char();
        match peek {
            Some('{') => unimplemented!(),
            Some('(') => self.parse_word_command(),
            _ => self.parse_word_parameter(),
        }
    }

    pub fn parse_word_parameter(&mut self) -> Result<Word, ParseError> {
        let (w, len) = self.parse_word_string(WordStringReadMode::Parameter)?;
        if len == 0 {
            Ok(RawWord::Parameter(WordParameter {
                name: "".to_owned(),
            })
            .into())
        } else {
            use std::ops::Deref;
            if let RawWord::String(name, _) = w.borrow().deref() {
                Ok(RawWord::Parameter(WordParameter {
                    name: name.to_string(),
                })
                .into())
            } else {
                panic!()
            }
        }
    }

    pub fn parse_word_command(&mut self) -> Result<Word, ParseError> {
        self.next_tok(); // (
        let prog = self.parse_program(false).invert()?.unwrap();
        self.next_tok(); // )
        Ok(RawWord::Command(prog).into())
    }
}

impl Iterator for Parser {
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
    use std::ops::Deref;
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
    fn read_word_single_quotes() {
        let s = "'hell_o' 'nice' '\\-meme😀' 'test'";
        let _result = ["hell_o", "nice", "-meme😀", "test"];
        let mut result = _result.iter().peekable();
        let mut p = super::Parser::new(new_dummy_buf(s.lines()));
        loop {
            p.lexer.borrow_mut().next(); // skip quote
            let x = p
                .parse_word_string(super::WordStringReadMode::SingleQuoted)
                .unwrap();
            let x = if let super::RawWord::String(s, q) = x.0.borrow().deref() {
                assert_eq!(*q, true);
                s.clone()
            } else {
                panic!()
            };
            let correct = result.next();
            if correct.is_none() && x != "" {
                panic!("still getting results: {:?}", x);
            } else if x == "" {
                break;
            }
            assert_eq!(x, *(correct.unwrap()));
            p.lexer.borrow_mut().next(); // skip space
        }
        assert_eq!(result.peek(), None);
    }

    #[test]
    fn read_parameter_word() {
        let mut p = super::Parser::new(new_dummy_buf("$PARAM".lines()));
        p.lexer.borrow_mut().next();
        assert_eq!(
            p.parse_word_parameter().unwrap(),
            super::RawWord::Parameter(super::WordParameter {
                name: "PARAM".to_owned()
            })
            .into(),
        );
    }

    #[test]
    fn read_word_error() {
        let mut p = super::Parser::new(new_dummy_buf("\"not finished".lines()));
        assert!(p.parse_word_list().unwrap().is_err());
    }
}
