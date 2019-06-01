use crate::shell::Context;
use calc::eval;

pub fn calc(_ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut args = args.into_iter();
    args.next(); // skip name

    let mut code = args.map(String::from).collect::<Vec<String>>().join(" ");
    match eval(&code) {
        Ok(val) => println!("{}", val),
        Err(err) => {
            eprintln!("{}", err);
            return 1;
        }
    }
    0
}
