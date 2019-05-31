use super::*;
use crate::parser;
use crate::shell::{self, Context, Process};
use nix::unistd;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::stdout;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::path::{Component, Path, PathBuf};
use std::process::exit;
use std::rc::Rc;

pub struct Word {
    word: parser::Word,
    expand_tilde: bool,

    started: bool,
    fd: RawFd,
    process: Option<Rc<RefCell<Process>>>,
}

impl Word {
    pub fn new(word: parser::Word, expand_tilde: bool) -> Self {
        Word {
            word,
            expand_tilde,
            started: false,
            fd: -1,
            process: None,
        }
    }

    fn start_command(&mut self, prog: parser::Program, ctx: &mut Context) -> Result<(), String> {
        let (in_pipe, out_pipe) =
            unistd::pipe().map_err(|e| format!("couldn't pipe command for substitution: {}", e))?;

        let fork_result = match unistd::fork() {
            Ok(x) => x,
            Err(e) => {
                unistd::close(in_pipe).unwrap();
                unistd::close(out_pipe).unwrap();
                return Err(format!("couldn't fork: {}", e));
            }
        };
        match fork_result {
            unistd::ForkResult::Child => {
                unistd::close(in_pipe).unwrap();
                unistd::dup2(out_pipe, stdout().as_raw_fd()).unwrap();
                unistd::close(out_pipe).unwrap();

                exit(
                    shell::run_program(prog, ctx.state)
                        .map_err(|e| {
                            format!(
                                "error while executing command for command substitution: {}",
                                e
                            )
                        })?
                        .0,
                );
            }
            unistd::ForkResult::Parent { child: pid, .. } => {
                unistd::close(out_pipe).unwrap();
                self.process = Some(ctx.state.new_process(pid));
                self.fd = in_pipe;
                Ok(())
            }
        }
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
    if s.is_empty() || s.as_bytes()[0] != b'~' {
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
        let mut program = None;
        let mut to_replace = None;
        use std::ops::DerefMut;
        if let parser::RawWord::String(ref mut s, _) = self.word.borrow_mut().deref_mut() {
            if self.expand_tilde {
                expand_tilde(s)?;
            }
            return Ok(TaskStatus::Success(0));
        }
        match self.word.borrow().deref() {
            parser::RawWord::Parameter(param) => {
                let mut val = ctx.get_parameter_value(&param.name);
                if val.is_none() {
                    val = Some(String::new());
                }
                //*self.word.borrow_mut()
                to_replace = Some(parser::RawWord::String(val.unwrap(), false));
            }
            parser::RawWord::Command(prog) => {
                program = Some(prog.clone());
            }
            _ => panic!(),
        }
        if let Some(prog) = program {
            if !self.started {
                self.start_command(prog, ctx)?;
                self.started = true;

                let mut buf = Vec::new();
                {
                    let mut f = unsafe { File::from_raw_fd(self.fd) };
                    use std::io::Read;
                    f.read_to_end(&mut buf)
                        .map_err(|e| format!("failed to read command output: {}", e))?;
                }
                self.fd = -1;

                // strip newlines
                let mut s = String::from_utf8(buf).unwrap();
                while s.ends_with('\n') {
                    s.pop();
                }

                *self.word.borrow_mut() = parser::RawWord::String(s, false);
            }

            return self.process.as_mut().unwrap().borrow_mut().poll();
        }
        *self.word.borrow_mut() = to_replace.unwrap();
        Ok(TaskStatus::Success(0))
    }
}
