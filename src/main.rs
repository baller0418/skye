mod bytestream;
mod common;

use bytestream::ByteStream;

fn main() {

    let source_code: &str = "

    rg rbp 50+3;
    rg rax 123 - |rbp|;
    store [rsp] rax;
    load rsi [rsp];
    syscall 60

    ";

    let mut stream = ByteStream::new();

    let statements: Vec<&str> = source_code
        .trim()
        .split_terminator(';')
        .map(|x| x.trim())
        .collect();

    for statement in statements {

        let words: Vec<&str> = statement.split_whitespace().collect();

        match words[0] {
            "syscall" => stream.process_syscall(words[1]),
            "load" => stream.process_load(
                words[1],
                strip_parenthesis(words[2], ('[', ']')),
            ),
            "store" => stream.process_store(
                strip_parenthesis(words[1], ('[', ']')),
                words[2]),
            "rg" => stream.process_register(words[1], words[2..].concat().as_str()),
            _ => {}
        }

    }

    stream.to_file("a.out");
}

fn strip_parenthesis(string: &str, parens: (char, char)) -> &str {
    string
        .trim_start_matches(parens.0)
        .trim_end_matches(parens.1)
}