use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::types::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Process {
    pub pid: i32,
    pub exe: String,
    pub env: String,
    pub cmd: String,
    pub cwd: String,
    pub main_thread: u64,
    pub threads: Vec<u64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum CallFrame {
    CFrame {
        ip: String,
        file: String,
        func: String,
        lineno: i64,
    },
    PyFrame {
        file: String,
        func: String,
        lineno: i64,
        locals: HashMap<String, Value>,
    },
}

impl Display for CallFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CallFrame::CFrame {
                ip,
                file,
                func,
                lineno,
            } => {
                write!(f, "[C/C++] {ip}, file: {file}:{lineno}\n\t{func}\n")
            }
            CallFrame::PyFrame {
                file,
                func,
                lineno,
                locals,
            } => {
                write!(f, "[Python] file: {file}:{lineno} func: {func}\n")?;
                write!(f, "\tlocals:\n")?;
                for (k, v) in locals {
                    write!(f, "\t\t{}: {}\n", k, v)?;
                }
                Ok(())
            }
        }
    }
}
