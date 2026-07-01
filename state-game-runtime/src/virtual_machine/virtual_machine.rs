use std::{
    collections::HashMap,
    ops::Deref,
    sync::{Arc, RwLock, TryLockError},
};

use crossbeam_channel::{Receiver, Sender};
use dashmap::DashMap;

use state_game_core::{Identifier, Namespace, helper::try_until};

use crate::virtual_machine::{
    event::{
        StateChange, TrapReason, VirtualMachineCallEvent, VirtualMachineEvent, VirtualMachineLog,
        VirtualMachineLogLevel, VirtualMachineTrap, VirtualMachineYield
    },
    instruction::{FunctionIdentifier, Functions, Instruction, Literal, Slot, SpecialFunctions},
    types::{Type, Value},
};
// ── Instruction pointer step ─────────────────────────────────────────────────

enum ExecutionResult {
    /// Advance by 1.
    Advance,
    /// Jump to an absolute position (already verified to be in-bounds).
    Goto(usize),

    YieldCall {
        function_identifier: FunctionIdentifier,
        inputs: HashMap<Slot, Arc<Value>>,
        destination_slots: Vec<Slot>,
        source_slots: Vec<Slot>,
    },
}

// ── Virtual Machine ──────────────────────────────────────────────────────────

/// A VirtualMachine is designed to support multiple concurrent instances.
/// Each instance operates independently with its own Instruction set.
///
/// Execution always creates and runs a new VirtualMachine instance.
/// Instances are allowed to execute in parallel unless an explicit
/// dependency relationship is declared.
///
/// Dependency management is the responsibility of the caller.
/// Any race conditions, ordering issues, or other bugs resulting from
/// undeclared dependencies are considered developer errors and are not
/// attributed to the VirtualMachine implementation.
pub struct VirtualMachine {
    pub logger_sender: Sender<VirtualMachineEvent>,
    pub scheduler_sender: Sender<VirtualMachineCallEvent>,
    pub scheduler_receiver: Receiver<VirtualMachineCallEvent>,
    pub self_identifier: FunctionIdentifier,
    pub instruction_pointer: usize,
    pub instructions: Arc<[Instruction]>,
    pub slots: HashMap<Slot, Arc<Value>>,
    pub global_memory: Arc<RwLock<DashMap<(Namespace, Identifier), Value>>>,
    pub modification_namespace_list: Arc<[Namespace]>,
    pub input_slots: HashMap<Slot, Arc<Value>>,
    pub output_slots: HashMap<Slot, Arc<Value>>,
}

impl VirtualMachine {
    pub fn new(
        logger_sender: Sender<VirtualMachineEvent>,
        scheduler_sender: Sender<VirtualMachineCallEvent>,
        scheduler_receiver: Receiver<VirtualMachineCallEvent>,
        self_identifier: FunctionIdentifier,
        instructions: Arc<[Instruction]>,
        global_memory: Arc<RwLock<DashMap<(Namespace, Identifier), Value>>>,
        modification_namespace_list: Arc<[Namespace]>,
    ) -> Self {
        Self::with_instruction_pointer(
            logger_sender,
            scheduler_sender,
            scheduler_receiver,
            self_identifier,
            instructions,
            0,
            global_memory,
            modification_namespace_list,
        )
    }

    pub fn with_instruction_pointer(
        logger_sender: Sender<VirtualMachineEvent>,
        scheduler_sender: Sender<VirtualMachineCallEvent>,
        scheduler_receiver: Receiver<VirtualMachineCallEvent>,
        self_identifier: FunctionIdentifier,
        instructions: Arc<[Instruction]>,
        instruction_pointer: usize,
        global_memory: Arc<RwLock<DashMap<(Namespace, Identifier), Value>>>,
        modification_namespace_list: Arc<[Namespace]>,
    ) -> Self {
        Self {
            logger_sender,
            scheduler_sender,
            scheduler_receiver,
            self_identifier,
            instruction_pointer,
            instructions,
            slots: HashMap::new(),
            global_memory,
            modification_namespace_list,
            input_slots: HashMap::new(),
            output_slots: HashMap::new(),
        }
    }

