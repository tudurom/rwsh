use crate::shell::Context;

pub fn r#true(_ctx: &mut Context, _args: Vec<&str>) -> i32 {
    0
}

pub fn r#false(_ctx: &mut Context, _args: Vec<&str>) -> i32 {
    1
}
