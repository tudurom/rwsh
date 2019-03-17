pub mod lex;

use self::lex::Lexer;
use std::iter::Peekable;

#[derive(Debug)]
pub struct Command(String, Vec<String>);

#[derive(Debug)]
pub enum ParseNode {
    WordList(Vec<String>),
    Command(Command),
}

pub struct Parser<'a> {
    lexer: Peekable<lex::Lexer<'a>>,
    error: Option<String>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer) -> Parser {
        Parser {
            lexer: lexer.peekable(),
            error: None,
        }
    }
    fn parse_command(&mut self) -> Option<Result<Command, String>> {
        match self.parse_word_list() {
            Some(Ok(name)) => {
                let mut v: Vec<String> = Vec::new();
                while let Some(Ok(tok)) = self.lexer.peek() {
                    match tok {
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
                        _ => {}
                    }
                }
                match self.lexer.peek() {
                    Some(Err(e)) => Some(Err(e.clone())),
                    _ => Some(Ok(Command(name, v))),
                }
            }
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
    fn parse_word_list(&mut self) -> Option<Result<String, String>> {
        let mut r = String::new();
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

impl<'a> Iterator for Parser<'a> {
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

pub fn parse_line(base: &str) -> Vec<String> {
    let mut in_quote = '\0';
    let mut escaping = false;
    let mut v: Vec<String> = Vec::new();
    let mut s = String::new();
    let mut it = base.chars();
    loop {
        s.clear();
        while let Some(c) = it.next() {
            if in_quote == '\0' && (c == '\'' || c == '"') {
                in_quote = c;
                continue;
            }
            if !escaping && in_quote == '\0' && c.is_whitespace() {
                if s.is_empty() {
                    continue;
                }
                break;
            }
            if in_quote != '\0' {
                if escaping {
                    s.push(escape(c));
                    escaping = false;
                } else if c != in_quote {
                    if c == '\\' {
                        escaping = true;
                    } else {
                        s.push(c);
                    }
                } else {
                    in_quote = '\0';
                }
            } else if escaping {
                s.push(escape(c));
                escaping = false;
            } else if c == '\\' {
                escaping = true;
            } else {
                s.push(c);
            }
        }
        if in_quote != '\0' || escaping || s.is_empty() {
            break;
        }
        v.push(s.clone());
    }
    v
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
