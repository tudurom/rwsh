pub mod lex;

pub fn parse_line(base: &str) -> Vec<String> {
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

fn escape(c: char) -> char {
    match c {
        'n' => '\n',
        't' => '\t',
        'a' => '\x07',
        'b' => '\x08',
        _ => c,
    }
}
