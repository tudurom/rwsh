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
use crate::util::BufReadChars;

/// Reads a regular expression until it reaches a delimiter.
pub fn read_regexp(it: &mut BufReadChars, delimiter: char) -> (String, bool) {
    let mut s = String::new();
    let mut closed = false;

    while let Some(&c) = it.peek() {
        if c == delimiter {
            closed = true;
            break;
        } else if c == '\\' {
            it.next();
            match it.peek() {
                Some('\\') => {
                    s.push_str("\\\\");
                }
                Some(&x @ '/') | Some(&x @ '?') => {
                    if x != delimiter {
                        s.push('\\');
                    }
                    s.push(x);
                }
                Some(&x) => {
                    s.push('\\');
                    s.push(x);
                }
                None => {}
            }
        } else {
            s.push(c);
        }
        it.next();
    }

    (s, closed)
}
