mod process;

use crate::archs::target;
use crate::bytestream::ByteStream;
use crate::preprocess;

pub struct BuildContext {
    pub source: String,
    pub exec_path: String,
}

pub struct Driver {
    context: BuildContext,
    isa: &'static target::Isa,
    bytestream: ByteStream,
    statements: Vec<String>,
}

impl Driver {
    pub fn new(context: BuildContext) -> Self {
        let isa = preprocess::get_target_arch(&context.source);

        Self {
            context,
            isa,
            bytestream: ByteStream::new(isa),
            statements: Vec::new(),
        }
    }

    pub fn preprocess(mut self) -> Self {
        self.statements = preprocess::preprocess(&self.context.source, self.isa);

        self
    }

    pub fn process(mut self) -> Self {
        let statements = self.statements;
        self.statements = Vec::new();

        process::run(&mut self, statements);

        self
    }

    pub fn write(mut self) {
        self.bytestream.to_file(&self.context.exec_path);
    }
}
