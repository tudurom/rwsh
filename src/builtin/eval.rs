use crate::parser::Parser;
use crate::shell::{self, Context};
use crate::util::{BufReadChars, FileLineReader};

pub fn eval(ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut args = args.into_iter();
    args.next(); // skip name

    let mut code = args.map(String::from).collect::<Vec<String>>().join(" ");
    code.push('\n');

    let reader = BufReadChars::new(FileLineReader::new(code.as_bytes()).unwrap());
    let mut parser = Parser::new(reader);
    let prog = parser.next().unwrap();

    if let Ok(prog) = prog {
        if prog.0.is_empty() {
            return 0;
        }
        match shell::run_program(prog, ctx.state) {
            Ok(status) => status.0,
            Err(error) => {
                eprintln!("{}", error);
                1
            }
        }
    } else {
        eprintln!("{}", prog.err().unwrap());
        1
    }
}
