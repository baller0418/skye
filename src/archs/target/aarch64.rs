use crate::encoder::aarch64;

pub const AARCH64: super::Isa = super::Isa {
    NAME: "aarch64",
    MACHINE: 0xb7,

    REGS: &[
        ("r0", 0),
        ("r1", 1),
        ("r2", 2),
        ("r3", 3),
        ("r4", 4),
        ("r5", 5),
        ("r6", 6),
        ("r7", 8),
        ("r31", 31),
    ],
    SYSCALL_ARGS: &["r0", "r1", "r2"],
    SYSCALL_NUM: "r7",

    SYSCALL: &[0x01, 0x00, 0x00, 0xD4],
    STORE: &[0x00, 0x00, 0x00, 0xF9],
    LOAD: &[0x00, 0x00, 0x40, 0xF9],
    JUMP: &[0x00, 0x00, 0x00, 0x14],
    CALL: &[0x00, 0x00, 0x00, 0x94],
    RET: &[0xC0, 0x03, 0x5F, 0xD6],

    EXIT: "93",
    WRITE: "64",

    STACK: "r31",

    REL_WIDTH: 0,

    ENC_REG: aarch64::assemble_register,
    ENC_MEM: aarch64::assemble_memory_operation,
    ENC_ADDR: aarch64::assemble_address,
    ENC_CMP: aarch64::assemble_compare,

    PATCH: |out, start, _slot, target| {
        let word = u32::from_le_bytes(out[start..start + 4].try_into().unwrap());

        let offset = target as i64 - start as i64;

        let patched = if word & 0x9F00_0000 == 0x1000_0000 {
            let imm = offset as u32 & 0x1F_FFFF;

            word | ((imm & 0b11) << 29) | ((imm >> 2) << 5)
        } else if word & 0xFF00_0010 == 0x5400_0000 {
            word | ((((offset / 4) as u32) & 0x7_FFFF) << 5)
        } else {
            word | (((offset / 4) as u32) & 0x03FF_FFFF)
        };

        out[start..start + 4].copy_from_slice(&patched.to_le_bytes());
    },
};
