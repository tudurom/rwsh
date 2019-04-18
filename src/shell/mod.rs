use crate::parser::{Parser, Pipeline};
use crate::util::process::{self, Child};
use crate::util::{BufReadChars, InteractiveLineReader, LineReader};
use nix::unistd;
use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::ffi::OsStr;

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
        let mut previous_command = None;
        let len = p.0.len();
        for (i, command) in p.0.iter().enumerate() {
            match &command.0 as &str {
                "cd" => {
                    return Self::do_cd(command.1.iter().map(|x| &x[..]));
                }
                name => {
                    let stdin = previous_command.map_or(0, |output: Child| output.output);

                    match process::run_command(
                        process::exec(name, command.1.iter()),
                        stdin,
                        i == 0,
                        i == len - 1,
                    ) {
                        Ok(child) => {
                            previous_command = Some(child);
                        }
                        Err(e) => {
                            return Err(format!("rwsh: {}", e));
                        }
                    }
                }
            }
        }

        if let Some(mut final_command) = previous_command {
            if let Err(e) = final_command.wait() {
                return Err(format!("rwsh: {}", e));
            }
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
