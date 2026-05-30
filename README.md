# Stage Game
modding able state game

# Specifications
1. Instruction. Virtual Machine-friendly
2. Virtual Machine. Run to Instruction.
3. State Game Application Programming Interface. Call by Virtual Machine

## Instruction
Instruction is Virtual Machine-Friendly struct
1. Bind. Bind is make new Slot and overwrite.
2. Call. Call to function Virtual Machine inside. make new Slot and overwrite.
3. Jump. Move to Instruction Pointer
4. ConditionalJump. Conditional is can use boolean Slot. Instruction Pointer is can two select. left side is true. right side is false.
5. SpecialCall. SpecialCall is can read and write Virtual Machine field

## Virtual Machine. 