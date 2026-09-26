//! linux specific for now
//! may support windows/macOS in fututre,

mod aarch64;
mod x86_64;

pub use aarch64::AARCH64;
pub use x86_64::X86_64;

#[allow(non_snake_case, unused)]
pub struct Isa {
    pub NAME: &'static str,
    pub MACHINE: u16,

    pub REGS: &'static [(&'static str, u8)],
    pub SYSCALL_ARGS: &'static [&'static str],
    pub SYSCALL_NUM: &'static str,

    pub SYSCALL: &'static [u8],
    pub STORE: &'static [u8],
    pub LOAD: &'static [u8],
    pub JUMP: &'static [u8],
    pub CALL: &'static [u8],
    pub RET: &'static [u8],

    pub EXIT: &'static str,
    pub WRITE: &'static str,

    pub STACK: &'static str,

    pub REL_WIDTH: usize,

    pub ENC_REG: fn(&'static Isa, u8, &str) -> Vec<u8>,
    pub ENC_MEM: fn(&'static Isa, &[u8], u8, &str) -> Vec<u8>,
    pub ENC_ADDR: fn(&'static Isa, u8) -> Vec<u8>,
    pub ENC_CMP: fn(&'static Isa, &str) -> (Vec<u8>, Vec<u8>),

    pub PATCH: fn(&mut [u8], usize, usize, usize),
}

pub fn get(name: &str) -> &'static Isa {
    match name.trim().to_ascii_lowercase().as_str() {
        "x86" | "x86_64" | "x64" | "amd" | "amd64" => &X86_64,
        "arm" | "arm64" | "aarch64" => &AARCH64,
        other => panic!("unknown target: {other}"),
    }
}

#[cfg(test)]
pub fn all() -> &'static [&'static Isa] {
    &[&X86_64, &AARCH64]
}

impl Isa {
    pub fn reg(&self, name: &str) -> u8 {
        let name = name.trim().trim_matches('|');
        let name = if name == "stack" { self.STACK } else { name };

        self.REGS
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("{}: unknown register {name}", self.NAME))
            .1
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn targets_have_distinct_machines() {
        let mut seen = std::collections::HashSet::new();

        for isa in crate::archs::target::all() {
            assert!(
                seen.insert(isa.MACHINE),
                "{} duplicates a machine id",
                isa.NAME
            );
        }
    }
}
