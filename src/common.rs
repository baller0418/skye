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
    (RAX, "rax", 0)
    (RCX, "rcx", 1)
    (RDX, "rdx", 2)
    (RBX, "rbx", 3)
    (RSP, "rsp", 4)
    (RBP, "rbp", 5)
    (RSI, "rsi", 6)
    (RDI, "rdi", 7)
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
    5, 0, 0, 0,             
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0, 0, 0, 0, 0, 0, 0, 
    0, 0x10, 0, 0, 0, 0, 0, 0, 
];

pub const MOV: [u8; 3] = [0x48, 0xC7, 0xC0];
pub const SYSCALL: [u8; 2] = [0x0F, 0x05];

pub const ADD: [u8; 2] = [0x48, 0x01];
pub const SUB: [u8; 2] = [0x48, 0x29];

pub const STORE: [u8; 2] = [0x48, 0x89];
pub const LOAD: [u8; 2] = [0x48, 0x8B];



pub fn assemble_register(dst: u8, input: &str) -> Vec<u8> {

    fn parse(s: &[u8], pos: &mut usize, min_prec: u8) -> (i32, [i32; 8]) {

        let ws = |p: &mut usize| while *p < s.len() && s[*p].is_ascii_whitespace() { *p += 1 };

        ws(pos);

        let mut lhs = if s[*pos] == b'(' {
            *pos += 1;
            let v = parse(s, pos, 0);
            *pos += 1; 
            v
        } else if s[*pos] == b'|' {
            *pos += 1;
            let start = *pos;
            while s[*pos] != b'|' { *pos += 1; }
            let name = std::str::from_utf8(&s[start..*pos]).unwrap().trim();
            *pos += 1; 
            let code = REGS::from_str(name)
                .unwrap_or_else(|| panic!("unknown register: {name}")) as usize;
            let mut regs = [0; 8];
            regs[code] = 1;
            (0, regs)
        } else {
            let start = *pos;
            while *pos < s.len() && s[*pos].is_ascii_digit() { *pos += 1; }
            (std::str::from_utf8(&s[start..*pos]).unwrap().parse().unwrap(), [0; 8])
        };

        loop {
            ws(pos);
            if *pos >= s.len() || s[*pos] == b')' || s[*pos] == b'|' { break; }

            let prec = match s[*pos] { b'+' | b'-' => 1, b'*' | b'/' => 2, _ => break };
            if prec < min_prec { break; }

            let op = s[*pos];
            *pos += 1;
            let (rc, rr) = parse(s, pos, prec + 1);
            let (lc, mut lr) = lhs;

            lhs = match op {
                b'+' => { for i in 0..8 { lr[i] += rr[i]; } (lc + rc, lr) }
                b'-' => { for i in 0..8 { lr[i] -= rr[i]; } (lc - rc, lr) }
                b'*' => {
                    
                    let (k, mut v) = if lr == [0; 8] { (lc, rr) } else { (rc, lr) };
                    for i in 0..8 { v[i] *= k; }
                    (lc * rc, v)
                }
                b'/' => { for i in 0..8 { lr[i] /= rc; } (lc / rc, lr) }
                _ => unreachable!(),
            };
        }

        lhs
    }

    let (constant, regs) = parse(input.as_bytes(), &mut 0, 0);
    let mut out = MOV.to_vec();

    out[2] += dst;
    out.extend_from_slice(&constant.to_le_bytes());

    for (code, &coeff) in regs.iter().enumerate() {

        let modrm = 0xC0 | ((code as u8) << 3) | dst;

        match coeff {
            1  => { out.extend_from_slice(&ADD); out.push(modrm); }
            -1 => { out.extend_from_slice(&SUB); out.push(modrm); }
            _  => {}
        }

    }

    out
}

pub fn assemble_memory_operation(operation_bytes: &[u8], register: u8, memory_register: u8) -> Vec<u8> {

    let mut out = operation_bytes.to_vec();
    let reg = register << 3;

    match memory_register {
        4 => {  out.push(reg | 0b100); out.push(0x24);  } 
        5 => {  out.push(0x40 | reg | 0b101); out.push(0x00);  }
        _ => out.push(reg | memory_register),
    }

    out

}