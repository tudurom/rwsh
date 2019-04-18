use super::*;
use std::io::Write;

fn p(w: &mut Write, chars: &[char]) {
    for c in chars {
        write!(w, "{}", c).unwrap();
    }
}

pub struct P;

impl<'a> Command<'a> for P {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>> {
        p(w, &dot.buffer.data[dot.r.0..dot.r.1]);

        vec![*dot]
    }
}

#[cfg(test)]
mod tests {
    use crate::sre::Command;
    #[test]
    fn smoke() {
        let b = super::Buffer::new("xd lol".as_bytes()).unwrap();
        let addr = b.new_address(0, 2);
        let p = super::P;
        let mut w = Vec::new();
        p.execute(&mut w, &addr);
        assert_eq!(String::from_utf8_lossy(&w[..]), "xd");
    }
}
