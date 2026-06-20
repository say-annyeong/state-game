use std::collections::HashMap;
use std::sync::Arc;
use crate::virtual_machine::instruction::{FunctionIdentifier, Slot};
use crate::virtual_machine::instruction_verifier::VerifyError;
use crate::virtual_machine::types::{Type, Value};

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
    VerifierBug(String),
}

pub struct StateChange {
    pub identifier: String,
    pub old: Option<Arc<Value>>,
    pub new: Option<Arc<Value>>,
}

pub struct VirtualMachineCallEvent {
    pub self_identifier: FunctionIdentifier,
    pub function_identifier: FunctionIdentifier,
    pub input: HashMap<Slot, Arc<Value>>,
    pub output: HashMap<Slot, Arc<Value>>
}

impl VirtualMachineCallEvent {
    pub fn new(self_identifier: FunctionIdentifier, function_identifier: FunctionIdentifier, input: HashMap<Slot, Arc<Value>>, output: HashMap<Slot, Arc<Value>>) -> Self {
        Self { self_identifier, function_identifier, input, output }
    }
}

pub enum VirtualMachineYield {
    Call {
        function_identifier: FunctionIdentifier,
        inputs: HashMap<Slot, Arc<Value>>,
        outputs: Vec<Slot>,
    },

    Finished,
}