use crate::builtin;
use crate::parser;
use crate::shell::Context;
use crate::shell::Process;
use nix::sys::wait;
use nix::unistd;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::{CString, OsStr};
use std::io::{stdin, stdout};
use std::ops::Deref;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;

fn os2c(s: &OsStr) -> CString {
    CString::new(s.as_bytes()).unwrap_or_else(|_e| CString::new("<string-with-nul>").unwrap())
}

#[derive(Clone, Copy)]
pub enum TaskStatus {
    Wait,
    Success(i32),
}

/// A task defines any operation that the shell might have to schedule
/// and execute to get a final result. The task's behaviour is defined
/// by its [`TaskImpl`](trait.TaskImpl.html). The status of the task
/// is defined by [`TaskStatus`](enum.TaskStatus.html).
///
/// Task can have other nested task, such as functions.
pub struct Task {
    ti: Box<TaskImpl>,
    status: Result<TaskStatus, String>,
}

impl Task {
    pub fn new(ti: Box<TaskImpl>) -> Self {
        Task {
            ti,
            status: Ok(TaskStatus::Wait),
        }
    }

    pub fn run(&mut self, ctx: &mut Context) -> Result<i32, Box<Error>> {
        loop {
            let p = self.poll(ctx)?;
            if let TaskStatus::Success(code) = p {
                return Ok(code);
            }
            let stat = wait::waitpid(None, None)?;
            ctx.state.update_process(stat.pid().unwrap(), stat);
        }
    }

    /// Executes the implementation's polling function if still waiting.
    /// Returns the result without executing a second time if not waiting.
    pub fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if let Ok(TaskStatus::Wait) = self.status {
            self.status = self.ti.poll(ctx);
        }
        self.status.clone()
    }

    pub fn new_from_word(word: parser::Word, expand_tilde: bool) -> Self {
        if let parser::RawWord::List(children, double_quoted) = word.borrow().deref() {
            let mut tl = TaskList::new();
            for child in children {
                tl.children.push(Self::new_from_word(
                    child.clone(),
                    !double_quoted && expand_tilde,
                ));
            }
            return Task::new(Box::new(tl));
        }

        let tw = Word::new(word, expand_tilde);
        Task::new(Box::new(tw))
    }

    pub fn new_from_command(c: parser::Command) -> Self {
        let mut tl = TaskList::new();
        let sc = c.clone();
        tl.children.push(Self::new_from_word(sc.0.clone(), true));
        for arg in c.1 {
            tl.children.push(Self::new_from_word(arg, true));
        }
        tl.children.push(Task::new(Box::new(Command::new(sc))));

        Task::new(Box::new(tl))
    }

    pub fn new_from_sre_sequence(seq: parser::SRESequence) -> Self {
        unimplemented!()
    }

    pub fn new_from_pipeline(p: parser::Pipeline) -> Self {
        let mut tp = Pipeline::new();

        for pi in p.0 {
            tp.children.push(match pi {
                parser::Pipe::Command(c) => Self::new_from_command(c),
                parser::Pipe::SREProgram(seq) => Self::new_from_sre_sequence(seq),
            });
        }

        Self::new(Box::new(tp))
    }

    pub fn new_from_node(n: parser::Node) -> Self {
        match n {
            parser::Node::Pipeline(p) => Self::new_from_pipeline(p),
        }
    }

    pub fn new_from_command_lists(v: Vec<parser::CommandList>) -> Self {
        let mut tl = TaskList::new();

        for cl in v {
            let child = Self::new_from_node(cl.0);
            tl.children.push(child);
        }

        Self::new(Box::new(tl))
    }
}

/// Defines the behaviour of a [`Task`](struct.Task.html).
pub trait TaskImpl {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String>;
}

pub struct TaskList {
    children: Vec<Task>,
    current: usize,
}

impl TaskList {
    pub fn new() -> TaskList {
        TaskList {
            children: vec![],
            current: 0,
        }
    }
}

impl TaskImpl for TaskList {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let mut ret = Ok(TaskStatus::Wait);
        while self.current < self.children.len() {
            let child = &mut self.children[self.current];

            ret = child.poll(ctx);
            match ret {
                Ok(TaskStatus::Success(_)) => {}
                _ => return ret,
            }

            self.current += 1;
        }

        ret
    }
}

