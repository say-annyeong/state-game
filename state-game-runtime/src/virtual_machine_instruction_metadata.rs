use crate::runtime_task::instruction::Slot;
use crate::runtime_task::types::Type;
use std::collections::HashMap;

struct VirtualMachineInstructionMetadata {
    slot_types: HashMap<Slot, Type>,
    input_types: HashMap<Slot, Type>,
    output_types: HashMap<Slot, Type>,
}