    pub fn call_function(
        &self,
        scheduler_sender: Sender<VirtualMachineCallEvent>,
        scheduler_receiver: Receiver<VirtualMachineCallEvent>,
        self_identifier: FunctionIdentifier,
        instruction_pointer: usize,
        instructions: Arc<[Instruction]>,
        input_slots: HashMap<Slot, Arc<Value>>,
    ) -> Self {
        Self {
            logger_sender: self.logger_sender.clone(),
            scheduler_sender,
            scheduler_receiver,
            self_identifier,
            instruction_pointer,
            instructions,
            slots: HashMap::new(),
            global_memory: self.global_memory.clone(),
            modification_namespace_list: self.modification_namespace_list.clone(),
            input_slots,
            output_slots: HashMap::new(),
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn emit(&self, event: VirtualMachineEvent) {
        _ = self.logger_sender.send(event);
    }

    fn log(&self, level: VirtualMachineLogLevel, message: impl Into<String>) {
        self.emit(VirtualMachineEvent::Log(VirtualMachineLog {
            level,
            message: message.into(),
        }));
    }

    fn trap(&self, reason: TrapReason) -> VirtualMachineTrap {
        VirtualMachineTrap {
            trapped_position: self.instruction_pointer,
            reason,
        }
    }

    fn slot_name(slot: Slot) -> String {
        format!("slot_{slot}")
    }

    /// Read a slot. Returns `Err(VerifierBug)` if the slot was never written —
    /// this indicates the instruction stream was not verified before execution.
    fn read(&self, slot: Slot) -> Result<Arc<Value>, VirtualMachineTrap> {
        self.slots
            .get(&slot)
            .cloned()
            .ok_or_else(|| self.trap(TrapReason::VerifierBug("Unbound Slot".to_string())))
    }

    fn write(&mut self, slot: Slot, value: Arc<Value>) {
        let old = self.slots.insert(slot, value.clone());
        self.emit(VirtualMachineEvent::StateChange(StateChange {
            identifier: Self::slot_name(slot),
            old,
            new: Some(value),
        }));
    }

    // ── Main loop ─────────────────────────────────────────────────────────────

    pub fn run_until_yield(&mut self) -> Result<VirtualMachineYield, VirtualMachineTrap> {
        self.log(VirtualMachineLogLevel::Info, "Virtual Machine Resume");

        while self.instruction_pointer < self.instructions.len() {
            let instr = self.instructions[self.instruction_pointer].clone();

            match self.execute(&instr) {
                Ok(ExecutionResult::Advance) => {
                    self.instruction_pointer += 1;
                }

                Ok(ExecutionResult::Goto(target)) => {
                    self.instruction_pointer = target;
                }

                Ok(ExecutionResult::YieldCall {
                    function_identifier,
                    inputs,
                    destination_slots,
                    source_slots,
                }) => {
                    return Ok(VirtualMachineYield::Call {
                        function_identifier,
                        inputs,
                        destination_slots,
                        source_slots,
                    });
                }

                Err(trap) => {
                    self.emit(VirtualMachineEvent::Trap(trap.clone()));

                    self.emit(VirtualMachineEvent::ExecutionFinished);

                    return Err(trap);
                }
            }
        }

        self.log(VirtualMachineLogLevel::Info, "Virtual Machine Halt");

        self.emit(VirtualMachineEvent::ExecutionFinished);

        Ok(VirtualMachineYield::Finished)
    }

    pub fn resume_call(
        &mut self,
        values: HashMap<Slot, Arc<Value>>,
    ) -> Result<(), VirtualMachineTrap> {
        for (slot, value) in values {
            self.slots.insert(slot, value);
        }

        self.instruction_pointer += 1;

        Ok(())
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn execute(&mut self, instr: &Instruction) -> Result<ExecutionResult, VirtualMachineTrap> {
        match instr {
            // ── Bind ──────────────────────────────────────────────────────────
            Instruction::Bind {
                slot,
                type_name,
                value,
            } => {
                let v = parse_literal(type_name, value).ok_or_else(|| {
                    self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))
                })?;
                self.write(*slot, Arc::new(v));
                Ok(ExecutionResult::Advance)
            }

            // ── Call ──────────────────────────────────────────────────────────
            Instruction::Call {
                function_name,
                inputs,
                output,
            } => {
                let args = inputs
                    .iter()
                    .map(|s| self.read(*s))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = self.dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(ExecutionResult::Advance)
            }

            // ── Jump ──────────────────────────────────────────────────────────
            Instruction::Jump { target_position } => {
                self.log(
                    VirtualMachineLogLevel::Debug,
                    format!("Jump to {}", target_position),
                );
                Ok(ExecutionResult::Goto(*target_position))
            }

            // ── ConditionalJump ───────────────────────────────────────────────
            Instruction::ConditionalJump {
                condition,
                true_target_position,
                false_target_position,
            } => {
                let v = self.read(*condition)?;
                let b = match &*v {
                    Value::Boolean(b) => *b,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())));
                    }
                };
                self.log(
                    VirtualMachineLogLevel::Debug,
                    format!(
                        "Jump positions. true: {}, false: {}",
                        true_target_position, false_target_position
                    ),
                );
                Ok(ExecutionResult::Goto(if b {
                    self.log(
                        VirtualMachineLogLevel::Debug,
                        format!("Jump to {}", true_target_position),
                    );
                    *true_target_position
                } else {
                    self.log(
                        VirtualMachineLogLevel::Debug,
                        format!("Jump to {}", false_target_position),
                    );
                    *false_target_position
                }))
            }

            // ── SpecialCall ───────────────────────────────────────────────────
            Instruction::SpecialCall {
                function_name,
                inputs,
                output,
            } => {
                let args = inputs
                    .iter()
                    .map(|s| self.read(*s))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = self.special_dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(ExecutionResult::Advance)
            }

            Instruction::DefinedCall {
                function_identifier,
                inputs,
                destination_slots,
                source_slots,
            } => {
                let resolved_inputs = {
                    let mut result = HashMap::new();
                    for slot in inputs {
                        let read = match self.read(*slot) {
                            Ok(read) => read,
                            Err(error) => return Err(error),
                        };
                        result.insert(*slot, read);
                    }
                    result
                };

                Ok(ExecutionResult::YieldCall {
                    function_identifier: *function_identifier,
                    inputs: resolved_inputs,
                    destination_slots: destination_slots.clone(),
                    source_slots: source_slots.clone(),
                })
            }
        }
    }

    // ── Function dispatch ─────────────────────────────────────────────────────

    fn dispatch(&self, func: Functions, args: &[Arc<Value>]) -> Result<Value, VirtualMachineTrap> {
        // Convenience extractors — return VerifierBug trap on wrong variant.
        macro_rules! int {
            ($v:expr) => {
                match &*$v {
                    Value::Integer(n) => *n,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! float {
            ($v:expr) => {
                match &*$v {
                    Value::Float(f) => *f,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! bool_ {
            ($v:expr) => {
                match &*$v {
                    Value::Boolean(b) => *b,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! str_ {
            ($v:expr) => {
                match &*$v {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! vec_ {
            ($v:expr) => {
                match &*$v {
                    Value::Vector(v) => v.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }

        match func {
            // ── Integer arithmetic ────────────────────────────────────────────
            Functions::AddInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_add(int!(args[1]))))
            }
            Functions::SubInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_sub(int!(args[1]))))
            }
            Functions::MulInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(int!(args[0]).wrapping_mul(int!(args[1]))))
            }
            Functions::DivInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = int!(args[1]);
                if rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Value::Integer(int!(args[0]) / rhs))
            }
            Functions::ModInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let rhs = int!(args[1]);
                if rhs == 0 {
                    return Err(self.trap(TrapReason::DivisionByZero));
                }
                Ok(Value::Integer(int!(args[0]) % rhs))
            }
            Functions::PowInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let base = int!(args[0]);
                let exp = int!(args[1]);
                let exp_u = u32::try_from(exp).unwrap_or(0);
                Ok(Value::Integer(base.wrapping_pow(exp_u)))
            }

            // ── Float arithmetic ──────────────────────────────────────────────
            Functions::AddFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) + float!(args[1])))
            }
            Functions::SubFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) - float!(args[1])))
            }
            Functions::MulFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) * float!(args[1])))
            }
            Functions::DivFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]) / float!(args[1])))
            }
            Functions::PowFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Float(float!(args[0]).powf(float!(args[1]))))
            }

            // ── Integer comparisons ───────────────────────────────────────────
            Functions::EqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) == int!(args[1])))
            }
            Functions::NotEqualInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) != int!(args[1])))
            }
            Functions::GreaterThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) > int!(args[1])))
            }
            Functions::LessThanInteger => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(int!(args[0]) < int!(args[1])))
            }

            // ── Float comparisons ─────────────────────────────────────────────
            Functions::GreaterThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(float!(args[0]) > float!(args[1])))
            }
            Functions::LessThanFloat => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(float!(args[0]) < float!(args[1])))
            }

            // ── Boolean logic ─────────────────────────────────────────────────
            Functions::Not => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(!bool_!(args[0])))
            }
            Functions::And => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) && bool_!(args[1])))
            }
            Functions::Or => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) || bool_!(args[1])))
            }
            Functions::Xor => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(bool_!(args[0]) ^ bool_!(args[1])))
            }

            // ── String operations ─────────────────────────────────────────────
            Functions::EqualString => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(str_!(args[0]) == str_!(args[1])))
            }
            Functions::StringLength => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Integer(str_!(args[0]).chars().count() as i64))
            }
            Functions::StringGetChar => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let s = str_!(args[0]);
                let idx = int!(args[1]);
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let i = usize::try_from(idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::StringIndexOutOfBounds {
                            index: idx,
                            length: len,
                        })
                    })?;
                Ok(Value::Char(chars[i]))
            }

            // ── Vector get ────────────────────────────────────────────────────
            Functions::VectorGetInteger
            | Functions::VectorGetFloat
            | Functions::VectorGetString
            | Functions::VectorGetChar
            | Functions::VectorGetBoolean => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let v = vec_!(args[0]);
                let idx = int!(args[1]);
                let len = v.len();
                let i = usize::try_from(idx)
                    .ok()
                    .filter(|&i| i < len)
                    .ok_or_else(|| {
                        self.trap(TrapReason::IndexOutOfBounds {
                            index: idx,
                            length: len,
                        })
                    })?;
                Ok(v[i].clone())
            }

            // ── Vector init ───────────────────────────────────────────────────
            Functions::VectorInitInteger
            | Functions::VectorInitFloat
            | Functions::VectorInitString
            | Functions::VectorInitChar
            | Functions::VectorInitBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Vector(vec![(*args[0]).clone()]))
            }

            // ── Vector push ───────────────────────────────────────────────────
            Functions::VectorPushInteger
            | Functions::VectorPushFloat
            | Functions::VectorPushString
            | Functions::VectorPushChar
            | Functions::VectorPushBoolean => {
                if args.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let mut v = vec_!(args[0]);
                v.push((*args[1]).clone());
                Ok(Value::Vector(v))
            }

            // ── Vector pop ────────────────────────────────────────────────────
            Functions::VectorPopInteger
            | Functions::VectorPopFloat
            | Functions::VectorPopString
            | Functions::VectorPopChar
            | Functions::VectorPopBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let mut v = vec_!(args[0]);
                if v.is_empty() {
                    return Err(self.trap(TrapReason::IndexOutOfBounds {
                        index: -1,
                        length: 0,
                    }));
                }
                v.pop();
                Ok(Value::Vector(v))
            }

            // ── Option / Result inspection ────────────────────────────────────
            Functions::IsSome => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Option(Some(_)))))
            }
            Functions::IsNone => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Option(None))))
            }
            Functions::IsOk => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Result(Ok(_)))))
            }
            Functions::IsErr => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                Ok(Value::Boolean(matches!(&*args[0], Value::Result(Err(_)))))
            }

            // ── UnwrapSome ────────────────────────────────────────────────────
            Functions::UnwrapSomeInteger
            | Functions::UnwrapSomeFloat
            | Functions::UnwrapSomeString
            | Functions::UnwrapSomeChar
            | Functions::UnwrapSomeBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Option(Some(inner)) => Ok(*inner.clone()),
                    Value::Option(None) => Err(self.trap(TrapReason::UnwrapNone)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapOk ──────────────────────────────────────────────────────
            Functions::UnwrapOkInteger
            | Functions::UnwrapOkFloat
            | Functions::UnwrapOkString
            | Functions::UnwrapOkChar
            | Functions::UnwrapOkBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Result(Ok(inner)) => Ok(*inner.clone()),
                    Value::Result(Err(_)) => Err(self.trap(TrapReason::UnwrapErrOnOk)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }

            // ── UnwrapErr ─────────────────────────────────────────────────────
            Functions::UnwrapErrInteger
            | Functions::UnwrapErrFloat
            | Functions::UnwrapErrString
            | Functions::UnwrapErrChar
            | Functions::UnwrapErrBoolean => {
                if args.len() != 1 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                match &*args[0] {
                    Value::Result(Err(inner)) => Ok(*inner.clone()),
                    Value::Result(Ok(_)) => Err(self.trap(TrapReason::UnwrapOkOnErr)),
                    _ => Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string()))),
                }
            }
        }
    }

    // ── Special function dispatch ─────────────────────────────────────────────

    fn special_dispatch(
        &mut self,
        special_functions: SpecialFunctions,
        arguments: &[Arc<Value>],
    ) -> Result<Value, VirtualMachineTrap> {
        macro_rules! int {
            ($v:expr) => {
                match &*$v {
                    Value::Integer(n) => *n,
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }
        macro_rules! str_ {
            ($v:expr) => {
                match &*$v {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(self.trap(TrapReason::VerifierBug("Type Mismatch".to_string())))
                    }
                }
            };
        }

        match special_functions {
            SpecialFunctions::ReadGlobalMemoryInteger
            | SpecialFunctions::ReadGlobalMemoryFloat
            | SpecialFunctions::ReadGlobalMemoryString
            | SpecialFunctions::ReadGlobalMemoryChar
            | SpecialFunctions::ReadGlobalMemoryBoolean => {
                if arguments.len() != 2 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                let value = match self.global_memory.try_read() {
                    Ok(value) => value
                        .get(&(Namespace { 0: namespace }, Identifier { 0: identifier }))
                        .map(|v| Box::new(v.clone()))
                        .ok_or_else(|| Box::new(Value::String("not in value".to_string()))),
                    Err(error) => match error {
                        TryLockError::Poisoned(value) => {
                            self.emit(VirtualMachineEvent::Log(VirtualMachineLog {
                                level: VirtualMachineLogLevel::Warn,
                                message: "global memory is poisoned".to_string(),
                            }));
                            Ok(Box::new(Value::Option(
                                value
                                    .get_ref()
                                    .get(&(
                                        Namespace { 0: namespace },
                                        Identifier { 0: identifier },
                                    ))
                                    .map(|v| Box::new(v.clone())),
                            )))
                        }
                        TryLockError::WouldBlock => {
                            self.emit(VirtualMachineEvent::Log(VirtualMachineLog {
                                level: VirtualMachineLogLevel::Warn,
                                message: "global memory is blocking".to_string(),
                            }));
                            Err(Box::new(Value::String(
                                "global memory is blocking".to_string(),
                            )))
                        }
                    },
                };
                Ok(Value::Result(value))
            }

            SpecialFunctions::WriteGlobalMemoryInteger
            | SpecialFunctions::WriteGlobalMemoryFloat
            | SpecialFunctions::WriteGlobalMemoryString
            | SpecialFunctions::WriteGlobalMemoryChar
            | SpecialFunctions::WriteGlobalMemoryBoolean => {
                if arguments.len() != 3 {
                    return Err(self.trap(TrapReason::VerifierBug(
                        "Argument count Mismatch".to_string(),
                    )));
                }
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                match self.global_memory.try_write() {
                    Ok(value) => {
                        value
                            .insert(
                                (Namespace { 0: namespace }, Identifier { 0: identifier }),
                                arguments[2].deref().clone(),
                            )
                            .map(|v| Box::new(v.clone()));
                    }
                    Err(error) => match error {
                        TryLockError::Poisoned(mut value) => {
                            self.emit(VirtualMachineEvent::Log(VirtualMachineLog {
                                level: VirtualMachineLogLevel::Warn,
                                message: "global memory is poisoned".to_string(),
                            }));
                            value
                                .get_mut()
                                .insert(
                                    (Namespace { 0: namespace }, Identifier { 0: identifier }),
                                    arguments[2].deref().clone(),
                                )
                                .map(|v| Box::new(v.clone()));
                        }
                        TryLockError::WouldBlock => {
                            self.emit(VirtualMachineEvent::Log(VirtualMachineLog {
                                level: VirtualMachineLogLevel::Warn,
                                message: "global memory is blocking".to_string(),
                            }));
                        }
                    },
                };
                Ok(Value::Void) // todo: need result<void, string>
            }

            SpecialFunctions::GetInstructionPosition => {
                Ok(Value::Integer(self.instruction_pointer as i64))
            }

            SpecialFunctions::GetModificationNamespaceList => Ok(Value::Vector(
                self.modification_namespace_list
                    .iter()
                    .map(|ns| Value::String(ns.0.clone()))
                    .collect(),
            )),
        }
    }
}

