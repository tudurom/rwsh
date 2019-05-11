use crate::shell::State;
use std::process;

pub fn exit(_state: &mut State, args: Vec<&str>) -> i32 {
    if args.len() > 2 {
        eprintln!("exit: Usage:\nexit [code]");
        return 1;
    }

    if args.len() == 2 {
        match args[1].parse::<i32>() {
            Ok(i) => process::exit(i),
            Err(_) => {
                eprintln!("exit: exit code not an integer");
                return 1;
            },
        }
    } else {
        process::exit(0);
    }
}
