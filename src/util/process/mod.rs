use nix;
use nix::sys::wait;
use nix::unistd::{self, ForkResult, Pid};
use std::error::Error;
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::process::exit;

pub struct Child {
    pub pid: Pid,
    pub output: RawFd,
}

impl Child {
    pub fn wait(&self) -> Result<(), nix::Error> {
        wait::waitpid(self.pid, None)?;
        Ok(())
    }
}

fn os2c(s: &OsStr) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_e| CString::new("<string-with-nul>").unwrap())
}

pub fn exec<I, S, A>(prog: S, args: I) -> impl FnOnce() -> Result<(), nix::Error>
where
    S: AsRef<OsStr>,
    A: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
{
    let prog = os2c(prog.as_ref());
    let mut c_args: Vec<CString> = vec![prog];
    for arg in args {
        c_args.push(os2c(arg.as_ref()));
    }
    move || {
        unistd::execvp(&c_args[0], &c_args)?;
        Ok(())
    }
}

pub fn run_command<F, E>(body: F, input: RawFd, first: bool, last: bool) -> nix::Result<Child>
where
    F: FnOnce() -> Result<(), E>,
    E: Error,
{
    let (read_pipe, write_pipe) = unistd::pipe()?;
    let mut pid: Pid = Pid::this();

    match unistd::fork()? {
        ForkResult::Parent { child, .. } => pid = child,
        ForkResult::Child => {
            if first && !last && input == 0 {
                unistd::dup2(write_pipe, 1)?;
            } else if !first && !last && input != 0 {
                unistd::dup2(input, 0)?;
                unistd::dup2(write_pipe, 1)?;
            } else {
                unistd::dup2(input, 0)?;
            }

            if body().is_err() {
                exit(1);
            }
        }
    }

    if input != 0 {
        unistd::close(input)?;
    }

    unistd::close(write_pipe)?;
    if last {
        unistd::close(read_pipe)?;
    }

    Ok(Child {
        pid,
        output: read_pipe,
    })
}
