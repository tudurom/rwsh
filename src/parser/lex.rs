use std::iter::Peekable;

#[derive(Debug, Clone)]
pub enum Token {
    Space,
    Pipe,
    Newline,
    WordString(char, String)
}

pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut result = Vec::new();

    let mut it = input.chars().peekable();
    while let Some(&c) = it.peek() {
        if is_clear_string_char(c) {
            let s = read_string('\0', &mut it)?;
            result.push(Token::WordString('\0', s));
        } else if c == '"' || c == '\'' {
            it.next();
            let s = read_string(c, &mut it)?;
            result.push(Token::WordString(c, s));
        } else if c == '|' {
            result.push(Token::Pipe);
        } else if c == '\n' {
            result.push(Token::Newline);
        } else if c.is_whitespace() {
            skip_whitespace(&mut it);
            result.push(Token::Space);
        } else {
            return Err(format!("unexpected character {}", c));
        }
    }

    Ok(result)
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
        'a' ... 'z' | 'A' ... 'Z' | '-' | '_' => {
            true
        }
        _ => false
    }
}

fn read_string<I: Iterator<Item = char>>(quote: char, it: &mut Peekable<I>) -> Result<String, String> {
    let mut s = String::new();
    let mut escaping = false;
    if quote == '\0' {
        while let Some(&c) = it.peek () {
            if !is_clear_string_char(c) {
                break;
            }
            if escaping {
                s.push(super::escape(c));
                escaping = false;
            } else {
                if c == '\\' {
                    escaping = true;
                } else {
                    s.push(c);
                }
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
            return Err(format!("expected {} at the end of string", quote))
        }
    }
    if escaping {
        Err(format!("expected {} at the end of string", quote))
    } else {
        Ok(s)
    }
}