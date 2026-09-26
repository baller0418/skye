use crate::target::Isa;

use super::parse_expression;
use super::register_operand;

fn word(out: &mut Vec<u8>, w: u32) {
    out.extend_from_slice(&w.to_le_bytes());
}

pub fn assemble_register(isa: &'static Isa, destination: u8, expression: &str) -> Vec<u8> {
    assert!(
        destination != 31,
        "aarch64: cannot use stack as an arithmetic destination"
    );

    let (constant, registers) = parse_expression(isa, expression);
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
            word(
                &mut output,
                0xD280_0000 | ((coefficient.unsigned_abs() & 0xFFFF) << 5) | 9,
            );
            word(&mut output, 0x9B00_7C00 | (9 << 16) | (d << 5) | d);
        }

        if constant != 0 {
            let base = if constant > 0 {
                0x9100_0000
            } else {
                0xD100_0000
            };

            word(
                &mut output,
                base | ((constant.unsigned_abs() & 0xFFF) << 10) | (d << 5) | d,
            );
        }
    }

    for (register, &coefficient) in registers.iter().enumerate() {
        if register == destination as usize || coefficient == 0 {
            continue;
        }

        let base = if coefficient > 0 {
            0x8B00_0000
        } else {
            0xCB00_0000
        };

        let m = register as u32;

        for _ in 0..coefficient.unsigned_abs() {
            word(&mut output, base | (m << 16) | (d << 5) | d);
        }
    }

    output
}

pub fn assemble_memory_operation(
    isa: &'static Isa,
    operation_bytes: &[u8],
    register: u8,
    memory: &str,
) -> Vec<u8> {
    let base_word = u32::from_le_bytes(operation_bytes.try_into().unwrap());

    let split = memory.find(['+', '-']).unwrap_or(memory.len());
    let base = isa.reg(memory[..split].trim()) as u32;
    let displacement: i32 = memory[split..].replace(' ', "").parse().unwrap_or(0);

    assert!(
        displacement >= 0 && displacement % 8 == 0,
        "aarch64: unscaled displacement"
    );

    let mut output = Vec::new();

    word(
        &mut output,
        base_word | (((displacement as u32) / 8) << 10) | (base << 5) | register as u32,
    );

    output
}

pub fn assemble_address(_isa: &'static Isa, register: u8) -> Vec<u8> {
    let mut output = Vec::new();

    word(&mut output, 0x1000_0000 | register as u32);

    output
}

pub fn assemble_compare(isa: &'static Isa, condition: &str) -> (Vec<u8>, Vec<u8>) {
    const OPERATORS: [(&str, u32); 6] = [
        ("==", 0),
        ("!=", 1),
        ("<=", 13),
        (">=", 10),
        ("<", 11),
        (">", 12),
    ];

    let &(operator, cond) = OPERATORS
        .iter()
        .find(|(operator, _)| condition.contains(operator))
        .expect("condition needs a comparison operator");

    let (left, right) = condition.split_once(operator).unwrap();

    let n = register_operand(isa, left).expect("left side of condition must be |reg|") as u32;

    let mut compare = Vec::new();

    match register_operand(isa, right) {
        Some(m) => {
            word(&mut compare, 0xEB00_001F | ((m as u32) << 16) | (n << 5));
        }

        None => {
            let immediate: u32 = right
                .trim()
                .parse()
                .expect("right side must be |reg| or integer");

            word(
                &mut compare,
                0xF100_001F | ((immediate & 0xFFF) << 10) | (n << 5),
            );
        }
    }

    let mut branch = Vec::new();

    word(&mut branch, 0x5400_0000 | cond);

    (compare, branch)
}
