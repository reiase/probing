use std::collections::HashMap;

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
        ip: usize,
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

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Value {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}
