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
use crate::parser::Parser;
use crate::shell::{self, Context};
use crate::util::{BufReadChars, FileLineReader};
use std::io::Cursor;

pub fn eval(ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut args = args.into_iter();
    args.next(); // skip name

    let mut code = args.map(String::from).collect::<Vec<String>>().join(" ");
    code.push('\n');

    let reader = BufReadChars::new(Box::new(FileLineReader::new(Cursor::new(code)).unwrap()));
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
