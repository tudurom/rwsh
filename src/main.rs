use rwsh::shell::Shell;

fn main() {
    let mut sh = Shell::new_interactive();
    sh.run();
}
