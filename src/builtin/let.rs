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
use crate::shell::Context;
use crate::shell::Var;
use getopts::Options;

fn is_special_var(s: &str) -> bool {
    s == "" || s == "?"
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: {} [options] key value\n       {0} [options] -e key",
        program
    );
    eprint!("{}", opts.usage(&brief));
}

pub fn r#let(ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("x", "", "export variable");
    opts.optflag("e", "", "erase variable");
    let matches = match opts.parse(args.iter()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("let: {}", e);
            print_usage(args[0], opts);
            return 2;
        }
    };
    if matches.free.len() < 2 {
        print_usage(args[0], opts);
        return 2;
    }

    if is_special_var(&matches.free[1]) {
        eprintln!("let: cannot change special variable");
        return 1;
    }

    macro_rules! wrong {
        ($nr:expr) => {
            if matches.free.len() != $nr {
                print_usage(args[0], opts);
                return 2;
            }
        };
    }

    if matches.opt_present("x") {
        if matches.opt_present("e") {
            wrong!(2);
            ctx.state.unexport_var(matches.free[1].clone());
        } else {
            wrong!(3);
            ctx.state
                .export_var(matches.free[1].clone(), matches.free[2].clone());
        }
    } else {
        if matches.opt_present("e") {
            wrong!(2);
            ctx.state.vars.remove(&matches.free[1]);
        } else {
            wrong!(3);
            ctx.state.vars.insert(
                matches.free[1].clone(),
                Var::String(matches.free[2].clone()),
            );
        }
    }

    0
}
