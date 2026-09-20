//! linux specific for now
//! may support windows/macOS in fututre,


#[macro_export]
macro_rules! maketarget {

    (
        fields { $(  $field:ident : $type:ty,  )* }

        $(
            $target:ident => [ $($alias:literal),* ] {
                $(  $key:ident : $value:expr,  )*
            }
        )*
    ) => {

        #[allow(non_snake_case)]
        pub struct Isa {
            $(  pub $field: $type,  )*
        }

        $(
            pub const $target: Isa = Isa {
                $(  $key: $value,  )*
            };
        )*

        pub fn get(name: &str) -> &'static Isa {
            match name.trim().to_ascii_lowercase().as_str() {
                $(  $($alias)|* => &$target,  )*
                other => panic!("unknown target: {other}"),
            }
        }

        pub fn all() -> &'static [&'static Isa] {
            &[ $( &$target, )* ]
        }
    };
}

maketarget! {

    fields {
        NAME:    &'static str,
        MACHINE: u16,

        REGS:         &'static [(&'static str, u8)],
        SYSCALL_ARGS: &'static [&'static str],
        SYSCALL_NUM:  &'static str,

        SYSCALL: &'static [u8],
        STORE:   &'static [u8],
        LOAD:    &'static [u8],
        JUMP:    &'static [u8],
        CALL:    &'static [u8],
        RET:     &'static [u8],

        EXIT:  &'static str,
        WRITE: &'static str,

        STACK: &'static str,

        REL_WIDTH: usize,

        ENC_REG:  fn(&'static Isa, u8, &str) -> Vec<u8>,
        ENC_MEM:  fn(&'static Isa, &[u8], u8, &str) -> Vec<u8>,
        ENC_ADDR: fn(&'static Isa, u8) -> Vec<u8>,
        ENC_CMP:  fn(&'static Isa, &str) -> (Vec<u8>, Vec<u8>),

        PATCH: fn(&mut [u8], usize, usize, usize),
    }

    X86_64 => ["x86", "x86_64", "x64", "amd", "amd64"] {
        NAME:    "x86_64",
        MACHINE: 0x3e,

        REGS: &[("r0",0),("r1",1),("r2",2),("r3",3),
                ("r4",4),("r5",5),("r6",6),("r7",7)],
        SYSCALL_ARGS: &["r7","r6","r2"],
        SYSCALL_NUM:  "r0",

        SYSCALL: &[0x0F, 0x05],
        STORE:   &[0x48, 0x89],
        LOAD:    &[0x48, 0x8B],
        JUMP:    &[0xE9],
        CALL:    &[0xE8],
        RET:     &[0xC3],

        EXIT:  "60",
        WRITE: "1",

        STACK: "r4",

        REL_WIDTH: 4,
        ENC_REG:  crate::encoder::assemble_register,
        ENC_MEM:  crate::encoder::assemble_memory_operation,
        ENC_ADDR: crate::encoder::assemble_address,
        ENC_CMP:  crate::encoder::assemble_compare,
        PATCH: |out, _start, slot, target| {
            let rel = target as i32 - (slot as i32 + 4);
            out[slot..slot+4].copy_from_slice(&rel.to_le_bytes());
        },
    }

    AARCH64 => ["arm", "arm64", "aarch64"] {
        NAME:    "aarch64",
        MACHINE: 0xb7,

        REGS: &[("r0",0),("r1",1),("r2",2),("r3",3),
                ("r4",4),("r5",5),("r6",6),("r7",8),("r31",31)],
        SYSCALL_ARGS: &["r0","r1","r2"],
        SYSCALL_NUM:  "r7",
        
        SYSCALL: &[0x01, 0x00, 0x00, 0xD4],
        STORE:   &[0x00, 0x00, 0x00, 0xF9],
        LOAD:    &[0x00, 0x00, 0x40, 0xF9],
        JUMP:    &[0x00, 0x00, 0x00, 0x14],
        CALL:    &[0x00, 0x00, 0x00, 0x94],
        RET:     &[0xC0, 0x03, 0x5F, 0xD6],

        EXIT:  "93",
        WRITE: "64",

        STACK: "r31",

        REL_WIDTH: 0,
        ENC_REG:  crate::encoder::arm::assemble_register,
        ENC_MEM:  crate::encoder::arm::assemble_memory_operation,
        ENC_ADDR: crate::encoder::arm::assemble_address,
        ENC_CMP:  crate::encoder::arm::assemble_compare,
        PATCH: |out, start, _slot, target| {

            let word = u32::from_le_bytes(out[start..start+4].try_into().unwrap());
            let offset = target as i64 - start as i64;

            let patched = if word & 0x9F00_0000 == 0x1000_0000 {
                let imm = offset as u32 & 0x1F_FFFF;
                word | ((imm & 0b11) << 29) | ((imm >> 2) << 5)
            } else if word & 0xFF00_0010 == 0x5400_0000 {
                word | ((((offset / 4) as u32) & 0x7_FFFF) << 5)
            } else {
                word | (((offset / 4) as u32) & 0x03FF_FFFF)
            };

            out[start..start+4].copy_from_slice(&patched.to_le_bytes());
        },
    }
}

impl Isa {
    pub fn reg(&self, name: &str) -> u8 {
        let name = name.trim().trim_matches('|');
        let name = if name == "stack" { self.STACK } else { name };
        self.REGS.iter().find(|(n,_)| *n == name)
            .unwrap_or_else(|| panic!("{}: unknown register {name}", self.NAME))
            .1
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_have_distinct_machines() {
        let mut seen = std::collections::HashSet::new();
        for isa in crate::target::all() {
            assert!(seen.insert(isa.MACHINE), "{} duplicates a machine id", isa.NAME);
        }
    }
}