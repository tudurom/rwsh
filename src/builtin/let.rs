use crate::shell::State;
use crate::shell::Var;

pub fn r#let(state: &mut State, args: Vec<&str>) -> i32 {
    if args.len() != 3 {
        eprintln!("let: Usage:\nlet <key> <value>");
        return 1;
    }

    state.set_var(args[1].to_owned(), Var::String(args[2].to_owned()));
    0
}

pub fn unset(state: &mut State, args: Vec<&str>) -> i32 {
    if args.len() != 2 {
        eprintln!("unset: Usage:\nunset <key>");
        return 1;
    }

    state.vars.remove(args[1]);
    0
}