pub struct Pipeline {
    children: Vec<Task>,
    started: bool,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            children: vec![],
            started: false,
        }
    }

    fn start(&mut self, ctx: &mut Context) -> Result<(), String> {
        let mut dup_stdin = -1;
        let mut dup_stdout = -1;
        if self.children.len() > 1 {
            dup_stdin = unistd::dup(stdin().as_raw_fd())
                .map_err(|e| format!("failed to duplicate stdin: {}", e))?;
            dup_stdout = unistd::dup(stdout().as_raw_fd())
                .map_err(|e| format!("failed to duplicate stdout: {}", e))?;
        }

        let mut last_stdout = -1;
        let len = self.children.len();
        for (i, child) in self.children.iter_mut().enumerate() {
            if i > 0 {
                unistd::dup2(last_stdout, stdin().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdin: {}", e))?;
                unistd::close(last_stdout).unwrap();
            }

            let mut new_stdout = dup_stdout;
            if i < len - 1 {
                let (read_pipe, write_pipe) =
                    unistd::pipe().map_err(|e| format!("failed to pipe: {}", e))?;
                last_stdout = read_pipe;
                new_stdout = write_pipe;
            }

            if new_stdout >= 0 {
                unistd::dup2(new_stdout, stdout().as_raw_fd())
                    .map_err(|e| format!("failed to duplicate stdout: {}", e))?;
                unistd::close(new_stdout).unwrap();
            }

            child.poll(ctx)?;
        }

        if dup_stdin >= 0 {
            unistd::dup2(dup_stdin, stdin().as_raw_fd())
                .map_err(|e| format!("failed to duplicate stdin: {}", e))?;
            unistd::close(dup_stdin).unwrap();
        }

        Ok(())
    }
}

impl TaskImpl for Pipeline {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.started {
            self.start(ctx)?;
            self.started = true;
        }

        let mut ret = Ok(TaskStatus::Success(0));
        for child in self.children.iter_mut() {
            ret = child.poll(ctx);
            match ret {
                Ok(TaskStatus::Success(_)) => {}
                _ => return ret,
            }
        }

        ret
    }
}

pub struct Command {
    cmd: parser::Command,
    started: bool,
    args: Vec<String>,
    t: CommandType,
    process: Option<Rc<RefCell<Process>>>,
}

pub enum CommandType {
    Process,
    Builtin,
}

fn word_to_str(w: parser::Word) -> String {
    match w.borrow().deref() {
        parser::RawWord::String(s, _) => s.to_string(),
        parser::RawWord::List(ws, _) => {
            let mut s = String::new();
            for w in ws {
                s.push_str(&word_to_str(w.clone()));
            }
            s
        }
        _ => panic!(),
    }
}

impl Command {
    pub fn new(cmd: parser::Command) -> Self {
        Command {
            cmd,
            started: false,
            args: Vec::new(),
            t: CommandType::Process,
            process: None,
        }
    }

    fn process_start(&mut self, ctx: &mut Context) -> Result<(), String> {
        match unistd::fork().map_err(|e| format!("failed to fork: {}", e))? {
            unistd::ForkResult::Child => {
                if let Err(e) = unistd::execvp(
                    &os2c(OsStr::new(&self.args[0].as_str())),
                    self.args
                        .iter()
                        .map(|a| os2c(OsStr::new(&a)))
                        .collect::<Vec<CString>>()
                        .as_slice(),
                ) {
                    eprintln!("{}: {}", self.args[0], e);
                    std::process::exit(127);
                }
                Ok(())
            }
            unistd::ForkResult::Parent { child: pid, .. } => {
                self.process = Some(ctx.state.new_process(pid));
                Ok(())
            }
        }
    }

    fn process_poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.started {
            self.process_start(ctx)?;
            self.started = true;
        }

        self.process.clone().unwrap().borrow_mut().poll()
    }

    fn builtin_poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        unimplemented!()
    }

    fn get_args(&mut self, ctx: &Context) {
        self.args.push(word_to_str(self.cmd.0.clone()));
        for arg in &self.cmd.1 {
            self.args.push(word_to_str(arg.clone()));
        }
    }
}

impl TaskImpl for Command {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        if !self.started {
            self.get_args(ctx);
            self.t = if builtin::get_builtin(&self.args[0]).is_some() {
                CommandType::Builtin
            } else {
                CommandType::Process
            };
        }

        match self.t {
            CommandType::Process => self.process_poll(ctx),
            CommandType::Builtin => self.builtin_poll(ctx),
        }
    }
}

pub struct Word {
    word: parser::Word,
    expand_tilde: bool,
}

impl Word {
    pub fn new(word: parser::Word, expand_tilde: bool) -> Self {
        Word { word, expand_tilde }
    }
}

impl TaskImpl for Word {
    fn poll(&mut self, ctx: &mut Context) -> Result<TaskStatus, String> {
        let mut to_replace;
        match self.word.borrow().deref() {
            parser::RawWord::String(s, _) => return Ok(TaskStatus::Success(0)),
            parser::RawWord::Parameter(param) => {
                let mut val = ctx.get_parameter_value(&param.name);
                if val.is_none() {
                    val = Some(String::new());
                }
                //*self.word.borrow_mut()
                to_replace = Some(parser::RawWord::String(val.unwrap(), false));
            }
            _ => panic!(),
        }
        *self.word.borrow_mut() = to_replace.unwrap();
        Ok(TaskStatus::Success(0))
    }
}
