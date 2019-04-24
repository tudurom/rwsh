use super::Command;
use crate::parser::{escape, skip_whitespace};
use crate::util::{BufReadChars, LineReader, ParseError};

fn arg_nr(name: char) -> i8 {
    match name {
        'p' => 0,

        'a' | 'c' | 'i' => 1,
        'd' => 0,

        'g' | 'v' => 1,
        'x' | 'y' => 1,

        'Z' => 3, // for debugging
        _ => -1,
    }
}

fn has_command_argument(name: char) -> bool {
    let s = ['g', 'v', 'x', 'y'];
    s.binary_search(&name).is_ok()
}

#[allow(clippy::collapsible_if)]
fn read_arg<R: LineReader>(it: &mut BufReadChars<R>) -> Result<String, ParseError> {
    skip_whitespace(it);
    it.next(); // /
    let mut s = String::new();
    let mut escaping = false;
    while let Some(&c) = it.peek() {
        if escaping {
            escaping = false;
            s.push(escape(c));
        } else {
            if c == '/' {
                break;
            } else if c == '\\' {
                escaping = true;
            } else {
                s.push(c);
            }
        }
        it.next();
    }
    if escaping {
        Err(it.new_error("unexpected EOF while escaping".to_owned()))
    } else {
        Ok(s)
    }
}

#[derive(Debug, PartialEq)]
/// A simple command is a command without any address.
/// It has a list of slash-delimited arguments and an optional command argument, for commands such as `x` and `g`,
/// that do an action (the command) based on a condition.
pub struct SimpleCommand {
    pub name: char,
    pub args: Vec<String>,
    pub command_arg: Option<Box<Command>>,
}

/// Parses the whole command. If the command accepts a command argument, the argument is recursively parsed too.
pub fn parse_command<R: LineReader>(it: &mut BufReadChars<R>) -> Result<SimpleCommand, ParseError> {
    skip_whitespace(it);
    let chr = it.next();
    let nr = chr.map_or(-1, arg_nr);
    let mut args = Vec::new();
    match chr {
        Some(name) if nr != -1 => {
            let mut i = 0;
            while i < nr && it.peek() == Some(&'/') {
                args.push(read_arg(it)?);
                i += 1;
            }
            if i < nr {
                let peek = it.peek().cloned();
                if peek.is_none() {
                    Err(it.new_error("unexpected EOF when reading argument".to_owned()))
                } else {
                    Err(it.new_error(format!(
                        "unexpected character '{}' when reading argument",
                        peek.unwrap()
                    )))
                }
            } else {
                if nr > 0 {
                    if let Some(&'/') = it.peek() {
                        it.next();
                    } else {
                        return Err(it.new_error("missing terminal '/' in parameter".to_owned()));
                    }
                }
                let command_arg = if has_command_argument(name) {
                    Some(super::parse_command(it)?)
                } else {
                    None
                };
                Ok(SimpleCommand {
                    name,
                    args,
                    command_arg: command_arg.map(Box::new),
                })
            }
        }
        Some(c) => Err(it.new_error(format!(
            "unexpected character '{}' when reading command name",
            c
        ))),
        None => Err(it.new_error("unexpected EOF when reading command".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::common::new_dummy_buf;

    #[test]
    fn smoke() {
        assert_eq!(
            super::parse_command(&mut new_dummy_buf("p".lines())).unwrap(),
            super::SimpleCommand {
                name: 'p',
                args: vec![],
                command_arg: None
            }
        );
        assert_eq!(
            super::parse_command(&mut new_dummy_buf("a/xd/".lines())).unwrap(),
            super::SimpleCommand {
                name: 'a',
                args: vec!["xd".to_owned()],
                command_arg: None
            }
        );
    }

    #[test]
    fn command_arg() {
        let v = super::parse_command(&mut new_dummy_buf("x/lmao/ 1x/kek/ 2a/xd/".lines())).unwrap();
        assert_eq!(v.command_arg.clone().unwrap().name, 'x');
        assert_eq!(
            v.command_arg.clone().unwrap().command_arg.unwrap().name,
            'a'
        );
        assert!(v
            .command_arg
            .unwrap()
            .command_arg
            .unwrap()
            .command_arg
            .is_none());
    }

    #[test]
    fn many_string_arguments() {
        let mut buf = new_dummy_buf("Z/xd  &$\\n#@\\/xd/\\tlol    /ke.k\\//".lines());
        assert_eq!(
            super::parse_command(&mut buf).unwrap(),
            super::SimpleCommand {
                name: 'Z',
                args: vec![
                    "xd  &$\n#@/xd".to_owned(),
                    "\tlol    ".to_owned(),
                    "ke.k/".to_owned()
                ],
                command_arg: None
            }
        );
    }
}
