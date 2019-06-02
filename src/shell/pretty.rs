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
