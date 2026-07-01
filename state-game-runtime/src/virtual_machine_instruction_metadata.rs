use crate::virtual_machine::instruction::Slot;
use crate::virtual_machine::types::Type;
use std::collections::HashMap;

struct VirtualMachineInstructionMetadata {
    slot_types: HashMap<Slot, Type>,
    input_types: HashMap<Slot, Type>,
    output_types: HashMap<Slot, Type>,
}
