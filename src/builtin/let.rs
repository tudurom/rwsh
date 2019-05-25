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
