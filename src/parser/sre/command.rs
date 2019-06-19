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
use super::Command;
use crate::parser::{escape, skip_whitespace};
use crate::util::{BufReadChars, ParseError};

fn arg_nr(name: char) -> i32 {
    match name {
        'p' => 0,

        'a' | 'c' | 'i' => 1,
        'd' => 0,

        'g' | 'v' => 1,
        'x' | 'y' => 1,

        '=' => 0,

        'Z' => 3, // for debugging
        _ => -1,
    }
}

/// It is identical to `has_command_argument`, for now.
fn has_regex_argument(name: char) -> bool {
    let s = ['g', 'v', 'x', 'y'];
    s.binary_search(&name).is_ok()
}

fn has_command_argument(name: char) -> bool {
    let s = ['g', 'v', 'x', 'y'];
    s.binary_search(&name).is_ok()
}

#[allow(clippy::collapsible_if)]
fn read_arg(it: &mut BufReadChars) -> Result<String, ParseError> {
    skip_whitespace(it, false);
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

fn read_regex_arg(it: &mut BufReadChars) -> Result<String, ParseError> {
    skip_whitespace(it, false);
    it.next(); // /
    let (s, closed) = crate::parser::misc::read_regexp(it, '/');
    if closed {
        Ok(s)
    } else {
        Err(it.new_error("unexpected EOF while reading regexp".to_owned()))
    }
}

#[derive(Debug, PartialEq)]
/// A simple command is a command without any address.
/// It has a list of slash-delimited arguments and an optional command argument, for commands such as `x` and `g`,
/// that do an action (the command) based on a condition.
pub struct SimpleCommand {
    pub name: char,
    pub args: Vec<String>,
    pub command_args: Vec<Command>,
}

/// Parses the whole command. If the command accepts a command argument, the argument is recursively parsed too.
pub fn parse_command(
    it: &mut BufReadChars,
    brace: bool,
) -> Result<Option<SimpleCommand>, ParseError> {
    skip_whitespace(it, true);
    let chr = it.next();
    let nr = chr.map_or(-1, arg_nr);
    let mut args = Vec::new();
    match chr {
        Some(name) if nr != -1 => {
            let mut i = 0;
            while i < nr && it.peek() == Some(&'/') {
                let arg = if i == 0 && has_regex_argument(name) {
                    read_regex_arg(it)?
                } else {
                    read_arg(it)?
                };
                args.push(arg);
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
                let command_args = if has_command_argument(name) {
                    it.ps2_enter(format!("{}", name));
                    let r = vec![super::parse_command(it, false)?.unwrap()];
                    it.ps2_exit();
                    r
                } else {
                    vec![]
                };
                Ok(Some(SimpleCommand {
                    name,
                    args,
                    command_args,
                }))
            }
        }
        Some('{') => {
            it.ps2_enter("{".to_owned());
            let mut x = super::parse_command(it, true);
            let mut command_args = Vec::new();
            while let Ok(Some(c)) = x {
                command_args.push(c);
                x = super::parse_command(it, true);
            }
            it.ps2_exit();
            if x.is_err() {
                Err(x.err().unwrap())
            } else {
                Ok(Some(SimpleCommand {
                    name: '{',
                    args: Vec::new(),
                    command_args,
                }))
            }
        }
        Some('}') if brace => Ok(None),
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
            super::parse_command(&mut new_dummy_buf("p".lines()), false)
                .unwrap()
                .unwrap(),
            super::SimpleCommand {
                name: 'p',
                args: vec![],
                command_args: vec![],
            }
        );
        assert_eq!(
            super::parse_command(&mut new_dummy_buf("a/xd/".lines()), false)
                .unwrap()
                .unwrap(),
            super::SimpleCommand {
                name: 'a',
                args: vec!["xd".to_owned()],
                command_args: vec![],
            }
        );
    }

    #[test]
    fn command_arg() {
        let v = super::parse_command(&mut new_dummy_buf("x/lmao/ 1x/kek/ 2a/xd/".lines()), false)
            .unwrap()
            .unwrap();
        assert_eq!(v.command_args.clone()[0].name, 'x');
        assert_eq!(v.command_args.clone()[0].command_args[0].name, 'a');
        assert!(v.command_args[0].command_args[0].command_args.is_empty());
    }

    #[test]
    fn many_string_arguments() {
        let mut buf = new_dummy_buf("Z/xd  &$\\n#@\\/xd/\\tlol    /ke.k\\//".lines());
        assert_eq!(
            super::parse_command(&mut buf, false).unwrap().unwrap(),
            super::SimpleCommand {
                name: 'Z',
                args: vec![
                    "xd  &$\n#@/xd".to_owned(),
                    "\tlol    ".to_owned(),
                    "ke.k/".to_owned()
                ],
                command_args: vec![],
            }
        );
    }
}
