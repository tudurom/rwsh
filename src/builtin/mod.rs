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

mod calc;
mod cd;
mod eval;
mod exit;
mod r#let;
mod r#true;
use self::calc::calc;
use cd::cd;
use eval::eval;
use exit::exit;
use r#let::{r#let, unset};
use r#true::{r#false, r#true};

type BuiltinFunc = fn(&mut Context, Vec<&str>) -> i32;

#[derive(Clone, Copy)]
pub struct Builtin {
    pub name: &'static str,
    pub func: BuiltinFunc,
}

macro_rules! b {
    ($name:ident) => {
        Builtin {
            name: stringify!($name),
            func: $name,
        }
    };
}
static BULTINS: [Builtin; 8] = [
    // keep sorted pls
    b!(calc),
    b!(cd),
    b!(eval),
    b!(exit),
    Builtin {
        name: "false",
        func: r#false,
    },
    Builtin {
        name: "let",
        func: r#let,
    },
    Builtin {
        name: "true",
        func: r#true,
    },
    b!(unset),
];

pub fn get_builtin(name: &str) -> Option<Builtin> {
    BULTINS
        .binary_search_by(|b| b.name.cmp(name))
        .ok()
        .map(|i| BULTINS[i])
}
