use crate::shell::State;

pub fn cd(_state: &mut State, args: Vec<&str>) -> i32 {
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
