use super::{Buffer, Range};
use crate::parser::sre::address::{ComposedAddress, SimpleAddress};

#[derive(Debug)]
pub enum AddressResolveError {
    OutOfRange,
    WrongOrder,
    NoMatch,
    RegexError(regex::Error),
}

impl std::fmt::Display for AddressResolveError {
    fn fmt(&self, w: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            w,
            "{}",
            match self {
                AddressResolveError::OutOfRange => "out of range".to_owned(),
                AddressResolveError::WrongOrder => "wrong order".to_owned(),
                AddressResolveError::NoMatch => "no match".to_owned(),
                AddressResolveError::RegexError(rerror) => format!("regex error: {}", rerror),
            }
        )
    }
}

impl std::error::Error for AddressResolveError {}

#[derive(Copy, Clone, Debug)]
/// An address is a chunk of a (../struct.Buffer.html).
pub struct Address<'a> {
    pub r: Range,
    pub buffer: &'a Buffer,
}

impl<'a> Address<'a> {
    pub fn new(buffer: &Buffer) -> Address {
        buffer.new_address(0, 0)
    }

    pub fn address(self, ca: ComposedAddress) -> Result<Self, AddressResolveError> {
        self.resolveAddress(Some(ca), 0)
    }

    fn lineAddress(self, line: usize, sign: i32) -> Result<Self, AddressResolveError> {
        let mut a = Address::new(self.buffer);

        let mut p = 0;
        let mut n = 0;

        if sign >= 0 {
            if line == 0 {
                // if we are either at the end or we are specifically
                // asking for the null line by being requesting the absolute
                // 0th line (sign == 0), then return the null line
                if sign == 0 || self.r.1 == 0 {
                    a.r = Range(0, 0);
                    return Ok(a);
                }
                // otherwise, the current line in its entirety will be selected
                // you will see this below
                a.r.0 = self.r.1;
                p = self.r.1 - 1;
            } else {
                if sign == 0 || self.r.1 == 0 {
                    p = 0;
                    // n = 1 means that we will skip the line we are on
                    // n is the iterator
                    n = 1;
                } else {
                    p = self.r.1 - 1;
                    // skip it if we are just at the start of the line
                    if self.buffer.data.as_bytes()[p] == '\n' as u8 {
                        n = 1;
                    } else {
                        n = 0;
                    }
                    p += 1;
                }
                // counting the lines
                while n < line {
                    if p >= self.buffer.data.len() {
                        return Err(AddressResolveError::OutOfRange);
                    }
                    if self.buffer.data.as_bytes()[p] == '\n' as u8 {
                        n += 1;
                    }
                    p += 1;
                }
                // we reached the start of the line
                a.r.0 = p;
            }
            // find the end of the line
            while p < self.buffer.data.len() && self.buffer.data.as_bytes()[p] != '\n' as u8 {
                p += 1;
            }
            a.r.1 = p;
            if p < self.buffer.data.len() {
                a.r.1 += 1;
            }
        } else {
            p = self.r.0;
            if line == 0 {
                // we are looking for the 0th line,
                // relative from where we are now, but backwards.
                // so we are actually looking for the end of the previous line
                a.r.1 = self.r.0;
            } else {
                n = 0;
                while n < line {
                    if p == 0 {
                        n += 1;
                        if n != line {
                            // we reached the beginning of the buffer
                            // and the search is not over
                            return Err(AddressResolveError::OutOfRange);
                        }
                    } else {
                        let c = self.buffer.data.as_bytes()[p - 1];
                        if c != '\n' as u8 || n + 1 != line {
                            p -= 1;
                        }
                        if c == '\n' as u8 {
                            n += 1;
                        }
                    }
                }
                a.r.1 = p;
                if p > 0 {
                    p -= 1;
                }
            }
            // lines start after a newline
            while p > 0 && self.buffer.data.as_bytes()[p - 1] != '\n' as u8 {
                p -= 1;
            }
            a.r.0 = p;
        }

        Ok(a)
    }

    fn charAddress(mut self, pos: usize, sign: i32) -> Result<Self, AddressResolveError> {
        if sign == 0 {
            self.r = Range(pos, pos);
        } else if sign < 0 {
            if self.r.0 < pos {
                return Err(AddressResolveError::OutOfRange);
            }
            self.r.0 -= pos;
            self.r.1 -= pos;
        } else {
            self.r.0 += pos;
            self.r.1 += pos;
        }
        if self.r.1 > self.buffer.data.len() {
            Err(AddressResolveError::OutOfRange)
        } else {
            Ok(self)
        }
    }

