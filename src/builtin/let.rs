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

fn is_special_var(s: &str) -> bool {
    s == "" || s == "?"
}

pub fn r#let(ctx: &mut Context, args: Vec<&str>) -> i32 {
    if args.len() != 3 {
        eprintln!("let: Usage:\nlet <key> <value>");
        return 1;
    }

    if is_special_var(args[1]) {
        eprintln!("let: cannot change special variable");
        return 1;
    }

    ctx.state
        .set_var(args[1].to_owned(), Var::String(args[2].to_owned()));
    0
}

pub fn unset(ctx: &mut Context, args: Vec<&str>) -> i32 {
    if args.len() != 2 {
        eprintln!("unset: Usage:\nunset <key>");
        return 1;
    }

    if is_special_var(args[1]) {
        eprintln!("unset: cannot change special variable");
        return 1;
    }

    ctx.state.vars.remove(args[1]);
    0
}
