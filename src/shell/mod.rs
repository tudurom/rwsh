use crate::parser::lex::Lexer;
use crate::parser::{ParseNode, Parser};
use crate::util::{BufReadChars, InteractiveLineReader};
use dirs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

pub struct Shell {
    p: Parser,
}

impl Shell {
    pub fn new() -> Shell {
        let ilr = InteractiveLineReader::new();
        let r = BufReadChars::new(Box::new(ilr));
        let l = Lexer::new(r);
        let p = Parser::new(l);
        Shell { p }
    }

    pub fn run(&mut self) {
        for t in self.p.by_ref() {
            if let Ok(ParseNode::Command(c)) = t {
                let parts = c.1.iter().map(|x| &x[..]);
                if let Err(error) = Shell::run_command(&c.0, parts) {
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
            "cd" => Shell::do_cd(args),
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

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

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
