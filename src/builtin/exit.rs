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
