use rwsh::parser::lex::Lexer;
use rwsh::parser::Parser;
use rwsh::shell::Shell;
use std::io::{stdin, Read};

fn main() {
    let sh = Shell::new();
    sh.run();
    /*
    let mut s = String::new();
    stdin().read_to_string(&mut s).unwrap();
    let l = Lexer::new(&s);
    let p = Parser::new(l);
    for t in p {
        println!("{:?}", t);
    }
    */
}