    fn regexAddress(self, re: &str, sign: i32) -> Result<Self, AddressResolveError> {
        let mut loc;
        let mut l: usize;
        let re = match regex::Regex::new(re) {
            Ok(re) => re,
            Err(e) => return Err(AddressResolveError::RegexError(e)),
        };
        if sign >= 0 {
            l = self.r.1;
            loc = match re.find(&self.buffer.data[l..]) {
                Some(x) => (x.start() + l, x.end() + l),
                None => (l, l),
            };

            if loc.0 == loc.1 && loc.0 == l {
                l += 1;
                if l > self.buffer.data.len() {
                    l = 0;
                }
                loc = match re.find(&self.buffer.data[l..]) {
                    Some(x) => (x.start() + l, x.end() + l),
                    None => return Err(AddressResolveError::NoMatch),
                }
            }
        } else {
            l = self.r.0;
            let mut locs: Vec<_> = re
                .find_iter(&self.buffer.data[..l])
                .map(|m| (m.start(), m.end()))
                .collect();
            if locs.is_empty() {
                locs.push((l, l));
            }
            loc = *locs.last().unwrap();
            if loc.0 == loc.1 && loc.0 == l {
                if l == 0 {
                    l = self.buffer.data.len();
                }
                locs = re
                    .find_iter(&self.buffer.data[..l])
                    .map(|m| (m.start(), m.end()))
                    .collect();
                if locs.is_empty() {
                    return Err(AddressResolveError::NoMatch);
                }
                loc = *locs.last().unwrap();
            }
        }
        Ok(Address {
            r: Range(loc.0, loc.1),
            buffer: self.buffer,
        })
    }

    fn resolveAddress(
        mut self,
        mut ca: Option<ComposedAddress>,
        mut sign: i32,
    ) -> Result<Self, AddressResolveError> {
        while let Some(a) = ca {
            match a.simple {
                SimpleAddress::Line(l) => self = self.lineAddress(l, sign)?,
                SimpleAddress::Char(c) => self = self.charAddress(c, sign)?,
                SimpleAddress::Dollar => {
                    self.r = Range(self.buffer.data.len(), self.buffer.data.len())
                }
                SimpleAddress::Dot => {}
                SimpleAddress::Regex(re, true) => {
                    let sign = if sign == 0 { -1 } else { -sign };
                    self = self.regexAddress(&re[1..], sign)?;
                }
                SimpleAddress::Regex(re, false) => self = self.regexAddress(&re[1..], sign)?,
                SimpleAddress::Comma | SimpleAddress::Semicolon => {
                    let mut a1: Address = Address::new(self.buffer);
                    let mut a2: Address = Address::new(self.buffer);
                    if a.left.is_some() {
                        a1 = self.resolveAddress(a.left.map(|x| *x), sign)?;
                    } else {
                        a1.buffer = self.buffer;
                        a1.r = Range(0, 0);
                    }
                    if a.next.is_some() {
                        a2 = self.resolveAddress(a.next.map(|x| *x), sign)?;
                    } else {
                        a2.buffer = self.buffer;
                        a2.r = Range(self.buffer.data.len(), self.buffer.data.len());
                    }
                    self.buffer = a1.buffer;
                    self.r = Range(a1.r.0, a2.r.1);
                    if self.r.1 < self.r.0 {
                        return Err(AddressResolveError::WrongOrder);
                    }
                    return Ok(self);
                }
                SimpleAddress::Plus => sign = 1,
                SimpleAddress::Minus => sign = -1,
                SimpleAddress::Nothing => panic!(),
            }

            ca = a.next.map(|x| *x);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::sre::Range;
    use crate::tests::common::{new_buffer, new_composed_address};
    #[test]
    fn smoke() {
        let buf = new_buffer("aaaa\nbbbb\ncccc\ndddd\n");
        let addr = super::Address::new(&buf)
            .address(new_composed_address("#3,3+#3"))
            .unwrap()
            .address(new_composed_address("-0+,+0-"))
            .unwrap();
        assert_eq!(addr.r, Range(5, 15));
    }
}
