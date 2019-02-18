use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::process::Command;
use std::str::CharIndices;

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        print!("> ");
        stdout().flush()?;
        let mut line = String::new();
        stdin().read_line(&mut line).unwrap();
        let mut parts = Args::new(line.trim());
        let command = parts.next().unwrap();
        let args = parts;
        run_command(command, args).unwrap();
    }
}

fn run_command<'a, I>(command: &str, args: I) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a str>,
{
    let mut child = Command::new(command).args(args).spawn()?;
    child.wait()?;
    Ok(())
}

pub struct Args<'a> {
    base: &'a str,
    it: CharIndices<'a>,
}

impl<'a> Args<'a> {
    pub fn new(base: &'a str) -> Args<'a> {
        Args {
            base,
            it: base.char_indices(),
        }
    }
}

impl<'a> Iterator for Args<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let mut l: i32 = -1;
        let mut r: i32 = -2;
        let mut in_quote = '\0';
        let mut escaping = false;
        while let Some((i, c)) = self.it.next() {
            if in_quote == '\0' && (c == '\'' || c == '"') {
                in_quote = c;
                continue;
            }
            if !escaping && c.is_whitespace() {
                if r < l {
                    continue;
                }
                break;
            }
            if l == -1 {
                l = i as i32;
                r = l - 1;
            }
            if in_quote != '\0' {
                if escaping {
                    r += 1;
                    escaping = false;
                } else if c != in_quote {
                    if c == '\\' {
                        escaping = true;
                    } else {
                        r += 1;
                    }
                } else {
                    in_quote = '\0';
                    break;
                }
            } else {
                if escaping {
                    r += 1;
                    escaping = false;
                } else if c == '\\' {
                    escaping = true;
                } else {
                    r += 1;
                }
            }
        }
        if in_quote != '\0' || escaping || r < l {
            return None;
        }
        Some(&self.base[l as usize..=r as usize])
    }
}
