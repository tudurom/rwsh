//! Utility functions for executing processes with pipes.
use nix;
use nix::sys::wait;
use nix::unistd::{self, ForkResult, Pid};
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::RawFd;
use std::process::exit;
use std::error::Error;

#[derive(Clone, Copy)]
/// A child process with its ID and `stdout` file descriptor
pub struct Child {
    pub pid: Pid,
    /// The child's `stdout` file descriptor.
    pub output: RawFd,
}

impl Child {
    /// Waits for the child to terminate.
    pub fn wait(self) -> nix::Result<()> {
        wait::waitpid(self.pid, None)?;
        Ok(())
    }
}

fn os2c(s: &OsStr) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_e| CString::new("<string-with-nul>").unwrap())
}

/// Provides a function that can be used as a process body for [`run_command`](fn.run_command.html) to execute external programs.
///
/// # Example
///
/// The following examples prints the text "Hello, world!" to `stdout` using the `echo` command:
///
/// ```rust
/// use rwsh::process::*;
/// run_command(exec("echo", vec!["Hello, world!"]), 0, true, true);
/// ```
pub fn exec<I, S, A>(prog: S, args: I) -> impl FnOnce()
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
        unistd::execvp(&c_args[0], &c_args).unwrap();
    }
}

/// Runs the code of the function `body` inside a new process that is eventualy piped to another.
///
/// # Example
///
/// ```rust
/// use rwsh::process::run_command;
///
/// run_command(move || {
///     println!("Hello, world!");
/// }, 0, true, true);
/// ```
pub fn run_command<F>(body: F, input: RawFd, first: bool, last: bool) -> Result<Child, Box<Error>>
where
    F: FnOnce(),
{
    let (read_pipe, write_pipe) = unistd::pipe()?;
    let pid: Pid;

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

            body();
            exit(0);
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

/// Helper for executing processes sequencially while piped.
pub struct PipeRunner {
    previous_command: Option<Child>,
    len: usize,
    i: usize,
}

impl PipeRunner {
    /// Creates a new, empty instance.
    pub fn new(len: usize) -> PipeRunner {
        PipeRunner {
            previous_command: None,
            len,
            i: 0,
        }
    }

    /// Runs the given `body` in a new process, piped to the previous one.
    ///
    /// If this is the first invocation, `stdin` will be the terminal's.
    /// If this is the last invocation, `stdout` will be the terminal's.
    pub fn run(&mut self, body: impl FnOnce()) -> Result<Child, Box<Error>> {
        let stdin = self
            .previous_command
            .map_or(0, |output: Child| output.output);

        self.previous_command = Some(run_command(
            body,
            stdin,
            self.i == 0,
            self.i == self.len - 1,
        )?);
        self.i += 1;
        Ok(self.previous_command.unwrap())
    }

    /// Waits for the last process to terminate.
    pub fn wait(&self) -> nix::Result<()> {
        if let Some(final_command) = self.previous_command {
            final_command.wait()?;
        }
        Ok(())
    }
}
