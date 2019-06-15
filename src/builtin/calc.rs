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
use crate::shell::Context;
use calc::eval;

pub fn calc(_ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut args = args.into_iter();
    args.next(); // skip name

    let code = args.map(String::from).collect::<Vec<String>>().join(" ");
    match eval(&code) {
        Ok(val) => println!("{}", val),
        Err(err) => {
            eprintln!("{}", err);
            return 1;
        }
    }
    0
}
