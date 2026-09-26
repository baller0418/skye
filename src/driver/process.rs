use super::Driver;
use crate::archs::encoder;
use crate::preprocess;
use std::collections::HashMap;

type Functions = HashMap<String, (Vec<String>, Option<String>, Vec<String>)>;

pub fn run(driver: &mut Driver, statements: Vec<String>) {
    let mut functions: Functions = HashMap::new();
    let mut collecting: Option<String> = None;

    for statement in statements {
        let words: Vec<&str> = statement.split_whitespace().collect();

        if let Some(name) = &collecting {
            if words[0] == "}" {
                collecting = None;
            } else {
                functions.get_mut(name).unwrap().2.push(statement);
            }

            continue;
        }

        if let Some(head) = statement
            .strip_prefix("direct")
            .and_then(|r| r.trim().strip_suffix('{'))
        {
            let (name, tail) = head.trim().split_once('(').unwrap_or((head, ""));
            let (params, returns) = match tail.split_once(')') {
                Some((params, tail)) => (
                    params
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                    tail.split_once("->").map(|(_, r)| r.trim().to_string()),
                ),
                None => (Vec::new(), None),
            };

            let name = name.trim().to_string();
            functions.insert(name.clone(), (params, returns, Vec::new()));
            collecting = Some(name);

            continue;
        }

        process(driver, &statement, &functions, 0);
    }
}

fn process(driver: &mut Driver, statement: &str, functions: &Functions, depth: usize) {
    assert!(depth < 64, "call nested too deeply (recursive function?)");

    let words: Vec<&str> = statement.split_whitespace().collect();
    let rest = statement[words[0].len()..].trim();

    match words[0] {
        "syscall" => {
            for (register, argument) in driver.isa.SYSCALL_ARGS.iter().zip(&words[2..]) {
                driver.bytestream.process_register(register, argument);
            }

            driver.bytestream.process_syscall(words[1]);
        }

        "load" => {
            let (destination, memory, _) = split_memory(rest);
            driver.bytestream.process_load(destination, memory);
        }

        "store" => {
            let (_, memory, source) = split_memory(rest);
            driver.bytestream.process_store(memory, source);
        }

        "direct" => driver.bytestream.process_label(rest),

        "jump" => match rest.split_once(" if ") {
            Some((label, condition)) => {
                driver.bytestream.process_conditional_jump(label, condition);
            }

            None => driver.bytestream.process_jump(driver.isa.JUMP, rest),
        },

        "data" => {
            let (name, value) = rest.split_once(' ').unwrap();

            driver
                .bytestream
                .process_data(name, encoder::parse_data(value.trim()));
        }

        "call" => {
            let (head, output) = match rest.split_once("->") {
                Some((head, output)) => (head.trim(), Some(output.trim())),
                None => (rest, None),
            };

            let words: Vec<&str> = head.split_whitespace().collect();
            let (name, arguments) = (words[0], &words[1..]);

            let (params, returns, body) = functions
                .get(name)
                .unwrap_or_else(|| panic!("unknown function: {name}"));

            assert!(
                params.len() == arguments.len(),
                "{name} takes {} arguments, got {}",
                params.len(),
                arguments.len()
            );

            let mut map: HashMap<&str, &str> = params
                .iter()
                .map(String::as_str)
                .zip(arguments.iter().copied())
                .collect();

            if let (Some(returns), Some(output)) = (returns, output) {
                map.insert(returns.as_str(), output);
            }

            for statement in body {
                let expanded = preprocess::substitute(statement, &map);
                process(driver, &expanded, functions, depth + 1);
            }
        }

        "}" => driver.bytestream.close_block(),
        "reg" => driver
            .bytestream
            .process_register(words[1], &words[2..].concat()),
        "ret" => driver.bytestream.process_ret(),
        "print" => driver.bytestream.process_print(rest),
        "space" => driver
            .bytestream
            .process_data(words[1], vec![0; words[2].parse().unwrap()]),
        "addr" => driver.bytestream.process_address(words[1], words[2]),

        _ => {}
    }
}

fn split_memory(text: &str) -> (&str, &str, &str) {
    let (before, rest) = text.split_once('[').unwrap();
    let (inside, after) = rest.split_once(']').unwrap();

    (before.trim(), inside.trim(), after.trim())
}
