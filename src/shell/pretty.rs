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
/// A node in the pretty print tree.
pub struct PrettyTree {
    pub text: String,
    pub children: Vec<PrettyTree>,
}

/// All parse types should be able to be pretty-printed.
pub trait PrettyPrint {
    fn pretty_print(&self) -> PrettyTree;
}

impl PrettyTree {
    /// Pretty prints the tree.
    pub fn print(&self) {
        self.print_tree("".to_owned(), true);
    }

    fn print_tree(&self, prefix: String, last: bool) {
        let current_prefix = if last { "└─ " } else { "├─ " };

        println!("{}{}{}", prefix, current_prefix, self.text);

        let child_prefix = if last { "   " } else { "│  " };
        let prefix = prefix + child_prefix;

        if !self.children.is_empty() {
            let last_child = self.children.len() - 1;

            for (i, child) in self.children.iter().enumerate() {
                child.print_tree(prefix.to_owned(), i == last_child);
            }
        }
    }
}
