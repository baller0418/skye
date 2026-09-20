use crate::append_bytes;
use crate::common::*;

pub fn assemble_register(destination: u8, expression: &str) -> Vec<u8> {

    fn parse(bytes: &[u8], position: &mut usize, min_precedence: u8) -> (i32, [i32; 8]) {

        skip_whitespace(bytes, position);

        let mut left = match bytes[*position] {

            b'(' => {

                *position += 1;

                let value = parse(bytes, position, 0);

                *position += 1;

                value

            }

            b'|' => {
                *position += 1;

                let start = *position;

                while bytes[*position] != b'|' {

                    *position += 1;

                }

                let name = std::str::from_utf8(&bytes[start..*position]).unwrap().trim();

                *position += 1;

                let register = reg(name) as usize;
                let mut registers = [0; 8];

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

                    [0; 8],
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
                _           => break,

            };

            if precedence < min_precedence {
                break;
            }

            *position += 1;
            let (right_constant, right_registers) =
                parse(bytes, position, precedence + 1);

            match operator {

                b'+' | b'-' => {

                    let sign = if operator == b'+' { 1 } else { -1 };

                    left.0 += right_constant * sign;

                    for i in 0..8 {
                        left.1[i] += right_registers[i] * sign;
                    }

                }

                b'*' => {

                    let (constant, mut registers) = if left.1 == [0; 8] {
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

    let (constant, registers) = parse(expression.as_bytes(), &mut 0, 0);
    let mut output = Vec::new();

    let coefficient = registers[destination as usize];

    if coefficient == 0 {

        append_bytes!(output, [0x48, 0xC7, modrm(0b11, 0, destination)]);
        append_bytes!(output, constant.to_le_bytes());

    } else {

        if coefficient != 1 {

            append_bytes!(
                output,
                [0x48, 0x69, modrm(0b11, destination, destination)],
                coefficient.to_le_bytes()
            );
        }

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

pub fn assemble_memory_operation(operation_bytes: &[u8],register: u8,memory: &str,) -> Vec<u8> {

    let split = memory.find(['+', '-']).unwrap_or(memory.len());

    let base = reg(memory[..split].trim());
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

pub fn assemble_compare(condition: &str) -> (Vec<u8>, [u8; 2]) {

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

    let left = register_operand(left).expect("left side of condition must be |reg|");

    let mut output = vec![0x48];

    match register_operand(right) {
        Some(right) => append_bytes!(output, [0x39, modrm(0b11, right, left)]),
        None => {
            let immediate: i32 = right.trim().parse().expect("right side must be |reg| or integer");
            append_bytes!(output, [0x81, modrm(0b11, 7, left)], immediate.to_le_bytes());
        }
    }

    (output, [0x0F, condition_code])

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

fn register_operand(operand: &str) -> Option<u8> {
    let name = operand.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(reg(name))
}