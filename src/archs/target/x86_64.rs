use crate::encoder::x86_64;

pub const X86_64: super::Isa = super::Isa {
    NAME: "x86_64",
    MACHINE: 0x3e,

    REGS: &[
        ("r0", 0),
        ("r1", 1),
        ("r2", 2),
        ("r3", 3),
        ("r4", 4),
        ("r5", 5),
        ("r6", 6),
        ("r7", 7),
    ],
    SYSCALL_ARGS: &["r7", "r6", "r2"],
    SYSCALL_NUM: "r0",

    SYSCALL: &[0x0F, 0x05],
    STORE: &[0x48, 0x89],
    LOAD: &[0x48, 0x8B],
    JUMP: &[0xE9],
    CALL: &[0xE8],
    RET: &[0xC3],

    EXIT: "60",
    WRITE: "1",

    STACK: "r4",

    REL_WIDTH: 4,

    ENC_REG: x86_64::assemble_register,
    ENC_MEM: x86_64::assemble_memory_operation,
    ENC_ADDR: x86_64::assemble_address,
    ENC_CMP: x86_64::assemble_compare,

    PATCH: |out, _start, slot, target| {
        let rel = target as i32 - (slot as i32 + 4);
        out[slot..slot + 4].copy_from_slice(&rel.to_le_bytes());
    },
};
