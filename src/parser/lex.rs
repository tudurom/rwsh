use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// One or more non-newline whitespace characters.
    Space,
    /// The pipe (`|`) character.
    Pipe,
    /// A newline.
    Newline,
    /// A sequence of concatenated words.
    ///
    /// The first tuple element is the quote type (`"` or `'`),
    /// or `\0` if none.
    WordString(char, String),
}

/// Transforms text to a sequence of [`Token`s](enum.Token.html).
pub struct Lexer<I: Iterator<Item = char>> {
    input: Peekable<I>,
}

impl<I: Iterator<Item = char>> Lexer<I> {
    /// Creates a new lexer based on a `char` iterator,
    /// usually a [`BufReadChars`](../../util/struct.BufReadChars.html).
    pub fn new(input: I) -> Lexer<I> {
        Lexer {
            input: input.peekable(),
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for Lexer<I> {
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
        if !c.is_whitespace() || c == '\n' {
            break;
        }
        it.next();
    }
}

fn is_special_char(c: char) -> bool {
    c == '|' || c == '\'' || c == '\"' || c == '&'
}

fn is_clear_string_char(c: char) -> bool {
    !(c.is_control() || c.is_whitespace() || is_special_char(c))
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
            it.next();
        }
    } else {
        let mut closed = false;
        while let Some(&c) = it.peek() {
            if escaping {
                s.push(escape(c));
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

fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::common::{new_dummy_buf, DummyLineReader};
    use crate::util::BufReadChars;

    #[test]
    fn read_string_no_quotes() {
        let s = "hell_o nice \\-meme😀 test";
        let _result = ["hell_o", "nice", "-meme😀", "test"];
        let mut result = _result.iter().peekable();
        let mut buf = BufReadChars::new(DummyLineReader(s.lines())).peekable();
        loop {
            let x = super::read_string('\0', &mut buf).unwrap();
            let correct = result.next();
            if correct.is_none() && x != "" {
                panic!("still getting results: {:?}", x);
            } else if x == "" {
                break;
            }
            assert_eq!(x, *(correct.unwrap()));
            buf.next();
        }
        assert_eq!(result.peek(), None);
    }

    #[test]
    fn read_string_quotes() {
        for q in ['\'', '\"'].iter() {
            let s = format!("{0}hell_o{0} {0}nice \\-meme😀 test\\ny{0}", q);
            let _result = ["hell_o", "nice -meme😀 test\ny"];
            let mut result = _result.iter().peekable();
            let mut buf = new_dummy_buf(s.lines()).peekable();
            loop {
                buf.next();
                if buf.peek().is_none() {
                    break;
                }
                let x = super::read_string(*q, &mut buf).unwrap();
                let correct = result.next();
                if correct.is_none() && x != "" {
                    panic!("still getting results: {:?}", x);
                } else if x == "" {
                    break;
                }
                assert_eq!(x, *(correct.unwrap()));
                buf.next();
            }
            assert_eq!(result.peek(), None);
        }
    }

    #[test]
    fn read_string_error() {
        for q in ['\'', '\"'].iter() {
            let s = format!("{}this is a bad string", q);
            let mut buf = new_dummy_buf(s.lines()).peekable();
            buf.next();
            let r = super::read_string(*q, &mut buf);
            assert!(r.is_err());
            assert_eq!(
                r.err().unwrap(),
                format!("expected {} at the end of string", q)
            );
        }
    }

    #[test]
    fn lex() {
        use super::Token::{self, *};
        let s = "echo this\\ is\\ a test\". ignore \"'this 'please | cat\nmeow";
        let ok: Vec<Result<Token, String>> = vec![
            Ok(WordString('\u{0}', "echo".to_owned())),
            Ok(Space),
            Ok(WordString('\u{0}', "this is a".to_owned())),
            Ok(Space),
            Ok(WordString('\u{0}', "test".to_owned())),
            Ok(WordString('\"', ". ignore ".to_owned())),
            Ok(WordString('\'', "this ".to_owned())),
            Ok(WordString('\u{0}', "please".to_owned())),
            Ok(Space),
            Ok(Pipe),
            Ok(Space),
            Ok(WordString('\u{0}', "cat".to_owned())),
            Ok(Newline),
            Ok(WordString('\u{0}', "meow".to_owned())),
            Ok(Newline),
        ];
        let buf = new_dummy_buf(s.lines());
        let l = super::Lexer::new(buf);
        assert_eq!(l.collect::<Vec<_>>(), ok);
    }

    #[test]
    fn lex_err() {
        use super::Token::{self, *};
        let s = "long_unimplemented_stuff & | cat";
        let ok: Vec<Result<super::Token, String>> = vec![
            Ok(WordString('\u{0}', "long_unimplemented_stuff".to_owned())),
            Ok(Space),
            Err("unexpected character &".to_owned()),
        ];
        let buf = new_dummy_buf(s.lines());
        let mut l = super::Lexer::new(buf).peekable();
        let mut result: Vec<Result<Token, String>> = Vec::new();
        while let Some(x) = l.peek() {
            result.push(x.clone());
            if let Err(_) = x {
                break;
            }
            l.next();
        }
        assert_eq!(result, ok);
    }
}
