use std::collections::HashMap;
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::instruction_run::instruction::{Functions, Instruction, Literal, Slot};
use crate::instruction_run::types::{Type, Value};
use crate::instruction_run::event::{
    StateChange, TrapReason, VirtualMachineEvent, VirtualMachineLog,
    VirtualMachineLogLevel, VirtualMachineTrap,
};

// ── Instruction pointer step ─────────────────────────────────────────────────

enum NextInstructionPointer {
    /// Advance by N (almost always 1).
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
    /// is read, so `unwrap()` on a missing slot is a verifier bug, not a
    /// runtime error.
    slots: HashMap<Slot, Arc<Value>>,
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

    /// Read a slot. Panics if the slot is unbound — the verifier must have
    /// already confirmed every slot is written before it is read.
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
                    // The verifier already confirmed every jump target is in-bounds.
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
        self.log(VirtualMachineLogLevel::Debug, format!("Instruction: {:?}", self.instructions));
        self.log(VirtualMachineLogLevel::Debug, format!("instruction_pointer: {}", self.instruction_pointer));
        self.log(VirtualMachineLogLevel::Debug, format!("slots: {:?}", self.slots));
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
            // The verifier confirmed the literal is parseable as `type_name`.
            Instruction::Bind { slot, type_name, value } => {
                let v = parse_literal(type_name, value);
                self.write(*slot, Arc::new(v));
                Ok(NextInstructionPointer::Advance)
            }

            // ── Call ──────────────────────────────────────────────────────────
            // The verifier confirmed arity and argument types; we only need to
            // handle runtime failures (division by zero, out-of-bounds, etc.).
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
            // The verifier confirmed `condition` holds a Boolean.
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

            // ── UnwrapSome ────────────────────────────────────────────────────
            // The verifier confirmed `input` is Option<_>. Runtime can still
            // trap on None.
            Instruction::UnwrapSome { output, input } => {
                let v = self.read(*input);
                match &*v {
                    Value::Option(Some(inner)) => {
                        self.write(*output, Arc::new(*inner.clone()));
                        Ok(NextInstructionPointer::Advance)
                    }
                    Value::Option(None) => Err(self.trap(TrapReason::UnwrapNone)),
                    _ => panic!("verifier bug: UnwrapSome on non-Option slot"),
                }
            }

            // ── UnwrapOk ──────────────────────────────────────────────────────
            // The verifier confirmed `input` is Result<_, _>. Runtime can still
            // trap if the value is Err.
            Instruction::UnwrapOk { output, input } => {
                let v = self.read(*input);
                match &*v {
                    Value::Result(Ok(inner)) => {
                        self.write(*output, Arc::new(*inner.clone()));
                        Ok(NextInstructionPointer::Advance)
                    }
                    Value::Result(Err(_)) => Err(self.trap(TrapReason::UnwrapErrOnOk)),
                    _ => panic!("verifier bug: UnwrapOk on non-Result slot"),
                }
            }

            // ── UnwrapErr ─────────────────────────────────────────────────────
            Instruction::UnwrapErr { output, input } => {
                let v = self.read(*input);
                match &*v {
                    Value::Result(Err(inner)) => {
                        self.write(*output, Arc::new(*inner.clone()));
                        Ok(NextInstructionPointer::Advance)
                    }
                    Value::Result(Ok(_)) => Err(self.trap(TrapReason::UnwrapOkOnErr)),
                    _ => panic!("verifier bug: UnwrapErr on non-Result slot"),
                }
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
            Functions::EqualInteger        => Ok(Value::Boolean(int!(args[0]) == int!(args[1]))),
            Functions::NotEqualInteger     => Ok(Value::Boolean(int!(args[0]) != int!(args[1]))),
            Functions::GreaterThanInteger  => Ok(Value::Boolean(int!(args[0]) >  int!(args[1]))),
            Functions::LessThanInteger     => Ok(Value::Boolean(int!(args[0]) <  int!(args[1]))),

            // ── Float comparisons ─────────────────────────────────────────────
            Functions::GreaterThanFloat => Ok(Value::Boolean(float!(args[0]) > float!(args[1]))),
            Functions::LessThanFloat    => Ok(Value::Boolean(float!(args[0]) < float!(args[1]))),

            // ── Boolean logic ─────────────────────────────────────────────────
            Functions::Not => Ok(Value::Boolean(!bool_!(args[0]))),
            Functions::And => Ok(Value::Boolean(bool_!(args[0]) && bool_!(args[1]))),
            Functions::Or  => Ok(Value::Boolean(bool_!(args[0]) || bool_!(args[1]))),
            Functions::Xor => Ok(Value::Boolean(bool_!(args[0]) ^  bool_!(args[1]))),

            // ── String operations ─────────────────────────────────────────────
            Functions::EqualString   => Ok(Value::Boolean(str_!(args[0]) == str_!(args[1]))),
            Functions::StringLength  => Ok(Value::Integer(str_!(args[0]).chars().count() as i64)),
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

            // ── Vector init (single-element) ──────────────────────────────────
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
                let len = v.len();
                if len == 0 {
                    return Err(self.trap(TrapReason::IndexOutOfBounds { index: -1, length: 0 }));
                }
                v.pop();
                Ok(Value::Vector(v))
            }
        }
    }
}

// ── Literal conversion ────────────────────────────────────────────────────────
// Literals already hold typed values, so this is a direct unwrap.

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
