use std::sync::Arc;
use crate::instruction_run::types::{Type, Value};

pub(super) enum VirtualMachineEvent {
    Log(VirtualMachineLog),
    Trap(VirtualMachineTrap),
    StateChange(StateChange),
    ExecutionFinished,
}

pub(super) struct VirtualMachineLog {
    pub(super) level: VirtualMachineLogLevel,
    pub(super) message: String,
}

#[derive(Debug)]
pub(super) enum VirtualMachineLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
pub(super) struct VirtualMachineTrap {
    pub(super) trapped_position: usize,
    pub(super) reason: TrapReason,
}

/// Runtime-only failures — conditions the verifier cannot rule out statically.
#[derive(Clone, Debug)]
pub(super) enum TrapReason {
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

pub(super) struct StateChange {
    pub(super) identifier: String,
    pub(super) old: Option<Arc<Value>>,
    pub(super) new: Option<Arc<Value>>,
}