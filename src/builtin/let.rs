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
 * You should have received a copy of the GNU General Public License
 * along with RWSH. If not, see <http://www.gnu.org/licenses/>.
 */
use crate::shell::Context;
use crate::shell::{Var, VarValue};
use getopts::Options;

fn is_special_var(s: &str) -> bool {
    s == "" || s == "?"
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: {} [options] key1 key2 ... keyN = value1 value 2 ... valueN\n       {0} [options] -e key",
        program
    );
    eprint!("{}", opts.usage(&brief));
}

#[derive(Copy, Clone)]
enum OperatorType {
    None,
    Arithmetic,
    Array,
}

#[derive(Copy, Clone)]
struct Operator {
    op: &'static str,
    typ: OperatorType,
}

macro_rules! op {
    ($op:expr, m) => {
        Operator {
            op: $op,
            typ: OperatorType::Arithmetic,
        }
    };
    ($op:expr, a) => {
        Operator {
            op: $op,
            typ: OperatorType::Array,
        }
    };
}

// keep sorted!
static OPERATORS: &'static [Operator] = &[
    op!("%=", m),
    op!("*=", m),
    op!("++=", a),
    op!("+=", m),
    op!("-=", m),
    op!("/=", m),
    op!("::=", a),
    Operator {
        op: "=",
        typ: OperatorType::None,
    },
];

fn get_operator(s: &str) -> Option<Operator> {
    if s.as_bytes()[s.len() - 1] != b'=' {
        return None;
    }
    OPERATORS
        .binary_search_by(|probe| probe.op.cmp(&s))
        .ok()
        .map(|i| OPERATORS[i])
}

enum Value<'a> {
    String(&'a str),
    Array(Vec<&'a str>),
}

impl<'a> Value<'a> {
    fn to_var(&self, key: String) -> Var {
        match self {
            Value::String(s) => Var::new(key, VarValue::Array(vec![(*s).to_owned()])),
            Value::Array(arr) => Var::new(
                key,
                VarValue::Array(arr.iter().map(|s| (*s).to_owned()).collect()),
            ),
        }
    }
}

struct KVReader<'a> {
    i: usize,
    args: &'a [String],
    op: Option<Operator>,
}

impl<'a> KVReader<'a> {
    fn new(args: &'a [String]) -> KVReader {
        KVReader {
            i: 1,
            args,
            op: None,
        }
    }

    fn read_keys(&mut self) -> Result<Vec<&'a str>, &'static str> {
        let mut keys: Vec<&'a str> = Vec::new();
        while self.i < self.args.len() {
            if let Some(op) = get_operator(&self.args[self.i]) {
                self.op = Some(op);
                break;
            }
            keys.push(&self.args[self.i]);
            self.i += 1;
        }
        if keys.is_empty() {
            return Err("missing keys");
        }
        Ok(keys)
    }

