use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::instruction_run::instruction::{Functions, Instruction, Literal, Slot, SpecialFunctions};
use crate::instruction_run::types::{Type, Value};
use crate::instruction_run::event::{
    StateChange, TrapReason, VirtualMachineEvent, VirtualMachineLog,
    VirtualMachineLogLevel, VirtualMachineTrap,
};
use crate::{Identifier, Namespace};
use crate::instruction_run::types::Value::Option;
// ── Instruction pointer step ─────────────────────────────────────────────────

enum NextInstructionPointer {
    /// Advance by 1.
    Advance,
    /// Jump to an absolute position (already verified to be in-bounds).
    Goto(usize),
}

// ── Virtual Machine ──────────────────────────────────────────────────────────

pub struct VirtualMachine {
    channel_transmit: Sender<VirtualMachineEvent>,
    instruction_pointer: usize,
    instructions: Arc<[Instruction]>,
    /// Slot storage. The verifier guarantees every slot is written before it
    /// is read, so a missing slot is always a verifier bug.
    slots: HashMap<Slot, Arc<Value>>,
    global_memory: HashMap<(Namespace, Identifier), Arc<Value>>,
}

impl VirtualMachine {
    pub fn new(
        channel_transmit: Sender<VirtualMachineEvent>,
        instructions: Arc<[Instruction]>,
    ) -> Self {
        Self {
            channel_transmit,
            instruction_pointer: 0,
            instructions,
            slots: HashMap::new(),
            global_memory: HashMap::new(),
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn emit(&self, event: VirtualMachineEvent) {
        let _ = self.channel_transmit.send(event);
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

    /// Read a slot. Panics if unbound — the verifier must have confirmed every
    /// slot is written before it is read.
    fn read(&self, slot: Slot) -> Arc<Value> {
        self.slots
            .get(&slot)
            .cloned()
            .unwrap_or_else(|| panic!("verifier bug: slot {slot} read before write"))
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

    pub fn run(&mut self) -> Result<(), VirtualMachineTrap> {
        self.log(VirtualMachineLogLevel::Info, "Virtual Machine Start");

        while self.instruction_pointer < self.instructions.len() {
            let instr = self.instructions[self.instruction_pointer].clone();
            match self.execute(&instr) {
                Ok(NextInstructionPointer::Advance) => self.instruction_pointer += 1,
                Ok(NextInstructionPointer::Goto(target)) => {
                    self.instruction_pointer = target;
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
        Ok(())
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn execute(
        &mut self,
        instr: &Instruction,
    ) -> Result<NextInstructionPointer, VirtualMachineTrap> {
        match instr {
            // ── Bind ──────────────────────────────────────────────────────────
            Instruction::Bind { slot, type_name, value } => {
                let v = parse_literal(type_name, value);
                self.write(*slot, Arc::new(v));
                Ok(NextInstructionPointer::Advance)
            }

            // ── Call ──────────────────────────────────────────────────────────
            Instruction::Call { function_name, output, arguments } => {
                let args: Vec<Arc<Value>> =
                    arguments.iter().map(|s| self.read(*s)).collect();
                let result = self.dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(NextInstructionPointer::Advance)
            }

            // ── Jump ──────────────────────────────────────────────────────────
            Instruction::Jump { target_position } => {
                Ok(NextInstructionPointer::Goto(*target_position))
            }

            // ── ConditionalJump ───────────────────────────────────────────────
            Instruction::ConditionalJump {
                condition,
                true_target_position,
                false_target_position,
            } => {
                let v = self.read(*condition);
                let b = match &*v {
                    Value::Boolean(b) => *b,
                    _ => panic!("verifier bug: condition slot is not Boolean"),
                };
                Ok(NextInstructionPointer::Goto(if b {
                    *true_target_position
                } else {
                    *false_target_position
                }))
            }

            Instruction::SpecialCall { function_name, output, arguments } => {
                let args: Vec<Arc<Value>> =
                    arguments.iter().map(|s| self.read(*s)).collect();
                let result = self.special_dispatch(*function_name, &args)?;
                self.write(*output, Arc::new(result));
                Ok(NextInstructionPointer::Advance)
            }
        }
    }

    // ── Function dispatch ─────────────────────────────────────────────────────

    fn dispatch(
        &self,
        func: Functions,
        args: &[Arc<Value>],
    ) -> Result<Value, VirtualMachineTrap> {
        // Convenience extractors — panic on wrong variant (verifier bug).
        macro_rules! int   { ($v:expr) => { match &*$v { Value::Integer(n) => *n, _ => panic!("verifier bug") } } }
        macro_rules! float { ($v:expr) => { match &*$v { Value::Float(f)   => *f, _ => panic!("verifier bug") } } }
        macro_rules! bool_ { ($v:expr) => { match &*$v { Value::Boolean(b) => *b, _ => panic!("verifier bug") } } }
        macro_rules! str_  { ($v:expr) => { match &*$v { Value::String(s)  => s.clone(), _ => panic!("verifier bug") } } }
        macro_rules! vec_  { ($v:expr) => { match &*$v { Value::Vector(v)  => v.clone(), _ => panic!("verifier bug") } } }

        match func {
            // ── Integer arithmetic ────────────────────────────────────────────
            Functions::AddInteger => Ok(Value::Integer(int!(args[0]).wrapping_add(int!(args[1])))),
            Functions::SubInteger => Ok(Value::Integer(int!(args[0]).wrapping_sub(int!(args[1])))),
            Functions::MulInteger => Ok(Value::Integer(int!(args[0]).wrapping_mul(int!(args[1])))),
            Functions::DivInteger => {
                let rhs = int!(args[1]);
                if rhs == 0 { return Err(self.trap(TrapReason::DivisionByZero)); }
                Ok(Value::Integer(int!(args[0]) / rhs))
            }
            Functions::ModInteger => {
                let rhs = int!(args[1]);
                if rhs == 0 { return Err(self.trap(TrapReason::DivisionByZero)); }
                Ok(Value::Integer(int!(args[0]) % rhs))
            }
            Functions::PowInteger => {
                let base = int!(args[0]);
                let exp  = int!(args[1]);
                let exp_u = u32::try_from(exp).unwrap_or(0);
                Ok(Value::Integer(base.wrapping_pow(exp_u)))
            }

            // ── Float arithmetic ──────────────────────────────────────────────
            Functions::AddFloat => Ok(Value::Float(float!(args[0]) + float!(args[1]))),
            Functions::SubFloat => Ok(Value::Float(float!(args[0]) - float!(args[1]))),
            Functions::MulFloat => Ok(Value::Float(float!(args[0]) * float!(args[1]))),
            Functions::DivFloat => Ok(Value::Float(float!(args[0]) / float!(args[1]))),
            Functions::PowFloat => Ok(Value::Float(float!(args[0]).powf(float!(args[1])))),

            // ── Integer comparisons ───────────────────────────────────────────
            Functions::EqualInteger       => Ok(Value::Boolean(int!(args[0]) == int!(args[1]))),
            Functions::NotEqualInteger    => Ok(Value::Boolean(int!(args[0]) != int!(args[1]))),
            Functions::GreaterThanInteger => Ok(Value::Boolean(int!(args[0]) >  int!(args[1]))),
            Functions::LessThanInteger    => Ok(Value::Boolean(int!(args[0]) <  int!(args[1]))),

            // ── Float comparisons ─────────────────────────────────────────────
            Functions::GreaterThanFloat => Ok(Value::Boolean(float!(args[0]) > float!(args[1]))),
            Functions::LessThanFloat    => Ok(Value::Boolean(float!(args[0]) < float!(args[1]))),

            // ── Boolean logic ─────────────────────────────────────────────────
            Functions::Not => Ok(Value::Boolean(!bool_!(args[0]))),
            Functions::And => Ok(Value::Boolean(bool_!(args[0]) && bool_!(args[1]))),
            Functions::Or  => Ok(Value::Boolean(bool_!(args[0]) || bool_!(args[1]))),
            Functions::Xor => Ok(Value::Boolean(bool_!(args[0]) ^  bool_!(args[1]))),

            // ── String operations ─────────────────────────────────────────────
            Functions::EqualString  => Ok(Value::Boolean(str_!(args[0]) == str_!(args[1]))),
            Functions::StringLength => Ok(Value::Integer(str_!(args[0]).chars().count() as i64)),
            Functions::StringGetChar => {
                let s   = str_!(args[0]);
                let idx = int!(args[1]);
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let i = usize::try_from(idx).ok().filter(|&i| i < len).ok_or_else(|| {
                    self.trap(TrapReason::StringIndexOutOfBounds { index: idx, length: len })
                })?;
                Ok(Value::Char(chars[i]))
            }

            // ── Vector get ────────────────────────────────────────────────────
            Functions::VectorGetInteger
            | Functions::VectorGetFloat
            | Functions::VectorGetString
            | Functions::VectorGetChar
            | Functions::VectorGetBoolean => {
                let v   = vec_!(args[0]);
                let idx = int!(args[1]);
                let len = v.len();
                let i = usize::try_from(idx).ok().filter(|&i| i < len).ok_or_else(|| {
                    self.trap(TrapReason::IndexOutOfBounds { index: idx, length: len })
                })?;
                Ok(v[i].clone())
            }

            // ── Vector init ───────────────────────────────────────────────────
            Functions::VectorInitInteger
            | Functions::VectorInitFloat
            | Functions::VectorInitString
            | Functions::VectorInitChar
            | Functions::VectorInitBoolean => {
                Ok(Value::Vector(vec![(*args[0]).clone()]))
            }

            // ── Vector push ───────────────────────────────────────────────────
            Functions::VectorPushInteger
            | Functions::VectorPushFloat
            | Functions::VectorPushString
            | Functions::VectorPushChar
            | Functions::VectorPushBoolean => {
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
                let mut v = vec_!(args[0]);
                if v.is_empty() {
                    return Err(self.trap(TrapReason::IndexOutOfBounds { index: -1, length: 0 }));
                }
                v.pop();
                Ok(Value::Vector(v))
            }

            // ── Option / Result inspection ────────────────────────────────────
            Functions::IsSome => Ok(Value::Boolean(matches!(&*args[0], Value::Option(Some(_))))),
            Functions::IsNone => Ok(Value::Boolean(matches!(&*args[0], Value::Option(None)))),
            Functions::IsOk   => Ok(Value::Boolean(matches!(&*args[0], Value::Result(Ok(_))))),
            Functions::IsErr  => Ok(Value::Boolean(matches!(&*args[0], Value::Result(Err(_))))),

            // ── UnwrapSome ────────────────────────────────────────────────────
            // Runtime trap if the value is None.
            Functions::UnwrapSomeInteger
            | Functions::UnwrapSomeFloat
            | Functions::UnwrapSomeString
            | Functions::UnwrapSomeChar
            | Functions::UnwrapSomeBoolean => {
                match &*args[0] {
                    Value::Option(Some(inner)) => Ok(*inner.clone()),
                    Value::Option(None)        => Err(self.trap(TrapReason::UnwrapNone)),
                    _                          => panic!("verifier bug: UnwrapSome on non-Option"),
                }
            }

            // ── UnwrapOk ──────────────────────────────────────────────────────
            // Runtime trap if the value is Err.
            Functions::UnwrapOkInteger
            | Functions::UnwrapOkFloat
            | Functions::UnwrapOkString
            | Functions::UnwrapOkChar
            | Functions::UnwrapOkBoolean => {
                match &*args[0] {
                    Value::Result(Ok(inner))  => Ok(*inner.clone()),
                    Value::Result(Err(_))     => Err(self.trap(TrapReason::UnwrapErrOnOk)),
                    _                         => panic!("verifier bug: UnwrapOk on non-Result"),
                }
            }

            // ── UnwrapErr ─────────────────────────────────────────────────────
            // Runtime trap if the value is Ok.
            Functions::UnwrapErrInteger
            | Functions::UnwrapErrFloat
            | Functions::UnwrapErrString
            | Functions::UnwrapErrChar
            | Functions::UnwrapErrBoolean => {
                match &*args[0] {
                    Value::Result(Err(inner)) => Ok(*inner.clone()),
                    Value::Result(Ok(_))      => Err(self.trap(TrapReason::UnwrapOkOnErr)),
                    _                         => panic!("verifier bug: UnwrapErr on non-Result"),
                }
            }
        }
    }

    fn special_dispatch(&mut self, special_functions: SpecialFunctions, arguments: &[Arc<Value>]) -> Result<Value, VirtualMachineTrap> {
        macro_rules! int   { ($v:expr) => { match &*$v { Value::Integer(n) => *n, _ => panic!("verifier bug") } } }
        macro_rules! float { ($v:expr) => { match &*$v { Value::Float(f)   => *f, _ => panic!("verifier bug") } } }
        macro_rules! bool_ { ($v:expr) => { match &*$v { Value::Boolean(b) => *b, _ => panic!("verifier bug") } } }
        macro_rules! str_  { ($v:expr) => { match &*$v { Value::String(s)  => s.clone(), _ => panic!("verifier bug") } } }
        macro_rules! vec_  { ($v:expr) => { match &*$v { Value::Vector(v)  => v.clone(), _ => panic!("verifier bug") } } }
        match special_functions {
            SpecialFunctions::ReadGlobalMemoryInteger
            | SpecialFunctions::ReadGlobalMemoryFloat
            | SpecialFunctions::ReadGlobalMemoryString
            | SpecialFunctions::ReadGlobalMemoryChar
            | SpecialFunctions::ReadGlobalMemoryBoolean => {
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                let value = self.global_memory.get(&(namespace, identifier)).map(|v| {
                    Box::new(v.deref().clone())
                });
                Ok(Option(value))
            }
            SpecialFunctions::WriteGlobalMemoryInteger
            | SpecialFunctions::WriteGlobalMemoryFloat
            | SpecialFunctions::WriteGlobalMemoryString
            | SpecialFunctions::WriteGlobalMemoryChar
            | SpecialFunctions::WriteGlobalMemoryBoolean => {
                let namespace = str_!(arguments[0]);
                let identifier = str_!(arguments[1]);
                self.global_memory.insert((namespace, identifier), arguments[2].clone());
                Ok(Value::Void)
            }
            SpecialFunctions::GetInstructionPosition => {
                Ok(Value::Integer(self.instruction_pointer as i64))
            }
        }
    }
}

// ── Literal conversion ────────────────────────────────────────────────────────

fn parse_literal(ty: &Type, lit: &Literal) -> Value {
    match (ty, lit) {
        (Type::Integer, Literal::Integer(n)) => Value::Integer(*n),
        (Type::Float,   Literal::Float(f))   => Value::Float(*f),
        (Type::String,  Literal::String(s))  => Value::String(s.clone()),
        (Type::Char,    Literal::Char(c))    => Value::Char(*c),
        (Type::Boolean, Literal::Boolean(b)) => Value::Boolean(*b),
        _ => panic!("verifier bug: literal/type mismatch"),
    }
}

// ── Logger ────────────────────────────────────────────────────────────────────

pub struct Logger {
    channel_receiver: Receiver<VirtualMachineEvent>,
    verbose: bool,
}

impl Logger {
    pub fn new(rx: Receiver<VirtualMachineEvent>) -> Self {
        Self { channel_receiver: rx, verbose: false }
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
