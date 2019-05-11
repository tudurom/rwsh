use crate::shell::State;

mod cd;
mod r#let;
pub use cd::cd;
pub use r#let::{r#let,unset};

type BuiltinFunc = fn(&mut State, Vec<&str>) -> i32;

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
    }
}
static BULTINS: [Builtin; 3] = [
    // keep sorted pls
    b!(cd),
    Builtin {
        name: "let",
        func: r#let,
    },
    b!(unset),
];

pub fn get_builtin(name: &str) -> Option<Builtin> {
    BULTINS
        .binary_search_by(|b| b.name.cmp(name))
        .ok()
        .map(|i| BULTINS[i])
}