    fn operator(&mut self) -> Result<Operator, &'static str> {
        if self.i == self.args.len() {
            return Err("missing '=' operator");
        }
        self.i += 1;
        Ok(self.op.unwrap().clone())
    }

    fn read_raw_values(&mut self) -> Result<(Operator, Vec<&'a str>), &'static str> {
        let op = self.operator()?;
        let mut vals: Vec<&'a str> = Vec::new();
        while self.i < self.args.len() {
            vals.push(&self.args[self.i]);
            self.i += 1;
        }
        if vals.is_empty() {
            return Err("missing values");
        }
        Ok((op, vals))
    }

    fn read_values(&mut self) -> Result<(Operator, Vec<Value<'a>>), &'static str> {
        let (op, raw) = self.read_raw_values()?;
        let mut values = Vec::new();
        let mut arr = Vec::new();
        let mut in_array = false;
        for rv in raw {
            if rv == "[" {
                in_array = true;
            } else if in_array {
                if rv == "]" {
                    in_array = false;
                    values.push(Value::Array(arr.clone()));
                    arr.clear();
                } else {
                    arr.push(rv);
                }
            } else {
                values.push(Value::String(rv));
            }
        }
        if in_array {
            return Err("array literal left open");
        }
        Ok((op, values))
    }
}

#[allow(clippy::collapsible_if)]
pub fn r#let(ctx: &mut Context, args: Vec<&str>) -> i32 {
    let mut opts = Options::new();
    opts.optflag("x", "", "export variable");
    opts.optflag("e", "", "erase variable");
    opts.optflag("l", "", "create variable in the local scope");

    macro_rules! err {
        ($reason:expr) => {{
            eprintln!("let: {}", $reason);
            print_usage(args[0], opts);
            return 2;
        }};
    }

    let matches = match opts.parse(args.iter()) {
        Ok(m) => m,
        Err(e) => err!(e),
    };
    if (matches.opt_present("e") || matches.opt_present("l")) && matches.free.len() < 2 {
        err!("not enough arguments");
    }

    if matches.free.len() == 1 {
        if matches.opt_present("x") {
            for (k, v) in &ctx.state.exported_vars {
                println!("{}={}", k, v);
            }
        } else {
            for k in ctx.state.vars.keys() {
                println!("{}={}", k, ctx.state.get_var(k).unwrap());
            }
        }
        return 0;
    }

    if is_special_var(&matches.free[1]) {
        eprintln!("let: cannot change special variable");
        return 1;
    }

    let mut reader = KVReader::new(&matches.free);
    let keys = match reader.read_keys() {
        Ok(ks) => ks,
        Err(e) => err!(e),
    };
    let (op, vals) = if !matches.opt_present("e") {
        match reader.read_values() {
            Ok(vs) => {
                if keys.len() != vs.1.len() {
                    err!("number of keys doesn't match the number of values");
                }
                vs
            }
            Err(e) => err!(e),
        }
    } else {
        (
            Operator {
                op: "",
                typ: OperatorType::None,
            },
            vec![],
        )
    };
    if matches.opt_present("x") {
        if matches.opt_present("e") {
            for key in keys {
                ctx.state.unexport_var(key);
            }
        } else {
            for (key, val) in keys.into_iter().zip(vals.into_iter()) {
                let val = val.to_var(key.to_owned()).to_string();
                ctx.state.export_var(key.to_owned(), val);
            }
        }
    } else {
        if matches.opt_present("e") {
            for key in keys {
                ctx.state.remove_var(key);
            }
        } else {
            for (key, val) in keys.into_iter().zip(vals.into_iter()) {
                let left = ctx.state.get_var(key);
                if left.is_none() && op.op != "=" {
                    err!(format!("variable '{}' doesn't exist", key));
                }
                if op.op == "=" {
                    ctx.state.set_var(
                        key.to_owned(),
                        val.to_var(key.to_owned()),
                        matches.opt_present("l"),
                    );
                    continue;
                }
                match op.typ {
                    OperatorType::None => {}
                    OperatorType::Arithmetic => {
                        let left = left.unwrap();
                        macro_rules! to_int {
                            ($expr:expr) => {{
                                match $expr.parse::<i64>() {
                                    Ok(i) => i,
                                    Err(e) => err!(format!("'{}' is not a number: {}", $expr, e)),
                                }
                            }};
                        }
                        let right = match val {
                            Value::String(s) => match s.parse::<i64>() {
                                Ok(i) => i,
                                Err(e) => err!(format!("cannot use string on number: {}", e)),
                            },
                            Value::Array(_) => err!("cannot use array on number"),
                        };
                        match left.value {
                            VarValue::Array(mut left) => {
                                for v in &mut left {
                                    let mut i = to_int!(v);
                                    match op.op.as_bytes()[0] {
                                        b'%' => i %= right,
                                        b'*' => i *= right,
                                        b'+' => i += right,
                                        b'-' => i -= right,
                                        b'/' => i /= right,
                                        _ => panic!(),
                                    }
                                    *v = i.to_string();
                                }
                                ctx.state.set_var(
                                    key.to_owned(),
                                    Var::new(key.to_owned(), VarValue::Array(left)),
                                    matches.opt_present("l"),
                                );
                            }
                        }
                    }
                    OperatorType::Array => {
                        let left = left.unwrap();
                        let mut right = match val {
                            Value::String(s) => vec![s],
                            Value::Array(arr) => arr,
                        }
                        .into_iter()
                        .map(|s| s.to_owned())
                        .collect::<Vec<_>>();
                        match left.value {
                            VarValue::Array(mut left) => {
                                match &op.op[..=1] {
                                    "++" => left.append(&mut right),
                                    "::" => {
                                        let mut old_left = left.clone();
                                        left.clear();
                                        left.append(&mut right);
                                        left.append(&mut old_left);
                                    }
                                    _ => panic!(),
                                }
                                ctx.state.set_var(
                                    key.to_owned(),
                                    Var::new(key.to_owned(), VarValue::Array(left)),
                                    matches.opt_present("l"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    0
}
