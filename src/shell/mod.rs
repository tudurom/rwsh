use crate::parser::parse_line;
use std::io::{self, stdin, stdout, Write};
use std::process::{exit, Command};

#[derive(Default)]
pub struct Shell;

impl Shell {
    pub fn new() -> Shell {
        Shell
    }

    pub fn run(&self) {
        loop {
            print!("> ");
            stdout().flush().unwrap();
            let mut line = String::new();
            if stdin().read_line(&mut line).unwrap() == 0 {
                exit(0);
            }
            let mut _parts = parse_line(line.trim());
            let mut parts = _parts.iter().map(|x| &x[..]);
            let command = parts.next();
            if command.is_none() {
                continue;
            }
            let args = parts;
            match self.run_command(command.unwrap(), args) {
                Err(ref error) if error.kind() == io::ErrorKind::NotFound => {
                    eprintln!("Command not found");
                }
                Err(error) => {
                    panic!(error);
                }
                _ => {}
            }
        }
    }
    fn run_command<'a, I>(&self, command: &str, args: I) -> io::Result<()>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut child = Command::new(command).args(args).spawn()?;
        child.wait()?;
        Ok(())
    }
}
