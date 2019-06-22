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
use super::{skip_whitespace, Command};
use crate::parser::lex::LexMode;
use crate::parser::{Parser, Word};
use crate::util::ParseError;

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
fn read_arg(p: &mut Parser) -> Result<Word, ParseError> {
    skip_whitespace(p, true);
    p.parse_word_delimited('/')
}

fn read_regex_arg(p: &mut Parser) -> Result<Word, ParseError> {
    skip_whitespace(p, true);
    p.parse_word_pattern(false)
}

#[derive(Debug, PartialEq)]
/// A simple command is a command without any address.
/// It has a list of slash-delimited arguments and an optional command argument, for commands such as `x` and `g`,
/// that do an action (the command) based on a condition.
pub struct SimpleCommand {
    pub name: char,
    pub args: Vec<Word>,
    pub command_args: Vec<Command>,
}

/// Parses the whole command. If the command accepts a command argument, the argument is recursively parsed too.
pub fn parse_command(p: &mut Parser, brace: bool) -> Result<Option<SimpleCommand>, ParseError> {
    skip_whitespace(p, false);

    let chr = p.next_char();
    let nr = chr.map_or(-1, arg_nr);
    let mut args = Vec::new();
    p.lexer.borrow_mut().mode.insert(LexMode::SLASH);
    let r = match chr {
        Some(name) if nr != -1 => {
            let mut i = 0;
            while i < nr && p.peek_char() == Some('/') {
                let arg = if i == 0 && has_regex_argument(name) {
                    read_regex_arg(p)?
                } else {
                    read_arg(p)?
                };
                args.push(arg);
                i += 1;
            }
            if i < nr {
                let peek = p.peek_char();
                if peek.is_none() {
                    Err(p.new_error("unexpected EOF when reading argument".to_owned()))
                } else {
                    Err(p.new_error(format!(
                        "unexpected character '{}' when reading argument",
                        peek.unwrap()
                    )))
                }
            } else {
                if nr > 0 {
                    if let Some('/') = p.peek_char() {
                        p.next_char();
                    } else {
                        return Err(p.new_error("missing terminal '/' in parameter".to_owned()));
                    }
                }
                let command_args = if has_command_argument(name) {
                    p.ps2_enter(format!("{}", name));
                    let r = vec![super::parse_command(p, false)?.unwrap()];
                    p.ps2_exit();
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
            p.ps2_enter("{".to_owned());
            let mut x = super::parse_command(p, true);
            let mut command_args = Vec::new();
            while let Ok(Some(c)) = x {
                command_args.push(c);
                x = super::parse_command(p, true);
            }
            p.ps2_exit();
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
        Some(c) => Err(p.new_error(format!(
            "unexpected character '{}' when reading command name",
            c
        ))),
        None => Err(p.new_error("unexpected EOF when reading command".to_owned())),
    };
    p.lexer.borrow_mut().mode.remove(LexMode::SLASH);
    r
}

#[cfg(test)]
mod tests {
    use crate::parser::{Parser, RawWord};
    use crate::tests::common::new_dummy_buf;

    macro_rules! word {
        ($s:expr) => {
            RawWord::List(vec![RawWord::String($s.to_owned(), false).into()], true).into()
        };
    }

    #[test]
    fn smoke() {
        assert_eq!(
            super::parse_command(&mut Parser::new(new_dummy_buf("p".lines())), false)
                .unwrap()
                .unwrap(),
            super::SimpleCommand {
                name: 'p',
                args: vec![],
                command_args: vec![],
            }
        );
        assert_eq!(
            super::parse_command(&mut Parser::new(new_dummy_buf("a/xd/".lines())), false)
                .unwrap()
                .unwrap(),
            super::SimpleCommand {
                name: 'a',
                args: vec![word!("xd")],
                command_args: vec![],
            }
        );
    }

    #[test]
    fn command_arg() {
        let v = super::parse_command(
            &mut Parser::new(new_dummy_buf("x/lmao/ 1x/kek/ 2a/xd/".lines())),
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(v.command_args.clone()[0].name, 'x');
        assert_eq!(v.command_args.clone()[0].command_args[0].name, 'a');
        assert!(v.command_args[0].command_args[0].command_args.is_empty());
    }

    #[test]
    fn many_string_arguments() {
        use crate::parser::WordParameter;
        let mut p = Parser::new(new_dummy_buf(
            "Z/xd  &$\\n#@\\/xd/\\tlol    /ke.k\\//".lines(),
        ));
        assert_eq!(
            super::parse_command(&mut p, false).unwrap().unwrap(),
            super::SimpleCommand {
                name: 'Z',
                args: vec![
                    RawWord::List(
                        vec![
                            RawWord::String("xd  &".to_owned(), false).into(),
                            RawWord::Parameter(WordParameter {
                                name: "\n".to_owned()
                            })
                            .into(),
                            RawWord::String("#@/xd".to_owned(), false).into(),
                        ],
                        true
                    )
                    .into(),
                    word!("\tlol    "),
                    word!("ke.k/"),
                ],
                command_args: vec![],
            }
        );
    }
}
