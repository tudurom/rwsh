pub mod lex;

use self::lex::Lexer;
use crate::util::BufReadChars;
use std::iter::Peekable;

#[derive(Debug)]
pub struct Command(pub String, pub Vec<String>);

#[derive(Debug)]
pub enum ParseNode {
    WordList(Vec<String>),
    Command(Command),
}

pub struct Parser {
    lexer: Peekable<Lexer<BufReadChars>>,
    error: Option<String>,
}

impl Parser {
    pub fn new(lexer: Lexer<BufReadChars>) -> Parser {
        Parser {
            lexer: lexer.peekable(),
            error: None,
        }
    }
    fn parse_command(&mut self) -> Option<Result<Command, String>> {
        match self.parse_word_list() {
            Some(Ok(name)) => {
                let mut v: Vec<String> = Vec::new();
                while let Some(r) = self.lexer.peek() {
                    match r {
                        Ok(tok) => match tok {
                            lex::Token::WordString(_, _) => match self.parse_word_list() {
                                Some(Ok(wl)) => {
                                    v.push(wl);
                                }
                                Some(Err(e)) => return Some(Err(e)),
                                None => panic!("no WordString"),
                            },
                            lex::Token::Space => {
                                self.lexer.next();
                            }
                            lex::Token::Newline => {
                                self.lexer.next();
                                break;
                            }
                            _ => {
                                return Some(Err(format!("unexpected token {:?} in command", tok)));
                            }
                        },
                        Err(e) => {
                            return Some(Err(e.clone()));
                        }
                    }
                }
                Some(Ok(Command(name, v)))
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
    fn skip_space(&mut self) {
        while let Some(Ok(tok)) = self.lexer.peek() {
            match tok {
                lex::Token::Space | lex::Token::Newline => {},
                _ => break,
            }
            self.lexer.next();
        }
    }
    fn parse_word_list(&mut self) -> Option<Result<String, String>> {
        let mut r = String::new();
        self.skip_space();
        match self.lexer.peek() {
            Some(Ok(lex::Token::WordString(_, _))) => {},
            Some(Ok(tok)) => return Some(Err(format!("unexpected token {:?} in word list", tok))),
            _ => {},
        }
        while let Some(Ok(lex::Token::WordString(_, s))) = self.lexer.peek() {
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

impl Iterator for Parser {
    type Item = Result<ParseNode, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            return None;
        }
        match self.parse_command() {
            Some(Ok(c)) => Some(Ok(ParseNode::Command(c))),
            Some(Err(e)) => {
                self.error = Some(e.clone());
                Some(Err(e))
            }
            None => None,
        }
    }
}

fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}
