use crate::archs::target::*;
use crate::bytestream::*;

pub struct BuildContext {
    target: Isa,
    source: String,
    exec_path: String,
}

pub struct Driver {
    context: BuildContext,
    bytestream: ByteStream,
}

impl Driver {
    fn new() -> Self {
        todo!()
    }
}
