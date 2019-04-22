use crate::parser::{escape, skip_whitespace};
use crate::sre::{commands, SimpleCommand};
use crate::util::{BufReadChars, LineReader, ParseError};
use std::collections::LinkedList;

fn arg_nr(name: char) -> i8 {
    match name {
        'p' => 0,
        'a' => 1,
        'Z' => 3, // for debugging
        _ => -1,
    }
}

fn build_command<'a>(name: char, mut args: LinkedList<String>) -> Box<dyn SimpleCommand<'a>> {
    match name {
        'p' => Box::new(commands::P),
        'a' => Box::new(commands::A(args.pop_front().unwrap())),
        _ => unimplemented!(),
    }
}

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

fn read_command<R: LineReader>(
    it: &mut BufReadChars<R>,
) -> Result<(char, LinkedList<String>), ParseError> {
    skip_whitespace(it);
    let chr = it.next();
    let nr = chr.map_or(-1, |name| arg_nr(name));
    let mut args = LinkedList::new();
    match chr {
        Some(name) if nr != -1 => {
            let mut i = 0;
            while i < nr && it.peek() == Some(&'/') {
                args.push_back(read_arg(it)?);
                i += 1;
            }
            if i < nr {
                let peek = it.peek().map(|c| *c);
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
                        Ok((name, args))
                    } else {
                        Err(it.new_error("missing terminal '/' in parameter".to_owned()))
                    }
                } else {
                    Ok((name, args))
                }
            }
        }
        Some(c) => Err(it.new_error(format!(
            "unexpected character '{}' when reading command name",
            c
        ))),
        None => Err(it.new_error("unexpected EOF when reading command".to_owned())),
    }
}

pub fn parse_simple_command<'a, R: LineReader>(
    it: &mut BufReadChars<R>,
) -> Result<Box<dyn SimpleCommand>, ParseError> {
    read_command(it).map(|c| build_command(c.0, c.1))
}

#[cfg(test)]
mod tests {
    use crate::tests::common::new_dummy_buf;
    use std::collections::LinkedList;

    // it's like vec! but for linked lists.
    // pretty much a hack
    macro_rules! ll {
        ( $( $x:expr ),* ) => {
            {
                #[allow(unused_mut)]
                let mut temp_list = LinkedList::new();
                $(
                    temp_list.push_back($x);
                )*
                temp_list
            }
        }
    }

    #[test]
    fn smoke() {
        assert_eq!(
            super::parse_simple_command(&mut new_dummy_buf("p".lines()))
                .unwrap()
                .to_tuple(),
            ('p', ll![])
        );
        assert_eq!(
            super::parse_simple_command(&mut new_dummy_buf("a/xd/".lines()))
                .unwrap()
                .to_tuple(),
            ('a', ll!["xd".to_owned()])
        );
    }

    #[test]
    fn read_command() {
        let mut buf = new_dummy_buf("Z/xd  &$\\n#@\\/xd/\\tlol    /ke.k\\//".lines());
        assert_eq!(
            super::read_command(&mut buf),
            Ok((
                'Z',
                ll![
                    "xd  &$\n#@/xd".to_owned(),
                    "\tlol    ".to_owned(),
                    "ke.k/".to_owned()
                ]
            ))
        );
    }
}
