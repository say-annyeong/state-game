use std::sync::Arc;
use crate::state_game_instruction::{Type, Value};

pub enum VirtualMachineEvent {
    Log(VirtualMachineLog),
    Trap(VirtualMachineTrap),
    StateChange(StateChange),
    ExecutionFinished
}

pub struct VirtualMachineLog {
    pub level: VirtualMachineLogLevel,
    pub message: String,
}

pub enum VirtualMachineLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub struct VirtualMachineTrap {
    pub trapped_position: usize,
    pub label: Option<String>,
    pub reason: TrapReason
}

pub enum TrapReason {
    UnwrapNone,
    InvalidIdentifier(String),
    InvalidJump(String),
    TypeMismatch {
        expected: Type,
        actual: Type,
    }
}

pub struct StateChange {
    pub identifier: String,
    pub old: Option<Arc<Value>>,
    pub new: Option<Arc<Value>>
}