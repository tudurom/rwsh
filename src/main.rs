use clap::{App, Arg};
use nix::unistd;
use rwsh::shell::{Config, Shell};
use rwsh::util::FileLineReader;
use std::fs::File;
use std::io::stdin;

fn main() {
    let matches = App::new("rwsh")
        .version("v0.0.0")
        .author("Tudor-Ioan Roman")
        .arg(Arg::with_name("input").help("input script").index(1))
        .arg(
            Arg::with_name("n")
                .short("n")
                .help("pretty print AST instead of executing"),
        )
        .get_matches();

    let cfg = Config {
        pretty_print: matches.is_present("n"),
    };
    if let Some(input) = matches.value_of("input") {
        Shell::new(
            FileLineReader::new(File::open(input).unwrap()).unwrap(),
            cfg,
        )
        .run();
    } else if unistd::isatty(0).unwrap() {
        Shell::new_interactive(cfg).run();
    } else {
        Shell::new(FileLineReader::new(stdin()).unwrap(), cfg).run();
    }
}
