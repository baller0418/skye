use crate::join_bytes;
use crate::common;
use crate::encoder;

use std::collections::HashMap;

pub struct ByteStream {
    raw_stream: Vec<u8>,
    labels: HashMap<String, usize>,
    fixups: Vec<(usize, String)>,
    block_stack: Vec<String>,
    block_counter: usize,
    data: Vec<(String, Vec<u8>)>,
}

impl ByteStream {

    pub fn new() -> Self {

        Self {
            raw_stream: join_bytes!(common::ELF64_HEADER, common::ELF64_PROGRAM_HEADER),
            labels: HashMap::new(),
            fixups: Vec::new(),
            block_stack: Vec::new(),
            block_counter: 0,
            data: Vec::new(),
        }

    }

    fn emit(&mut self, bytes: &[u8]) {
        self.raw_stream.extend_from_slice(bytes);
    }

    fn new_label(&mut self, prefix: &str) -> String {
        self.block_counter += 1;
        format!("__{prefix}{}", self.block_counter)
    }

    pub fn process_label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.raw_stream.len());
    }

    pub fn process_jump(&mut self, opcode: &[u8], label: &str) {

        self.emit(opcode);
        self.fixups.push((self.raw_stream.len(), label.to_string()));
        self.emit(&[0; 4]);

    }

    pub fn process_conditional_jump(&mut self, label: &str, condition: &str) {

        let (compare, opcode) = encoder::assemble_compare(condition);

        self.emit(&compare);
        self.process_jump(&opcode, label);

    }

    pub fn process_ret(&mut self) {
        self.emit(&common::RET);
    }

    pub fn open_block(&mut self, name: &str) {

        let end = self.new_label("end");

        self.process_jump(&common::JUMP, &end);
        self.process_label(name);
        self.block_stack.push(end);

    }

    pub fn close_block(&mut self) {

        let end = self.block_stack.pop().unwrap();

        self.process_ret();
        self.process_label(&end);

    }

    pub fn process_register(&mut self, name: &str, value: &str) {
        self.emit(&encoder::assemble_register(common::reg(name), value));
    }

    pub fn process_syscall(&mut self, value: &str) {

        self.process_register("rax", value);
        self.emit(&common::SYSCALL);

    }

    pub fn process_store(&mut self, memory: &str, source: &str) {
        self.emit(&encoder::assemble_memory_operation(&common::STORE, common::reg(source), memory));
    }

    pub fn process_load(&mut self, destination: &str, memory: &str) {
        self.emit(&encoder::assemble_memory_operation(&common::LOAD, common::reg(destination), memory));
    }

    pub fn process_data(&mut self, name: &str, bytes: Vec<u8>) {
        self.data.push((name.to_string(), bytes));
    }

    pub fn process_address(&mut self, register: &str, label: &str) {
        self.process_jump(&[0x48, 0x8D, common::modrm(0b00, common::reg(register), 0b101)], label);
    }

    pub fn process_print(&mut self, text: &str) {

        let (label, length) = match text.strip_prefix('"') {

            Some(literal) => {
                let label = self.new_label("str");
                let bytes = encoder::unescape(literal.trim_end_matches('"'));
                let length = bytes.len();

                self.process_data(&label, bytes);
                (label, length)
            }

            None => {
                let length = self.data.iter().find(|(name, _)| name == text).unwrap().1.len();
                (text.to_string(), length)
            }
        };

        self.process_register("rdi", "1");
        self.process_address("rsi", &label);
        self.process_register("rdx", &length.to_string());
        self.process_syscall("1");

    }

    fn resolve_fixups(&mut self) {

        for (position, label) in &self.fixups {
            let relative = self.labels[label] as i32 - (*position as i32 + 4);
            patch(&mut self.raw_stream, *position, &relative.to_le_bytes());
        }

    }

    pub fn to_file(&mut self, path: &str) {

        for (label, bytes) in std::mem::take(&mut self.data) {
            self.process_label(&label);
            self.emit(&bytes);
        }

        self.resolve_fixups();

        let mut out = self.raw_stream.clone();
        let total = out.len() as u64;
        let base: u64 = 0x400000;

        patch(&mut out, 24, &(base + 0x78).to_le_bytes());
        patch(&mut out, 56, &1u16.to_le_bytes());
        patch(&mut out, 80, &base.to_le_bytes());
        patch(&mut out, 88, &base.to_le_bytes());
        patch(&mut out, 96, &total.to_le_bytes());
        patch(&mut out, 104, &total.to_le_bytes());

        std::fs::write(path, &out).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }

    }

}

fn patch(out: &mut [u8], at: usize, bytes: &[u8]) {
    out[at..at + bytes.len()].copy_from_slice(bytes);
}