use std::collections::HashMap;

type Macros<'a> = HashMap<&'a str, (Vec<&'a str>, &'a str)>;

pub fn preprocess(source: &str) -> Vec<String> {

    let source = char_literals(&strip_comments(source));
    let mut macros: Macros = HashMap::new();
    let mut code = String::new();

    for line in source.lines() {

        match line.trim().strip_prefix("#def") {

            Some(definition) => {

                let (head, body) = definition.split_once('@').unwrap();
                let mut head = head.split_whitespace();

                macros.insert(head.next().unwrap(), (head.collect(), body.trim()));
            }

            None => {

                code.push_str(line);
                code.push('\n');

            }
        }
    }

    let mut string_buffer = Vec::new();

    for statement in split_statements(&code) {
        expand(statement, &macros, &mut string_buffer);
    }

    string_buffer
}

fn expand(statement: &str, macros: &Macros, string_buffer: &mut Vec<String>) {

    let words: Vec<&str> = statement.split_whitespace().collect();

    if let Some((params, body)) = macros.get(words[0]) {

        let arguments: HashMap<&str, &str> = params.iter().copied()
            .zip(words[1..].iter().copied())
            .collect();

        let body = substitute(body, &arguments);

        for inner in split_statements(&body) {
            expand(inner, macros, string_buffer);
        }

        return;
    }

    let constants: HashMap<&str, &str> = macros.iter()
        .filter(|(_, (params, _))| params.is_empty())
        .map(|(name, (_, body))| (*name, *body))
        .collect();

    string_buffer.push(substitute(statement, &constants));

}

fn split_statements(source: &str) -> impl Iterator<Item = &str> {

    source
        .split_inclusive(|c| matches!(c, ';' | '{' | '}'))
        .map(|s| s.trim().trim_end_matches(';').trim())
        .filter(|s| !s.is_empty())

}

fn strip_comments(source: &str) -> String {

    let mut string_buffer = String::new();
    let mut chars = source.chars().peekable();
    let mut depth = 0;

    while let Some(c) = chars.next() {

        match (c, chars.peek()) {

            ('/', Some('*')) => { chars.next(); depth += 1; }
            ('*', Some('/')) if depth > 0 => { chars.next(); depth -= 1; string_buffer.push(' '); }
            ('/', Some('/')) if depth == 0 => while chars.next_if(|&c| c != '\n').is_some() {},

            _ if depth > 0 => {}
            _ => string_buffer.push(c),
        }
    }

    string_buffer

}

fn substitute(text: &str, map: &HashMap<&str, &str>) -> String {

    let mut string_buffer = String::new();
    let mut word = String::new();

    for char in text.chars() {

        if char.is_alphanumeric() || char == '_' {
            word.push(char);
            continue;
        }

        string_buffer.push_str(map.get(word.as_str()).copied().unwrap_or(&word));
        string_buffer.push(char);
        word.clear();
    }

    string_buffer.push_str(map.get(word.as_str()).copied().unwrap_or(&word));
    string_buffer

}

fn char_literals(source: &str) -> String {

    let mut string_buffer = String::new();
    let mut chars = source.chars();

    while let Some(char) = chars.next() {

        if char != '\'' {
            string_buffer.push(char);
            continue;
        }

        let value = match chars.next().unwrap() {
            '\\' => match chars.next().unwrap() {
                'n' => 10,
                't' => 9,
                '0' => 0,
                other => other as u32,
            },
            other => other as u32,
        };

        chars.next();
        string_buffer.push_str(&value.to_string());
    }

    string_buffer
    
}