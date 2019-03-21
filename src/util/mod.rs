use std::io::{self, stdin, stdout, Write};
use std::iter::Iterator;

pub trait LineReader {
    fn read_line(&mut self) -> io::Result<Option<String>>;
}

#[derive(Default)]
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
            Ok(_) => Ok(Some(s)),
        }
    }
}

pub struct BufReadChars {
    r: Box<LineReader>,
    chars: Vec<char>,
    finished: bool,
    i: usize,
    initialized: bool,
}

impl BufReadChars {
    pub fn new(r: Box<LineReader>) -> BufReadChars {
        BufReadChars {
            r,
            chars: Vec::new(),
            finished: false,
            i: 0,
            initialized: false,
        }
    }

    fn refresh(&mut self) {
        match self.r.read_line().unwrap() {
            Some(line) => {
                self.chars = line.chars().collect();
                self.i = 0;
                self.initialized = true;
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
}

impl Iterator for BufReadChars {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        if !self.initialized {
            self.refresh();
            return self.next();
        }
        match self.next_char() {
            Some(c) => Some(c),
            None => {
                self.refresh();

                self.next()
            }
        }
    }
}
