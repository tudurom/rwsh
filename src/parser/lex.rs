use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone)]
pub enum Token {
    Space,
    Pipe,
    Newline,
    WordString(char, String),
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer {
            input: input.chars().peekable(),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, String>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&c) = self.input.peek() {
            if is_clear_string_char(c) {
                match read_string('\0', &mut self.input) {
                    Ok(s) => return Some(Ok(Token::WordString('\0', s))),
                    Err(e) => return Some(Err(e)),
                }
            } else if c == '"' || c == '\'' {
                self.input.next();
                match read_string(c, &mut self.input) {
                    Ok(s) => return Some(Ok(Token::WordString(c, s))),
                    Err(e) => return Some(Err(e)),
                }
            } else if c == '|' {
                self.input.next();
                return Some(Ok(Token::Pipe));
            } else if c == '\n' {
                self.input.next();
                return Some(Ok(Token::Newline));
            } else if c.is_whitespace() {
                skip_whitespace(&mut self.input);
                return Some(Ok(Token::Space));
            } else {
                return Some(Err(format!("unexpected character {}", c)));
            }
        }

        None
    }
}

fn skip_whitespace<I: Iterator<Item = char>>(it: &mut Peekable<I>) {
    while let Some(&c) = it.peek() {
        if !c.is_whitespace() {
            break;
        }
        it.next();
    }
}

fn is_clear_string_char(c: char) -> bool {
    match c {
        'a'...'z' | 'A'...'Z' | '-' | '_' | '\\' => true,
        _ => false,
    }
}

fn read_string<I: Iterator<Item = char>>(
    quote: char,
    it: &mut Peekable<I>,
) -> Result<String, String> {
    let mut s = String::new();
    let mut escaping = false;
    if quote == '\0' {
        while let Some(&c) = it.peek() {
            if escaping {
                s.push(super::escape(c));
                escaping = false;
            } else if c == '\\' {
                escaping = true;
            } else {
                if !is_clear_string_char(c) {
                    break;
                }
                s.push(c);
            }
            it.next();
        }
    } else {
        let mut closed = false;
        while let Some(&c) = it.peek() {
            if escaping {
                s.push(super::escape(c));
                escaping = false;
            } else {
                if c == quote {
                    closed = true;
                    it.next();
                    break;
                }
                if c == '\\' {
                    escaping = true;
                } else {
                    s.push(c);
                }
            }
            it.next();
        }
        if !closed {
            return Err(format!("expected {} at the end of string", quote));
        }
    }
    if escaping {
        Err(format!("expected {} at the end of string", quote))
    } else {
        Ok(s)
    }
}
