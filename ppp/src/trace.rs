use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum TraceEvent {
    FunctionCall { name: String },
    FunctionReturn { name: String },
    VariableUpdate { name: String, value: String },
}
