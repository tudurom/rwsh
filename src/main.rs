/* Copyright (C) 2019 Tudor-Ioan Roman
 *
 * This file is part of the Really Weird Shell, also known as RWSH.
 *
 * RWSH is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * RWSH is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with RWSH. If not, see <http://www.gnu.org/licenses/>.
 */
use getopts::Options;
use nix::unistd;
use rwsh::shell::{Config, Shell};
use rwsh::util::FileLineReader;
use std::env;
use std::fs::File;
use std::io::stdin;
use std::process::exit;

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "rwsh v{}\nUsage: {} [options] [file]",
        env!("CARGO_PKG_VERSION"),
        program
    );
    eprint!("{}", opts.usage(&brief));
}

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let mut opts = Options::new();
    opts.optflag("n", "", "pretty print AST instead of executing");
    opts.optflag("h", "help", "print this help message");
    let matches = match opts.parse(args.iter()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("rwsh: {}", e);
            print_usage(&args[0], opts);
            exit(2);
        }
    };
    if matches.opt_present("h") {
        print_usage(&matches.free[0], opts);
        return;
    }

    let cfg = Config {
        pretty_print: matches.opt_present("n"),
    };
    if let Some(input) = matches.free.get(1) {
        Shell::new(
            Box::new(FileLineReader::new(File::open(input).unwrap()).unwrap()),
            cfg,
        )
        .run();
    } else if unistd::isatty(0).unwrap() {
        Shell::new_interactive(cfg).run();
    } else {
        Shell::new(Box::new(FileLineReader::new(stdin()).unwrap()), cfg).run();
    }
}
