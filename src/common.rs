#[macro_export]
macro_rules! append_bytes {
    ($target:expr, $($bytes:expr),* $(,)?) => {{

        $(  $target.extend_from_slice(&$bytes[..]);  )*

    }};
}

pub const ELF64_HEADER: [u8; 64] = [
    0x7f, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0x3e, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0x38, 0,
    0, 0, 0x40, 0, 0, 0, 0, 0,
];

pub const ELF64_PROGRAM_HEADER: [u8; 56] = [
    1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0, 0, 0, 0, 0, 0,
];

pub const fn modrm(mode: u8, reg: u8, rm: u8) -> u8 {
    mode << 6 | (reg & 7) << 3 | (rm & 7)
}
