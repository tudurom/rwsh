pub mod process;

use std::error::Error;
use std::fmt;
use std::io::{self, stdin, stdout, Write};
use std::iter::Iterator;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl ParseError {
    pub fn mute_error(message: String) -> ParseError {
        ParseError {
            message,
            line: 0,
            col: 0,
        }
    }
}

impl PartialEq<ParseError> for ParseError {
    fn eq(&self, other: &ParseError) -> bool {
        self.message == other.message
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else if self.col == 0 {
            write!(f, "{}: {}", self.line, self.message)
        } else {
            write!(f, "{}:{}: {}", self.line, self.col, self.message)
        }
    }
}

impl Error for ParseError {}

/// An interface for reading lines of UTF-8 texts.
pub trait LineReader {
    fn read_line(&mut self) -> io::Result<Option<String>>;
}

/// A [`LineReader`](trait.LineReader.html) that reads from `stdin` and prints a prompt.
#[derive(Default, Clone)]
pub struct InteractiveLineReader(String);

impl InteractiveLineReader {
    pub fn new() -> InteractiveLineReader {
        InteractiveLineReader(String::new())
    }
}

impl LineReader for InteractiveLineReader {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        print!("> ");
        stdout().flush().unwrap();
        self.0.clear();
        let mut s = String::new();

        match stdin().read_line(&mut s) {
            Err(e) => Err(e),
            Ok(0) => Ok(None),
            Ok(_) => {
                if s.chars().last().unwrap_or_default() != '\n' {
                    s.push('\n');
                }
                Ok(Some(s))
            }
        }
    }
}

/// A char iterator for UTF-8 texts.
#[derive(Clone)]
pub struct BufReadChars<R: LineReader> {
    r: R,
    chars: Vec<char>,
    finished: bool,
    i: usize,
    initialized: bool,
    line: usize,
    col: usize,
    #[allow(clippy::option_option)]
    peeked: Option<Option<char>>,
}

impl<R: LineReader> BufReadChars<R> {
    pub fn new(r: R) -> BufReadChars<R> {
        BufReadChars {
            r,
            chars: Vec::new(),
            finished: false,
            i: 0,
            initialized: false,
            line: 0,
            col: 0,
            peeked: None,
        }
    }

    fn refresh(&mut self) {
        match self.r.read_line().unwrap() {
            Some(line) => {
                self.chars = line.chars().collect();
                self.i = 0;
                self.initialized = true;
                self.line += 1;
                self.col = 0;
            }
            None => self.finished = true,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        if self.i == self.chars.len() {
            None
        } else {
            self.i += 1;
            Some(self.chars[self.i - 1])
        }
    }

    pub fn get_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    pub fn new_error(&self, message: String) -> ParseError {
        ParseError {
            line: self.line,
            col: self.col,
            message,
        }
    }

    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next());
        }
        self.peeked.as_ref().unwrap().as_ref()
    }
}

impl<R: LineReader> Iterator for BufReadChars<R> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(v) = self.peeked.take() {
            return v;
        }
        if self.finished {
            return None;
        }
        if !self.initialized {
            self.refresh();
            return self.next();
        }
        match self.next_char() {
            Some(c) => {
                self.col += 1;
                Some(c)
            }
            None => {
                self.refresh();

                self.next()
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::tests::common::DummyLineReader;

    #[test]
    fn reads_correctly() {
        let correct = [
            'a', 'b', '\n', 'c', 'd', '\n', 'e', 'f', '\n', 'g', 'h', '\n',
        ];
        let s = "ab\ncd\nef\ngh";
        let dlr = DummyLineReader(s.lines());
        let buf = super::BufReadChars::new(dlr);

        assert_eq!(buf.collect::<Vec<char>>(), correct);
    }
}