// ── Literal conversion ────────────────────────────────────────────────────────

/// Returns `None` if the literal variant does not match the declared type,
/// which indicates the instruction stream was not verified before execution.
fn parse_literal(ty: &Type, lit: &Literal) -> Option<Value> {
    match (ty, lit) {
        (Type::Integer, Literal::Integer(n)) => Some(Value::Integer(*n)),
        (Type::Float, Literal::Float(f)) => Some(Value::Float(*f)),
        (Type::String, Literal::String(s)) => Some(Value::String(s.clone())),
        (Type::Char, Literal::Char(c)) => Some(Value::Char(*c)),
        (Type::Boolean, Literal::Boolean(b)) => Some(Value::Boolean(*b)),
        _ => None,
    }
}

// ── Logger ────────────────────────────────────────────────────────────────────

pub struct Logger {
    channel_receiver: Receiver<VirtualMachineEvent>,
    verbose: bool,
}

impl Logger {
    pub fn new(rx: Receiver<VirtualMachineEvent>) -> Self {
        Self {
            channel_receiver: rx,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, v: bool) -> Self {
        self.verbose = v;
        self
    }

    pub fn run(&self) {
        while let Ok(event) = self.channel_receiver.recv() {
            match event {
                VirtualMachineEvent::Log(l) => {
                    let line = format!("[{:?}] {}", l.level, l.message);
                    match l.level {
                        VirtualMachineLogLevel::Warn | VirtualMachineLogLevel::Error => {
                            eprintln!("{line}")
                        }
                        _ => println!("{line}"),
                    }
                }
                VirtualMachineEvent::Trap(t) => {
                    eprintln!("[TRAP @ {}] {:?}", t.trapped_position, t.reason);
                }
                VirtualMachineEvent::StateChange(s) if self.verbose => {
                    println!("[STATE] {}: {:?} -> {:?}", s.identifier, s.old, s.new);
                }
                VirtualMachineEvent::StateChange(_) => {}
                VirtualMachineEvent::ExecutionFinished => {
                    println!("[VM] finished");
                    break;
                }
            }
        }
    }
}
