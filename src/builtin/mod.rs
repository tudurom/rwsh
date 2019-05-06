use crate::shell::State;

type BuiltinFunc = fn(&mut State, Vec<&str>) -> i32;

#[derive(Clone, Copy)]
pub struct Builtin {
    pub name: &'static str,
    pub func: BuiltinFunc,
}

static BULTINS: [Builtin; 1] = [
    // keep sorted pls
    Builtin {
        name: "cd",
        func: cd,
    },
];

pub fn get_builtin(name: &str) -> Option<Builtin> {
    BULTINS
        .binary_search_by(|b| b.name.cmp(name))
        .ok()
        .map(|i| BULTINS[i])
}

fn cd(_state: &mut State, args: Vec<&str>) -> i32 {
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
