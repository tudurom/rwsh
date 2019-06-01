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
