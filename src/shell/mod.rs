use crate::parser::{Parser, Pipeline, Task};
use crate::process::{self, PipeRunner};
use crate::sre::{Buffer, Invocation};
use crate::util::{BufReadChars, InteractiveLineReader, LineReader};
use nix::unistd;
use std::env;
use std::io::{stdin, stdout};
use std::path::{Path, PathBuf};
use std::process::exit;

/// The shell engine with its internal state.
///
/// Use it with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html) to get an interactive shell.
pub struct Shell<R: LineReader> {
    p: Parser<R>,
}

impl Shell<InteractiveLineReader> {
    /// Create a new `Shell` with an [`InteractiveLineReader`](../util/struct.InteractiveLineReader.html).
    pub fn new_interactive() -> Shell<InteractiveLineReader> {
        Self::new(InteractiveLineReader::new())
    }
}

impl<R: LineReader> Shell<R> {
    /// Returns a new `Shell` with the given [`LineReader`](../util/trait.LineReader.html).
    pub fn new(r: R) -> Shell<R> {
        let buf = BufReadChars::new(r);
        let p = Parser::new(buf);
        Shell { p }
    }

    /// Start the REPL.
    pub fn run(&mut self) {
        for t in self.p.by_ref() {
            if let Ok(p) = t {
                if let Err(error) = Self::run_pipeline(p) {
                    eprintln!("{}", error);
                }
            } else if let Err(e) = t {
                eprintln!("{}", e);
                exit(1);
            }
        }
    }

    /// `cd` builtin
    fn do_cd<'a, I>(mut args: I) -> Result<(), String>
    where
        I: Iterator<Item = &'a str>,
    {
        let dir: &str;
        let home = unistd::getcwd().unwrap();
        if let Some(arg) = args.next() {
            dir = arg;
        } else {
            dir = home.to_str().unwrap();
        }
        let path = expand_home(dir);
        match env::set_current_dir(path) {
            Err(error) => Err(format!("cd: {}", error)),
            _ => Ok(()),
        }
    }

    /// Runs a [Pipeline](../parser/struct.Pipeline.html)
    fn run_pipeline(p: Pipeline) -> Result<(), String> {
        let mut runner = PipeRunner::new(p.0.len());
        for task in p.0.iter() {
            match task {
                Task::Command(command) => match &command.0 as &str {
                    "cd" => {
                        return Self::do_cd(command.1.iter().map(|x| &x[..]));
                    }
                    name => {
                        if let Err(e) = runner.run(process::exec(name, command.1.iter())) {
                            return Err(format!("rwsh: {}", e));
                        }
                    }
                },
                Task::SREProgram(p) => {
                    let mut prev_address = None;
                    runner
                        .run(move || {
                            let mut buf = Buffer::new(stdin()).unwrap();
                            for prog in &p.0 {
                                let inv =
                                    Invocation::new(prog.clone(), &buf, prev_address).unwrap();
                                let mut out = Box::new(stdout());
                                let addr = inv.execute(&mut out, &mut buf).unwrap();
                                use std::io::Write;
                                out.flush().unwrap();
                                prev_address = Some(buf.apply_changes(addr));
                            }
                        })
                        .unwrap();
                }
            }
        }

        if let Err(e) = runner.wait() {
            return Err(format!("rwsh: {}", e));
        }
        Ok(())
    }
}

impl Default for Shell<InteractiveLineReader> {
    fn default() -> Self {
        Self::new_interactive()
    }
}

/// Expands the ~ at the beginning of a path to the user's home directory.
fn expand_home<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut new_path = PathBuf::new();
    let mut it = path.as_ref().iter().peekable();

    if let Some(p) = it.peek() {
        if *p == "~" {
            new_path.push(unistd::getcwd().unwrap());
            it.next();
        }
    }
    for p in it {
        new_path.push(p);
    }
    new_path
}
