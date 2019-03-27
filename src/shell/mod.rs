use crate::parser::{ParseNode, Parser};
use crate::util::{BufReadChars, InteractiveLineReader, LineReader};
use dirs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

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
            if let Ok(ParseNode::Command(c)) = t {
                let parts = c.1.iter().map(|x| &x[..]);
                if let Err(error) = Self::run_command(&c.0, parts) {
                    eprintln!("{}", error);
                }
            } else if let Err(e) = t {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }

    fn do_cd<'a, I>(mut args: I) -> Result<(), String>
    where
        I: Iterator<Item = &'a str>,
    {
        let dir: &str;
        let home = dirs::home_dir().unwrap();
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

    fn run_command<'a, I>(command: &str, args: I) -> Result<(), String>
    where
        I: Iterator<Item = &'a str>,
    {
        match command {
            "cd" => Self::do_cd(args),
            _ => match Command::new(command).args(args).spawn() {
                Ok(mut child) => {
                    if let Err(error) = child.wait() {
                        return Err(format!("rwsh: {}", error));
                    }
                    Ok(())
                }
                Err(error) => Err(format!("rwsh: {}", error)),
            },
        }
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
            new_path.push(dirs::home_dir().unwrap());
            it.next();
        }
    }
    for p in it {
        new_path.push(p);
    }
    new_path
}
