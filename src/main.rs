mod bytestream;
mod common;
mod encoder;
mod preprocess;
mod target;

use bytestream::ByteStream;
use std::collections::HashMap;

type Functions = HashMap<String, (Vec<String>, Option<String>, Vec<String>)>;

fn main() {

    let source_code = std::fs::read_to_string(
        std::env::args().nth(1).unwrap_or("prog.src".into())
    ).unwrap();

    let (isa, statements) = preprocess::preprocess(&source_code);
    let mut stream = ByteStream::new(isa);

    let mut functions: Functions = HashMap::new();
    let mut collecting: Option<String> = None;

    for statement in statements {

        let words: Vec<&str> = statement.split_whitespace().collect();

        if let Some(name) = &collecting {

            if words[0] == "}" {
                collecting = None;
            } else {
                functions.get_mut(name).unwrap().2.push(statement.clone());
            }

            continue;
        }

        if let Some(head) = statement.strip_prefix("direct").and_then(|r| r.trim().strip_suffix('{')) {

            let head = head.trim();
            let (name, tail) = head.split_once('(').unwrap_or((head, ""));

            let (params, returns): (Vec<String>, Option<String>) = match tail.split_once(')') {
                Some((params, tail)) => (
                    params.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
                    tail.split_once("->").map(|(_, r)| r.trim().to_string()),
                ),
                None => (Vec::new(), None),
            };

            let name = name.trim().to_string();

            functions.insert(name.clone(), (params, returns, Vec::new()));
            collecting = Some(name);

            continue;
        }

        assemble(&statement, &mut stream, &functions, isa, 0);
    }

    stream.to_file("out");
}

fn assemble(
    statement: &str,
    stream: &mut ByteStream,
    functions: &Functions,
    isa: &'static target::Isa,
    depth: usize,
) {

    assert!(depth < 64, "call nested too deeply (recursive function?)");

    let words: Vec<&str> = statement.split_whitespace().collect();
    let rest = statement[words[0].len()..].trim();

    match words[0] {

        "syscall" => {
            for (register, argument) in isa.SYSCALL_ARGS.iter().zip(&words[2..]) {
                stream.process_register(register, argument);
            }

            stream.process_syscall(words[1]);
        }

        "load" => {
            let (destination, memory, _) = split_memory(rest);
            stream.process_load(destination, memory);
        }

        "store" => {
            let (_, memory, source) = split_memory(rest);
            stream.process_store(memory, source);
        }

        "direct" => stream.process_label(rest),

        "jump" => match rest.split_once(" if ") {
            Some((label, condition)) => stream.process_conditional_jump(label, condition),
            None => stream.process_jump(isa.JUMP, rest),
        },

        "data" => {
            let (name, value) = rest.split_once(' ').unwrap();
            stream.process_data(name, encoder::parse_data(value.trim()));
        }

        "call" => {

            let (head, output) = match rest.split_once("->") {
                Some((head, output)) => (head.trim(), Some(output.trim())),
                None => (rest, None),
            };

            let words: Vec<&str> = head.split_whitespace().collect();
            let (name, arguments) = (words[0], &words[1..]);

            let (params, returns, body) = functions.get(name)
                .unwrap_or_else(|| panic!("unknown function: {name}"));

            assert!(
                params.len() == arguments.len(),
                "{name} takes {} arguments, got {}", params.len(), arguments.len()
            );

            let mut map: HashMap<&str, &str> = params.iter()
                .map(String::as_str)
                .zip(arguments.iter().copied())
                .collect();

            if let (Some(returns), Some(output)) = (returns, output) {
                map.insert(returns.as_str(), output);
            }

            for statement in body {
                let expanded = preprocess::substitute(statement, &map);
                assemble(&expanded, stream, functions, isa, depth + 1);
            }
        }

        "}"     => stream.close_block(),
        "reg"   => stream.process_register(words[1], &words[2..].concat()),
        "ret"   => stream.process_ret(),
        "print" => stream.process_print(rest),
        "space" => stream.process_data(words[1], vec![0; words[2].parse().unwrap()]),
        "addr"  => stream.process_address(words[1], words[2]),

        _ => {}
    }
}

fn split_memory(text: &str) -> (&str, &str, &str) {

    let (before, rest) = text.split_once('[').unwrap();
    let (inside, after) = rest.split_once(']').unwrap();

    (before.trim(), inside.trim(), after.trim())
}