use crate::shell::Context;

pub fn exit(ctx: &mut Context, args: Vec<&str>) -> i32 {
    if args.len() > 2 {
        eprintln!("exit: Usage:\nexit [code]");
        return 1;
    }

    if args.len() == 2 {
        match args[1].parse::<i32>() {
            Ok(i) => ctx.state.exit = i,
            Err(_) => {
                eprintln!("exit: exit code not an integer");
                return 1;
            }
        }
    } else {
        ctx.state.exit = 0;
    }
    0
}
