use crate::append_bytes;
use crate::common::*;
use crate::target::Isa;

pub fn assemble_register(isa: &'static Isa, destination: u8, expression: &str) -> Vec<u8> {

    let (constant, registers) = parse_expression(isa, expression);
    let mut output = Vec::new();

    let coefficient = registers[destination as usize];

    if coefficient == 0 {
        // mov rd, imm32       REX.W C7 /0 id
        append_bytes!(output, [0x48, 0xC7, modrm(0b11, 0, destination)]);
        append_bytes!(output, constant.to_le_bytes());

    } else {

        if coefficient != 1 {
        // imul rd, rd, imm32    REX.W 69 /r id
            append_bytes!(
                output,
                [0x48, 0x69, modrm(0b11, destination, destination)],
                coefficient.to_le_bytes()
            );
        }
        // add/sub rd, imm32   REX.W 81 /0 id
        if constant != 0 {
            append_bytes!(  output, [0x48, 0x81, modrm(0b11, 0, destination)], constant.to_le_bytes()  );
        }
    }

    for (register, &coefficient) in registers.iter().enumerate() {

        if register == destination as usize || coefficient == 0 {
            continue;
        }

        let operation = match 
        coefficient > 0 {
            true => ADD,
            false => SUB,
        };

        for _ in 0..coefficient.unsigned_abs() {
            append_bytes!(
                output,
                operation,
                [modrm(0b11, register as u8, destination)]
            );
        }

    }

    output
}

pub fn parse_expression(isa: &'static Isa, expression: &str) -> (i32, [i32; 32]) {

    fn parse(isa: &'static Isa, bytes: &[u8], position: &mut usize, min_precedence: u8,) -> (i32, [i32; 32]) {

        skip_whitespace(bytes, position);

        let mut left = match bytes[*position] {

            b'(' => {
                *position += 1;

                let value = parse(isa, bytes, position, 0);

                *position += 1;

                value
            }

            b'|' => {
                *position += 1;

                let start = *position;

                while bytes[*position] != b'|' {
                    *position += 1;
                }

                let name = std::str::from_utf8(&bytes[start..*position])
                    .unwrap()
                    .trim();

                *position += 1;

                let register = isa.reg(name) as usize;
                let mut registers = [0; 32];

                registers[register] = 1;

                (0, registers)
            }

            _ => {
                let start = *position;

                while bytes.get(*position).is_some_and(u8::is_ascii_digit) {
                    *position += 1;
                }

                (
                    std::str::from_utf8(&bytes[start..*position])
                        .unwrap()
                        .parse()
                        .unwrap(),

                    [0; 32],
                )
            }
        };

        loop {

            skip_whitespace(bytes, position);

            let Some(&operator) = bytes.get(*position) else {
                break;
            };

            let precedence = match operator {
                b'+' | b'-' => 1,
                b'*' | b'/' => 2,
                b')' | b'|' => break,
                _ => break,
            };

            if precedence < min_precedence {
                break;
            }

            *position += 1;

            let (right_constant, right_registers) =
                parse(isa, bytes, position, precedence + 1);

            match operator {

                b'+' | b'-' => {
                    let sign = if operator == b'+' { 1 } else { -1 };

                    left.0 += right_constant * sign;

                    for (slot, right) in left.1.iter_mut().zip(right_registers) {
                        *slot += right * sign;
                    }
                }

                b'*' => {
                    let (constant, mut registers) = if left.1 == [0; 32] {

                        (left.0, right_registers)

                    } else {

                        (right_constant, left.1)

                    };

                    for register in &mut registers {

                        *register *= constant;

                    }

                    left = (left.0 * right_constant, registers);
                }

                b'/' => {
                    for register in &mut left.1 {

                        *register /= right_constant;

                    }

                    left.0 /= right_constant;
                }

                _ => unreachable!(),
            }
        }

        left
    }

    parse(isa, expression.as_bytes(), &mut 0, 0)
}

pub fn assemble_memory_operation(isa: &'static Isa, operation_bytes: &[u8], register: u8, memory: &str) -> Vec<u8> {

    let split = memory.find(['+', '-']).unwrap_or(memory.len());

    let base = isa.reg(memory[..split].trim());
    let displacement: i32 = memory[split..].replace(' ', "").parse().unwrap_or(0);

    let mode = match displacement {
        0 if base != 5 => 0b00,
        -128..=127 => 0b01,
        _ => 0b10,
    };

    let mut output = operation_bytes.to_vec();

    append_bytes!(output, [modrm(mode, register, base)]);

    if base == 4 {
        append_bytes!(output, [0x24]);
    }

    match mode {

        0b01 => append_bytes!(output, [displacement as i8 as u8]),
        0b10 => append_bytes!(output, displacement.to_le_bytes()),
        _ => {}

    }

    output
}

// lea rd, [rip + disp32]    REX.W 8D /r, mod=00 rm=101 selects RIP-relative
pub fn assemble_address(_isa: &'static Isa, register: u8) -> Vec<u8> {
    vec![0x48, 0x8D, modrm(0b00, register, 0b101)]
}

pub fn assemble_compare(isa: &'static Isa, condition: &str) -> (Vec<u8>, Vec<u8>) {

    // two char operators first so "<=" isn't matched as "<"
    const OPERATORS: [(&str, u8); 6] = [
        ("==", 0x84), ("!=", 0x85), ("<=", 0x8E),
        (">=", 0x8D), ("<",  0x8C), (">",  0x8F),
    ];

    let &(operator, condition_code) = OPERATORS
        .iter()
        .find(|(operator, _)| condition.contains(operator))
        .expect("condition needs a comparison operator");

    let (left, right) = condition.split_once(operator).unwrap();

    let left = register_operand(isa, left).expect("left side of condition must be |reg|");

    let mut output = vec![0x48];

    match register_operand(isa, right) {
        Some(right) => append_bytes!(output, [0x39, modrm(0b11, right, left)]),
        None => {
            let immediate: i32 = right.trim().parse().expect("right side must be |reg| or integer");
            append_bytes!(output, [0x81, modrm(0b11, 7, left)], immediate.to_le_bytes());
        }
    }

    // jcc rel32 — 0x0F then 0x80+cc; the caller makes the 4-byte displacement
    (output, vec![0x0F, condition_code])

}

