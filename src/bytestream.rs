use crate::append_bytes;
use crate::common;
use crate::join_bytes;

pub struct ByteStream {
    raw_stream: Vec<u8>,
}

impl ByteStream {

    pub fn new() -> Self {

        Self {
            raw_stream: join_bytes!(common::ELF64_HEADER, common::ELF64_PROGRAM_HEADER),
        }

    }

    pub fn process_register(&mut self, name: &str, value: &str) {

        let dst = common::REGS::from_str(name).unwrap() as u8;
        self.raw_stream.extend(common::assemble_register(dst, value));

        println!("{:?}", self.raw_stream);

    }

    pub fn process_syscall(&mut self, value: &str) {

        self.process_register("rax", value);
        
        append_bytes!(self.raw_stream, common::SYSCALL,);

    }

    pub fn process_store(&mut self, memory: &str, source: &str) {

        let source = common::REGS::from_str(source).unwrap() as u8;
        let memory = common::REGS::from_str(memory).unwrap() as u8;

        append_bytes!(
            self.raw_stream,
            common::assemble_memory_operation(
                &common::STORE,
                source,
                memory,
            )
        );

    }

    pub fn process_load(&mut self, destination: &str, memory: &str) {

        let destination = common::REGS::from_str(destination).unwrap() as u8;
        let memory = common::REGS::from_str(memory).unwrap() as u8;

        append_bytes!(
            self.raw_stream,
            common::assemble_memory_operation(
                &common::LOAD,
                destination,
                memory,
            )
        );

    }

    pub fn to_file(&self, path: &str) {

        let mut out = self.raw_stream.clone();
        let total = out.len() as u64;
        let base: u64 = 0x400000;

        out[24..32].copy_from_slice(&(base + 0x78).to_le_bytes());
        out[56..58].copy_from_slice(&1u16.to_le_bytes());
        out[80..88].copy_from_slice(&base.to_le_bytes());
        out[88..96].copy_from_slice(&base.to_le_bytes());
        out[96..104].copy_from_slice(&total.to_le_bytes());
        out[104..112].copy_from_slice(&total.to_le_bytes());

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
