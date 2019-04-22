use super::*;
use std::io::Write;

fn p(w: &mut Write, chars: &[char]) {
    for c in chars {
        write!(w, "{}", c).unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub struct P;

impl<'a> SimpleCommand<'a> for P {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>> {
        p(w, &dot.buffer.data[dot.r.0..dot.r.1]);

        vec![*dot]
    }
    fn to_tuple(&self) -> (char, LinkedList<String>) {
        ('p', LinkedList::new())
    }
}

#[derive(Debug, PartialEq)]
pub struct A(pub String);

impl<'a> SimpleCommand<'a> for A {
    fn execute(&self, w: &mut Write, dot: &'a Address) -> Vec<Address<'a>> {
        unimplemented!()
    }

    fn to_tuple(&self) -> (char, LinkedList<String>) {
        let mut list = LinkedList::new();
        list.push_back(self.0.clone());
        ('a', list)
    }
}

#[cfg(test)]
mod tests {
    use crate::sre::SimpleCommand;
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
