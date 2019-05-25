use super::*;
use crate::parser;
use crate::shell::{Context, Process};
use crate::sre::{Buffer, Invocation};
use nix::unistd;
use std::cell::RefCell;
use std::io::{stdin, stdout};
use std::rc::Rc;

pub struct SRESequence {
    seq: parser::SRESequence,
    started: bool,
    process: Option<Rc<RefCell<Process>>>,
}

impl SRESequence {
    pub fn new(seq: parser::SRESequence) -> Self {
        SRESequence {
            seq,
            started: false,
            process: None,
        }
    }

    fn process_start(&mut self, ctx: &mut Context) -> Result<(), String> {
        match unistd::fork().map_err(|e| format!("failed to fork: {}", e))? {
            unistd::ForkResult::Child => {
                let mut prev_address = None;
                let mut buf = Buffer::new(stdin()).unwrap();
                for prog in &self.seq.0 {
                    let inv = Invocation::new(prog.clone(), &buf, prev_address).unwrap();
                    let mut out = stdout();
                    let addr = inv.execute(&mut out, &mut buf).unwrap();
                    use std::io::Write;
                    out.flush().unwrap();
                    prev_address = Some(buf.apply_changes(addr));
                }
                std::process::exit(0);
            }
            unistd::ForkResult::Parent { child: pid, .. } => {
                self.process = Some(ctx.state.new_process(pid));
                Ok(())
            }
        }
    }
}

impl TaskImpl for SRESequence {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        ctx.state.if_condition_ok = None;
        if !self.started {
            self.process_start(ctx)?;
            self.started = true;
        }

        self.process.clone().unwrap().borrow_mut().poll()
    }
}
