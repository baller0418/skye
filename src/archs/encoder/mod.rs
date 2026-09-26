pub mod aarch64;
pub mod x86_64;

use crate::target::Isa;

pub fn parse_expression(isa: &'static Isa, expression: &str) -> (i32, [i32; 32]) {
    fn parse(
        isa: &'static Isa,
        bytes: &[u8],
        position: &mut usize,
        min_precedence: u8,
    ) -> (i32, [i32; 32]) {
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

            let (right_constant, right_registers) = parse(isa, bytes, position, precedence + 1);

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

pub(crate) fn register_operand(isa: &'static Isa, operand: &str) -> Option<u8> {
    let name = operand.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(isa.reg(name))
}
