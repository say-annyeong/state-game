use std::sync::Arc;
use crate::instruction_run::types::{Type, Value};

pub enum VirtualMachineEvent {
    Log(VirtualMachineLog),
    Trap(VirtualMachineTrap),
    StateChange(StateChange),
    ExecutionFinished,
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

#[derive(Clone, Debug)]
pub struct VirtualMachineTrap {
    pub trapped_position: usize,
    pub reason: TrapReason,
}

/// Runtime-only failures — conditions the verifier cannot rule out statically.
#[derive(Clone, Debug)]
pub enum TrapReason {
    /// `UnwrapSome` on a `None` value.
    UnwrapNone,
    /// `UnwrapOk` on an `Err` value.
    UnwrapErrOnOk,
    /// `UnwrapErr` on an `Ok` value.
    UnwrapOkOnErr,
    /// Integer division or modulo by zero.
    DivisionByZero,
    /// Vector index out of bounds.
    IndexOutOfBounds { index: i64, length: usize },
    /// `StringGetChar` index out of bounds.
    StringIndexOutOfBounds { index: i64, length: usize },
    VerifierBug
}

pub struct StateChange {
    pub identifier: String,
    pub old: Option<Arc<Value>>,
    pub new: Option<Arc<Value>>,
}