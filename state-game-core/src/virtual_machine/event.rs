use std::sync::Arc;
use crate::instruction::{Type, Value};

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

#[derive(Debug)]
pub enum VirtualMachineLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub struct VirtualMachineTrap {
    pub trapped_position: usize,
    pub reason: TrapReason
}

#[derive(Clone, Debug)]
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