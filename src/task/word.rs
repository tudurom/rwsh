use super::*;
use crate::parser;
use crate::shell::Context;
use std::ffi::{CStr, CString};
use std::path::{Component, Path, PathBuf};

pub struct Word {
    word: parser::Word,
    expand_tilde: bool,
}

impl Word {
    pub fn new(word: parser::Word, expand_tilde: bool) -> Self {
        Word { word, expand_tilde }
    }
}

fn get_pw_dir(user: &str) -> Result<PathBuf, String> {
    unsafe {
        nix::errno::Errno::clear();
        let p = libc::getpwnam(CString::new(user).unwrap().as_c_str().as_ptr());
        if p.is_null() {
            if nix::errno::errno() == 0 {
                Err("couldn't get home dir: no such user".to_owned())
            } else {
                Err(format!(
                    "couldn't get home dir: {}",
                    nix::errno::Errno::last().desc(),
                ))
            }
        } else {
            let mut buf = PathBuf::new();
            let dir = CStr::from_ptr((*p).pw_dir);
            buf.push(dir.to_str().unwrap());
            Ok(buf)
        }
    }
}

fn expand_tilde(s: &mut String) -> Result<(), String> {
    if s.len() == 0 || s.as_bytes()[0] != b'~' {
        return Ok(());
    }
    let mut buf = PathBuf::new();
    let mut components = Path::new(&s[1..]).components().peekable();
    match components.peek() {
        None => buf.push(dirs::home_dir().unwrap()),
        Some(p) => {
            if let Component::RootDir = p {
                buf.push(dirs::home_dir().unwrap());
            } else {
                buf.push(get_pw_dir(p.as_os_str().to_str().unwrap())?);
            }
            components.next();
        }
    }
    for c in components {
        buf.push(c);
    }
    *s = buf.to_str().unwrap().to_owned();
    Ok(())
}

impl TaskImpl for Word {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let mut to_replace;
        use std::ops::DerefMut;
        match self.word.borrow_mut().deref_mut() {
            parser::RawWord::String(ref mut s, _) => {
                if self.expand_tilde {
                    expand_tilde(s)?;
                }
                return Ok(TaskStatus::Success(0));
            }
            parser::RawWord::Parameter(param) => {
                let mut val = ctx.get_parameter_value(&param.name);
                if val.is_none() {
                    val = Some(String::new());
                }
                //*self.word.borrow_mut()
                to_replace = Some(parser::RawWord::String(val.unwrap(), false));
            }
            _ => panic!(),
        }
        *self.word.borrow_mut() = to_replace.unwrap();
        Ok(TaskStatus::Success(0))
    }
}
