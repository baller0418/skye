mod bytestream;
mod common;
mod encoder;
mod preprocess;
mod target;

use bytestream::ByteStream;

fn main() {

    let source_code = std::fs::read_to_string(
        std::env::args().nth(1).unwrap_or("prog.src".into())
    ).unwrap();

    let (isa, statements) = preprocess::preprocess(&source_code);
    let mut stream = ByteStream::new(isa);

    for statement in statements {

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

            "direct" => match rest.strip_suffix('{') {
                Some(name) => stream.open_block(name.trim()),
                None => stream.process_label(rest),
            },

            "jump" => match rest.split_once(" if ") {
                Some((label, condition)) => stream.process_conditional_jump(label, condition),
                None => stream.process_jump(isa.JUMP, rest),
            },

            "data" => {
                let (name, value) = rest.split_once(' ').unwrap();
                stream.process_data(name, encoder::parse_data(value.trim()));
            }

            "}"     => stream.close_block(),
            "reg"   => stream.process_register(words[1], &words[2..].concat()),
            "call"  => stream.process_jump(isa.CALL, rest),
            "ret"   => stream.process_ret(),
            "print" => stream.process_print(rest),
            "space" => stream.process_data(words[1], vec![0; words[2].parse().unwrap()]),
            "addr"  => stream.process_address(words[1], words[2]),

            _ => {}
        }
    }

    stream.to_file("out");

}

fn split_memory(text: &str) -> (&str, &str, &str) {

    let (before, rest) = text.split_once('[').unwrap();
    let (inside, after) = rest.split_once(']').unwrap();

    (before.trim(), inside.trim(), after.trim())
}