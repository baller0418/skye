pub mod aarch64;
pub mod x86_64;

pub use crate::archs::target::Isa;

#[derive(Clone, Copy)]
pub enum BitwiseOperator {
    And,
    Or,
    Xor,
    ShiftLeft,
    ShiftRight,
}

/// Returns the lowest-precedence top-level bitwise operation in an expression.
/// Register operands use `|name|`, so a `|` only counts as OR when it is not
/// the delimiter of such an operand.
pub fn split_bitwise_expression(expression: &str) -> Option<(&str, BitwiseOperator, &str)> {
    for (needle, operator) in [
        (b'|', BitwiseOperator::Or),
        (b'^', BitwiseOperator::Xor),
        (b'&', BitwiseOperator::And),
    ] {
        if let Some(index) = find_top_level(expression.as_bytes(), needle, false) {
            return Some((&expression[..index], operator, &expression[index + 1..]));
        }
    }

    for (needle, operator) in [
        (b'<', BitwiseOperator::ShiftLeft),
        (b'>', BitwiseOperator::ShiftRight),
    ] {
        if let Some(index) = find_top_level(expression.as_bytes(), needle, true) {
            return Some((&expression[..index], operator, &expression[index + 2..]));
        }
    }

    None
}

pub fn strip_outer_parentheses(expression: &str) -> &str {
    let expression = expression.trim();
    if !expression.starts_with('(') || !expression.ends_with(')') {
        return expression;
    }

    let mut depth = 0;
    for (index, byte) in expression.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 && index + 1 != expression.len() {
                    return expression;
                }
            }
            _ => {}
        }
    }

    &expression[1..expression.len() - 1]
}

fn find_top_level(bytes: &[u8], needle: u8, doubled: bool) -> Option<usize> {
    let mut result = None;
    let mut depth = 0;
    let mut in_register = false;
    let mut position = 0;

    while position < bytes.len() {
        match bytes[position] {
            b'(' if !in_register => depth += 1,
            b')' if !in_register => depth -= 1,
            b'|' if needle == b'|' && !in_register && depth == 0 => {
                // A pipe begins a register only when the next pipe encloses a name.
                let next = bytes[position + 1..].iter().position(|&b| b == b'|');
                if next.is_some_and(|end| {
                    !bytes[position + 1..position + 1 + end]
                        .iter()
                        .any(|b| b.is_ascii_whitespace() || matches!(*b, b'&' | b'^' | b'(' | b')'))
                }) {
                    in_register = true;
                } else {
                    result = Some(position);
                }
            }
            b'|' if in_register => in_register = false,
            byte if !in_register && depth == 0 && byte == needle => {
                if !doubled || bytes.get(position + 1) == Some(&needle) {
                    result = Some(position);
                    position += usize::from(doubled);
                }
            }
            _ => {}
        }

        position += 1;
    }

    result
}

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
