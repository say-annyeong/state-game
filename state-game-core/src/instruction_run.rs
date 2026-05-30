mod event;
mod types;
mod instruction;
mod instruction_verifier;
mod virtual_machine;

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::thread;
    use crate::instruction_run::instruction::{Functions, Instruction, Literal};
    use crate::instruction_run::instruction_verifier::InstructionVerifier;
    use crate::instruction_run::types::Type;
    use crate::instruction_run::virtual_machine::{Logger, VirtualMachine};

    #[test]
    fn test_instruction() {
        let instructions = Arc::new([
            Instruction::Bind { slot: 0, value: Literal::Integer(2), type_name: Type::Integer },
            Instruction::Bind { slot: 1, value: Literal::Integer(3), type_name: Type::Integer },
            Instruction::Call { function_name: Functions::AddInteger, output: 2, arguments: vec![0, 1] },
        ]);
        let instruction_verifier = InstructionVerifier::new(instructions.clone()).verify().is_empty();
        let (sender, receiver) = crossbeam_channel::unbounded();
        assert!(instruction_verifier);
        let mut virtual_machine = VirtualMachine::new(sender, instructions.clone());
        let logger = Logger::new(receiver);
        let (thread1, thread2) = (thread::spawn(move || {let _ = virtual_machine.run(); } ), thread::spawn(move || {let _ = logger.run(); } ));
        let _ = thread1.join();
        let _ = thread2.join();
    }
}