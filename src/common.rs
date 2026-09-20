#![allow(unused)]
#[cfg_attr(rustfmt, rustfmt::skip)]

#[macro_export]
macro_rules! makereg {
    ($(($name:ident, $str:literal, $bytevalue:expr))*) => {


        #[repr(u8)]
        #[derive(Debug)]
        pub enum REGS {
            $(  $name = $bytevalue,  )*
        }

        impl REGS {
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(  $str => Some(REGS::$name),  )*
                    _ => None,
                }
            }
        }


    };
}

#[macro_export]
macro_rules! join_bytes {
    ($($bytes:expr),* $(,)?) => {{

        let mut out = Vec::<u8>::new();
        $(  out.extend_from_slice(&$bytes);  )*
        out

    }};
}

#[macro_export]
macro_rules! append_bytes {
    ($target:expr, $($bytes:expr),* $(,)?) => {{

        $(  $target.extend_from_slice(&$bytes[..]);  )*
        
    }};
}

makereg! {
    (R0, "r0", 0)
    (R1, "r1", 1)
    (R2, "r2", 2)
    (R3, "r3", 3)
    (R4, "r4", 4)
    (R5, "r5", 5)
    (R6, "r6", 6)
    (R7, "r7", 7)
}
pub const ELF64_HEADER: [u8; 64] = [
    0x7f, b'E', b'L', b'F', 
    2,    
    1,    
    1,    
    0,    
    0,    
    0, 0, 0, 0, 0, 0, 0, 

    2, 0,                   
    0x3e, 0,                
    1, 0, 0, 0,             
    0, 0, 0, 0, 0, 0, 0, 0, 
    0x40, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0,             
    0x40, 0,                
    0x38, 0,                
    0, 0,                   
    0x40, 0,                
    0, 0,                   
    0, 0,                   
];

pub const ELF64_PROGRAM_HEADER: [u8; 56] = [
    1, 0, 0, 0,             
    7, 0, 0, 0,             
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0x10, 0, 0, 0, 0, 0, 0, 
];


pub const ADD: [u8; 2] = [0x48, 0x01];
pub const SUB: [u8; 2] = [0x48, 0x29];

pub fn reg(name: &str) -> u8 {
    REGS::from_str(name.trim().trim_matches('|')).unwrap() as u8
}

pub const fn modrm(mode: u8, reg: u8, rm: u8) -> u8 {
    mode << 6 | reg << 3 | rm
}