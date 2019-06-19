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
