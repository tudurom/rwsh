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

pub fn cd(_ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut dir;
    let home = dirs::home_dir().unwrap();
    if let Some(arg) = args.get(1) {
        dir = std::path::PathBuf::new();
        dir.push(arg);
    } else {
        dir = home;
    }
    if let Err(error) = std::env::set_current_dir(dir) {
        eprintln!("cd: {}", error);
        1
    } else {
        0
    }
}
