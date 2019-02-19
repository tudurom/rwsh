use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        print!("> ");
        stdout().flush()?;
        let mut line = String::new();
        stdin().read_line(&mut line).unwrap();
        let mut _parts = parse_line(line.trim());
        let mut parts = _parts.iter().map(|x| &x[..]);
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

fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}

fn parse_line(base: &str) -> Vec<String> {
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
