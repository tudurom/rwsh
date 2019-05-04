use clap::{App, Arg};
use nix::unistd;
use rwsh::shell::Shell;
use rwsh::util::FileLineReader;
use std::fs::File;
use std::io::stdin;

fn main() {
    let matches = App::new("rwsh")
        .version("v0.0.0")
        .author("Tudor-Ioan Roman")
        .arg(Arg::with_name("input").help("input script").index(1))
        .get_matches();

    if let Some(input) = matches.value_of("input") {
        Shell::new(FileLineReader::new(File::open(input).unwrap()).unwrap()).run();
    } else if unistd::isatty(0).unwrap() {
        Shell::new_interactive().run();
    } else {
        Shell::new(FileLineReader::new(stdin()).unwrap()).run();
    }
}