pub fn unescape(text: &str) -> Vec<u8> {

    let mut bytes = Vec::new();
    let mut chars = text.chars();

    while let Some(c) = chars.next() {

        match c {
            '\\' => bytes.push(match chars.next().unwrap() {
                'n' => b'\n',
                't' => b'\t',
                '0' => 0,
                other => other as u8,
            }),
            other => bytes.extend_from_slice(other.to_string().as_bytes()),
        }
    }

    bytes
}

pub fn parse_data(value: &str) -> Vec<u8> {

    if let Some(text) = value.strip_prefix('"') {
        return unescape(text.trim_end_matches('"'));
    }

    value
        .split_whitespace()
        .flat_map(|number| number.parse::<i64>().unwrap().to_le_bytes())
        .collect()
}

fn skip_whitespace(bytes: &[u8], position: &mut usize) {

    while bytes.get(*position).is_some_and(u8::is_ascii_whitespace) {
        *position += 1;
    }
}

fn register_operand(isa: &'static Isa, operand: &str) -> Option<u8> {
    let name = operand.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(isa.reg(name))
}

pub mod arm {

    use crate::target::Isa;

    fn word(out: &mut Vec<u8>, w: u32) {
        out.extend_from_slice(&w.to_le_bytes());
    }

    pub fn assemble_register(isa: &'static Isa, destination: u8, expression: &str) -> Vec<u8> {

        assert!(destination != 31, "aarch64: cannot use stack as an arithmetic destination");


        let (constant, registers) = crate::encoder::parse_expression(isa, expression);
        let mut output = Vec::new();

        let d = destination as u32;

        let coefficient = registers[destination as usize];

        if coefficient == 0 {

            if constant < 0 {

                let inverted = !(constant as u32);
                word(&mut output, 0x9280_0000 | ((inverted & 0xFFFF) << 5) | d);

            } else {

                let value = constant as u32;
                word(&mut output, 0xD280_0000 | ((value & 0xFFFF) << 5) | d);

                if value >> 16 != 0 {
                    word(&mut output, 0xF2A0_0000 | ((value >> 16) << 5) | d);
                }
            }
        } else {

            if coefficient != 1 {
                word(&mut output, 0xD280_0000 | ((coefficient.unsigned_abs() & 0xFFFF) << 5) | 9);
                word(&mut output, 0x9B00_7C00 | (9 << 16) | (d << 5) | d);
            }

            if constant != 0 {
                let base = if constant > 0 { 0x9100_0000 } else { 0xD100_0000 };
                word(&mut output, base | ((constant.unsigned_abs() & 0xFFF) << 10) | (d << 5) | d);
            }
        }

        for (register, &coefficient) in registers.iter().enumerate() {

            if register == destination as usize || coefficient == 0 {
                continue;
            }

            let base = if coefficient > 0 { 0x8B00_0000 } else { 0xCB00_0000 };
            let m = register as u32;

            for _ in 0..coefficient.unsigned_abs() {
                word(&mut output, base | (m << 16) | (d << 5) | d);
            }
        }

        output
    }

    pub fn assemble_memory_operation(isa: &'static Isa, operation_bytes: &[u8], register: u8, memory: &str) -> Vec<u8> {

        let base_word = u32::from_le_bytes(operation_bytes.try_into().unwrap());

        let split = memory.find(['+', '-']).unwrap_or(memory.len());
        let base = isa.reg(memory[..split].trim()) as u32;
        let displacement: i32 = memory[split..].replace(' ', "").parse().unwrap_or(0);

        assert!(displacement >= 0 && displacement % 8 == 0, "aarch64: unscaled displacement");

        let mut output = Vec::new();
        word(&mut output, base_word | (((displacement as u32) / 8) << 10) | (base << 5) | register as u32);
        output
    }

    pub fn assemble_address(_isa: &'static Isa, register: u8) -> Vec<u8> {
        let mut output = Vec::new();
        word(&mut output, 0x1000_0000 | register as u32);
        output
    }

    pub fn assemble_compare(isa: &'static Isa, condition: &str) -> (Vec<u8>, Vec<u8>) {

        const OPERATORS: [(&str, u32); 6] = [
            ("==", 0), ("!=", 1), ("<=", 13),
            (">=", 10), ("<", 11), (">", 12),
        ];

        let &(operator, cond) = OPERATORS
            .iter()
            .find(|(operator, _)| condition.contains(operator))
            .expect("condition needs a comparison operator");

        let (left, right) = condition.split_once(operator).unwrap();

        let n = crate::encoder::register_operand(isa, left)
            .expect("left side of condition must be |reg|") as u32;

        let mut compare = Vec::new();

        match crate::encoder::register_operand(isa, right) {
            Some(m) => word(&mut compare, 0xEB00_001F | ((m as u32) << 16) | (n << 5)),
            None => {
                let immediate: u32 = right.trim().parse().expect("right side must be |reg| or integer");
                word(&mut compare, 0xF100_001F | ((immediate & 0xFFF) << 10) | (n << 5));
            }
        }

        let mut branch = Vec::new();

        word(&mut branch, 0x5400_0000 | cond);

        (compare, branch)
    }
}