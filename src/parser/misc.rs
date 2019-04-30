use crate::util::{BufReadChars, LineReader};

/// Reads a regular expression until it reaches a delimiter.
pub fn read_regexp<R: LineReader>(it: &mut BufReadChars<R>, delimiter: char) -> (String, bool) {
    let mut s = String::new();
    let mut closed = false;

    while let Some(&c) = it.peek() {
        if c == delimiter {
            closed = true;
            break;
        } else if c == '\\' {
            it.next();
            match it.peek() {
                Some('\\') => {
                    s.push_str("\\\\");
                }
                Some(&x @ '/') | Some(&x @ '?') => {
                    if x != delimiter {
                        s.push('\\');
                    }
                    s.push(x);
                }
                Some(&x) => {
                    s.push('\\');
                    s.push(x);
                }
                None => {}
            }
        } else {
            s.push(c);
        }
        it.next();
    }

    (s, closed)
}
