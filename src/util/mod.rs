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
//! Provides functions and types that are used throughout the codebase.
use rustyline::{config::Builder, error::ReadlineError, Editor};
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::iter::Iterator;

#[derive(Debug, Clone)]
/// ParseError is a kind of error that appears while parsing.
/// It is used to report the position in the buffer to aid in debugging.
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
///
/// **Important**: It is guaranteed that all lines end in '\n' or `EOF`.
pub trait LineReader {
    fn read_line(&mut self) -> Result<Option<String>, Box<Error>>;

    fn ps2_enter(&self, _s: String) {}

    fn ps2_exit(&self) {}
}

#[derive(Default)]
pub struct NullReader;

impl NullReader {
    pub fn new() -> NullReader {
        NullReader
    }
}

impl LineReader for NullReader {
    fn read_line(&mut self) -> Result<Option<String>, Box<Error>> {
        Ok(None)
    }
}

/// A generic, non-interactive [`LineReader`](trait.LineReader.html).
pub struct FileLineReader<R: Read>(BufReader<R>);

impl<R: Read> FileLineReader<R> {
    pub fn new(r: R) -> Result<FileLineReader<R>, Box<Error>> {
        Ok(FileLineReader(BufReader::new(r)))
    }
}

impl<R: Read> LineReader for FileLineReader<R> {
    fn read_line(&mut self) -> Result<Option<String>, Box<Error>> {
        let mut s = String::new();
        self.0.read_line(&mut s)?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }
}

/// A [`LineReader`](trait.LineReader.html) that reads from `stdin` and prints a prompt.
pub struct InteractiveLineReader {
    pub ps1: String,
    pub ps2: String,

    ps2_stack: RefCell<Vec<String>>,
    rl: Editor<()>,
}

impl InteractiveLineReader {
    pub fn new() -> InteractiveLineReader {
        InteractiveLineReader {
            ps1: "€ ".to_owned(), // get it? it's like the dollar sign!
            ps2: "> ".to_owned(),

            ps2_stack: RefCell::new(vec![]),
            rl: Editor::with_config(Builder::new().auto_add_history(true).build()),
        }
    }
}

impl Default for InteractiveLineReader {
    fn default() -> Self {
        Self::new()
    }
}

impl LineReader for InteractiveLineReader {
    fn read_line(&mut self) -> Result<Option<String>, Box<Error>> {
        let ps = if self.ps2_stack.borrow().is_empty() {
            self.ps1.clone()
        } else {
            format!(
                "{}{}",
                self.ps2_stack
                    .borrow()
                    .iter()
                    .filter(|p| { !p.is_empty() })
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(" "),
                self.ps2
            )
        };
        let readline = self.rl.readline(&ps);
        match readline {
            Ok(mut s) => {
                if s.chars().last().unwrap_or_default() != '\n' {
                    s.push('\n');
                }
                Ok(Some(s))
            }
            Err(ReadlineError::Interrupted) => Ok(Some("\n".to_owned())),
            Err(ReadlineError::Eof) => Ok(None),
            Err(err) => Err(Box::new(err)),
        }
    }
    fn ps2_enter(&self, s: String) {
        self.ps2_stack.borrow_mut().push(s);
    }

    fn ps2_exit(&self) {
        self.ps2_stack.borrow_mut().pop();
    }
}

/// A char iterator for UTF-8 texts.
pub struct BufReadChars {
    r: Box<LineReader>,
    chars: Vec<char>,
    finished: bool,
    i: usize,
    initialized: bool,
    line: usize,
    col: usize,
    #[allow(clippy::option_option)]
    peeked: Option<Option<char>>,
}

impl BufReadChars {
    pub fn new(r: Box<LineReader>) -> BufReadChars {
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

    /// Returns the position in the buffer as a tuple.
    /// The first element is the line, the second is the column.
    /// It is mostly used for reporting errors with [`ParseError`](struct.ParseError.html).
    pub fn get_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    /// Returns a new error based on the current position in the buffer.
    pub fn new_error(&self, message: String) -> ParseError {
        ParseError {
            line: self.line,
            col: self.col,
            message,
        }
    }

    /// Returns the current character without advancing.
    pub fn peek(&mut self) -> Option<&<Self as Iterator>::Item> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next());
        }
        self.peeked.as_ref().unwrap().as_ref()
    }

    pub fn ps2_enter(&mut self, s: String) {
        self.r.ps2_enter(s);
    }
    pub fn ps2_exit(&mut self) {
        self.r.ps2_exit();
    }
}

impl Iterator for BufReadChars {
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
        let buf = super::BufReadChars::new(Box::new(dlr));

        assert_eq!(buf.collect::<Vec<char>>(), correct);
    }
}

pub fn regex(r: &str) -> Result<regex::Regex, regex::Error> {
    regex::RegexBuilder::new(r).multi_line(true).build()
}
