use crate::append_bytes;
use crate::common::*;

use super::Isa;
use super::parse_expression;
use super::register_operand;
use super::{BitwiseOperator, split_bitwise_expression, strip_outer_parentheses};

pub fn assemble_register(isa: &'static Isa, destination: u8, expression: &str) -> Vec<u8> {
    if split_bitwise_expression(expression).is_some() || expression.trim_start().starts_with('~') {
        return assemble_bitwise_register(isa, destination, expression);
    }

    let (constant, registers) = parse_expression(isa, expression);
    let mut output = Vec::new();

    let coefficient = registers[destination as usize];

    if coefficient == 0 {
        // mov rd, imm32       REX.W C7 /0 id
        append_bytes!(
            output,
            [rex(0, destination), 0xC7, modrm(0b11, 0, destination)]
        );
        append_bytes!(output, constant.to_le_bytes());
    } else {
        if coefficient != 1 {
            // imul rd, rd, imm32    REX.W 69 /r id
            append_bytes!(
                output,
                [
                    rex(destination, destination),
                    0x69,
                    modrm(0b11, destination, destination)
                ],
                coefficient.to_le_bytes()
            );
        }

        // add/sub rd, imm32   REX.W 81 /0 id
        if constant != 0 {
            append_bytes!(
                output,
                [rex(0, destination), 0x81, modrm(0b11, 0, destination)],
                constant.to_le_bytes()
            );
        }
    }

    for (register, &coefficient) in registers.iter().enumerate() {
        if register == destination as usize || coefficient == 0 {
            continue;
        }

        let operation = if coefficient > 0 { 0x01 } else { 0x29 };

        for _ in 0..coefficient.unsigned_abs() {
            append_bytes!(
                output,
                [
                    rex(register as u8, destination),
                    operation,
                    modrm(0b11, register as u8, destination)
                ]
            );
        }
    }

    output
}

fn rex(reg: u8, rm: u8) -> u8 {
    0x48 | ((reg >> 3) << 2) | (rm >> 3)
}

fn assemble_bitwise_register(isa: &'static Isa, destination: u8, expression: &str) -> Vec<u8> {
    assemble_bitwise_register_with_scratch(isa, destination, expression, 11)
}

fn assemble_bitwise_register_with_scratch(
    isa: &'static Isa,
    destination: u8,
    expression: &str,
    scratch: u8,
) -> Vec<u8> {
    let expression = strip_outer_parentheses(expression);

    if let Some(operand) = expression.strip_prefix('~') {
        let mut output = assemble_bitwise_register_with_scratch(isa, destination, operand, scratch);
        append_bytes!(
            output,
            [rex(2, destination), 0xF7, modrm(0b11, 2, destination)]
        ); // not rd
        return output;
    }

    let Some((left, operation, right)) = split_bitwise_expression(expression) else {
        return assemble_register(isa, destination, expression);
    };

    let mut output = assemble_bitwise_register_with_scratch(isa, destination, left, scratch);
    let source = match operation {
        BitwiseOperator::ShiftLeft | BitwiseOperator::ShiftRight => 1, // CL
        _ => scratch,
    };
    let next_scratch = if scratch == 11 { 10 } else { 11 };
    output.extend(assemble_bitwise_register_with_scratch(
        isa,
        source,
        right,
        next_scratch,
    ));

    match operation {
        BitwiseOperator::And => {
            append_bytes!(
                output,
                [
                    rex(source, destination),
                    0x21,
                    modrm(0b11, source, destination)
                ]
            )
        }
        BitwiseOperator::Or => {
            append_bytes!(
                output,
                [
                    rex(source, destination),
                    0x09,
                    modrm(0b11, source, destination)
                ]
            )
        }
        BitwiseOperator::Xor => {
            append_bytes!(
                output,
                [
                    rex(source, destination),
                    0x31,
                    modrm(0b11, source, destination)
                ]
            )
        }
        BitwiseOperator::ShiftLeft => {
            append_bytes!(
                output,
                [rex(4, destination), 0xD3, modrm(0b11, 4, destination)]
            )
        }
        BitwiseOperator::ShiftRight => {
            append_bytes!(
                output,
                [rex(5, destination), 0xD3, modrm(0b11, 5, destination)]
            )
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
        ("==", 0x84),
        ("!=", 0x85),
        ("<=", 0x8E),
        (">=", 0x8D),
        ("<", 0x8C),
        (">", 0x8F),
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
            let immediate: i32 = right
                .trim()
                .parse()
                .expect("right side must be |reg| or integer");

            append_bytes!(
                output,
                [0x81, modrm(0b11, 7, left)],
                immediate.to_le_bytes()
            );
        }
    }

    // jcc rel32 — 0x0F then 0x80+cc; the caller makes the 4-byte displacement
    (output, vec![0x0F, condition_code])
}
