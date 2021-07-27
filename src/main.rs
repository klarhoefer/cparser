use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::fs::File;
use std::collections::{HashSet, HashMap};


fn is_whitespace(b: u8) -> bool {
    match b {
        b' ' | b'\t' | b'\r' | b'\n' => true,
        _ => false,
    }
}

struct Tokenizer<'a> {
    line: &'a str,
    text: &'a [u8],
    last: usize,
    pos: usize
}

impl<'a> Tokenizer<'a> {

    fn new(line: &'a str) -> Self {
        let text = line.as_bytes();
        Tokenizer { line, text, last: 0, pos: 0 }
    }

    fn available(&self) -> bool {
        self.pos < self.text.len()
    }

    fn step(&mut self) {
        self.pos += 1;
    }

    fn step_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn current(&self) -> Option<u8> {
        if self.available() {
            Some(self.text[self.pos])
        } else {
            None
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos + 1 < self.text.len() {
            Some(self.text[self.pos + 1])
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.current() {
            if is_whitespace(b) {
                self.step();
            } else {
                break;
            }
        }
    }

    fn slice(&self) -> &'a str {
        &self.line[self.last..self.pos]
    }

    fn identifier(&mut self) -> &'a str {
        while let Some(b) = self.current() {
            match b {
                b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => self.step(),
                _ => break,
            }
        }

        self.slice()
    }

    fn number(&mut self) -> &'a str {
        while let Some(b) = self.current() {
            match b {
                b'0'..=b'9' | b'.' => self.step(),
                _ => break,
            }
        }

        self.slice()
    }

    fn character(&mut self) -> &'a str {
        let mut mask = false;
        while let Some(b) = self.current() {
            self.step();

            match b {
                b'\\' => mask = true,
                _ if mask => mask = false,
                b'\'' => return self.slice(),
                _ => (),
            }
        }
        unreachable!()
    }

    fn string(&mut self) -> &'a str {
        let mut mask = false;
        while let Some(b) = self.current() {
            self.step();

            match b {
                b'\\' => mask = true,
                _ if mask => mask = false,
                b'\"' => return self.slice(),
                _ => (),
            }
        }
        unreachable!()
    }
}


impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        self.last = self.pos;

        if let Some(b) = self.current() {
            self.step();

            match b {
                b';' | b',' | b'{' | b'}' | b'[' | b']' | b'(' | b')' | b'?' | b':' => return Some(self.slice()),
                b'.' => {
                    match (self.current(), self.peek()) {
                        (Some(b'.'), Some(b'.')) => self.step_n(2),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'*' | b'^' | b'!' | b'=' | b'/' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'&' | b'|' | b'+' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        Some(b2) if b2 == b => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'<' | b'>' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        Some(b2) if b2 == b => {
                            self.step();
                            match self.current() {
                                Some(b'=') => self.step(),
                                _ => (),
                            }
                        },
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'-' => {
                    match self.current() {
                        Some(b'>') | Some(b'-') | Some(b'=') => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'_' | b'a'..=b'z' | b'A'..=b'Z' => return Some(self.identifier()),
                b'0'..=b'9' => return Some(self.number()),
                b'"' => return Some(self.string()),
                b'\'' => return Some(self.character()),
                _ => unreachable!(),
            }
        }
        None
    }
}

#[derive(Debug)]
struct Member {
    ident: String,
    type_: String,
    dims: Option<String>,
}

#[derive(Debug)]
struct Value {
    ident: String,
    value: Option<String>,
}

enum Stmt {
    Alias(String),
    Enum(Vec<Value>),
    Struct(Vec<Member>),
}


fn try_parse_struct<'a>(stmt: &'a [&'a str]) -> Option<(Option<&'a str>, &'a [&'a str])> {
    let l = stmt.len();
    match stmt {
        ["struct", "{", .., "}"] => Some((None, &stmt[2..l-1])),
        ["struct", tag, "{", .., "}"] => Some((Some(tag), &stmt[3..l-1])),
        _ => None,
    }
}

fn try_parse_member<'a>(stmt: &'a [&'a str]) -> Option<Member> {
    let l = stmt.len();
    match stmt {
        [.., "]"] => if let Some(pos) = stmt.iter().position(|&s| s == "[") {
            let ident = stmt[pos - 1].into();
            let type_ = (*&stmt[..pos - 1].join("~")).clone();
            let dims = (*&stmt[pos + 1..l - 1].join("~")).clone();
            Some(Member { ident, type_, dims: Some(dims) })
        } else {
            None
        },
        [] => None,
        _ => {
            let ident = stmt[l - 1].into();
            let type_ = (*&stmt[..l - 1].join("~")).clone();
            Some(Member { ident, type_, dims: None })
        }
    }
}

fn parse_members<'a>(stmt: &'a [&'a str]) -> Vec<Member> {
    stmt.split(|&m| m == ";").filter_map(try_parse_member).collect()
}

fn try_parse_enum<'a>(stmt: &'a [&'a str]) -> Option<(Option<&'a str>, &'a [&'a str])> {
    let l = stmt.len();
    match stmt {
        ["enum", "{", .., "}"] => Some((None, &stmt[2..l-1])),
        ["enum", tag, "{", .., "}"] => Some((Some(tag), &stmt[3..l-1])),
        _ => None,
    }
}

fn try_parse_value<'a>(stmt: &'a [&'a str]) -> Option<Value> {
    match stmt {
        [name, "=", ..] => {
            let ident = (*name).into();
            let value = (*&stmt[2..].join("~")).clone();
            Some(Value { ident, value: Some(value) })
        },
        [name] => {
            let ident = (*name).into();
            Some(Value { ident, value: None })
        },
        _ => None,
    }
}

fn parse_values<'a>(stmt: &'a [&'a str]) -> Vec<Value> {
    stmt.split(|&m| m == ",").filter_map(try_parse_value).collect()
}

fn try_parse_typedef<'a>(stmt: &'a [&'a str]) -> Option<(&'a str, &'a [&'a str])> {
    let l = stmt.len();
    match stmt {
        ["typedef", .., name] => Some((name, &stmt[1..l - 1])),
        _ => None,
    }
}

fn try_parse_function<'a>(stmt: &'a [&'a str]) -> Option<(&'a [&'a str], &'a str, &'a [&'a str])> {
    let l = stmt.len();
    match stmt {
        [.., ")"] => if let Some(pos) = stmt.iter().position(|&s| s == "(") {
            Some((&stmt[..pos - 1], stmt[pos-1], &stmt[pos+1..l - 1]))
        } else {
            None
        },
        _ => None,
    }
}

fn try_parse_function_type<'a>(stmt: &'a [&'a str]) -> Option<(&'a [&'a str], &'a str, &'a [&'a str])> {
    match stmt {
        [.., ")"] => {
            let braces = stmt.iter().enumerate().filter(|&(_, &c)| c == "(" || c == ")").collect::<Vec<_>>();
            match braces.as_slice() {
                [(a, _), (b, _), (c, _), (d, _)] => Some((&stmt[..*a], stmt[*b - 1], &stmt[*c + 1..*d])),
                _ => None,
            }
        },
        _ => None,
    }
}

fn try_parse_decl<'a>(stmt: &'a [&'a str]) -> Option<&'a [&'a str]> {
    match stmt {
        ["__pragma", ..] | ["__declspec", ..] => Some(stmt),
        _ => None,
    }
}

fn try_parse_extern<'a>(stmt: &'a [&'a str]) -> Option<&'a [&'a str]> {
    match stmt {
        ["extern", ..] => Some(&stmt[1..]),
        _ => None,
    }
}

fn print_stmt(stmt: &[&str]) {
    println!("{}", stmt.join(" ~ "));
}

fn parse_statement(stmt: &[&str], types: &mut HashMap<String, Stmt>) {
    let l = stmt.len();
    let stmt = if let [.., ";"] = stmt {
        &stmt[..l - 1]
    } else {
        stmt
    };

    if let Some((name, typedef)) = try_parse_typedef(stmt) {
        if let Some((_tag, members)) = try_parse_struct(typedef) {
            let members = parse_members(members);
            types.insert(name.into(), Stmt::Struct(members));
        } else if let Some((_tag, values)) = try_parse_enum(typedef) {
            let values = parse_values(values);
            types.insert(name.into(), Stmt::Enum(values));
        } else if let Some((_ret, _name, _params)) = try_parse_function_type(&stmt[1..]) {
            // print_stmt(stmt);
            // println!("{} {:?}: {:?}", name, params, ret);
        } else {
            // print_stmt(stmt);
            // println!("TYPE {} = {:?}", name, typedef);
            types.insert(name.into(), Stmt::Alias(typedef.join("~")));
        }
    } else if let Some((_tag, _members)) = try_parse_struct(stmt) {
    } else if let Some((_ret, _name, _params)) = try_parse_function(stmt) {
        // println!("{} {:?}: {:?}", name, params, ret);
    } else if let Some(_) = try_parse_decl(stmt) {
    } else if let Some(_) = try_parse_extern(stmt) {
    } else {
        print_stmt(stmt);
    }
}


fn parse<P: AsRef<Path>>(path: P) -> io::Result<HashMap<String, Stmt>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut source = String::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.len() == 0 || trimmed.starts_with("#line") || trimmed.starts_with("#pragma") {
            continue;
        }

        source.push('\r');
        source.push_str(&line);
    }
    
    let mut balance = 0;
    let mut statement = Vec::new();
    let mut types = HashMap::new();

    let tokenizer = Tokenizer::new(&source);
    for token in tokenizer {
        statement.push(token);
        match token {
            "{" | "[" | "(" => balance += 1,
            "}" | "]" | ")" => {
                balance -= 1;
                if balance == 0 {
                    match statement[0] {
                        "__pragma" | "__declspec" => {
                            parse_statement(&statement, &mut types);
                            statement.clear();
                        },
                        _ => (),
                    }
                }
            },
            ";" if balance == 0 => {
                parse_statement(&statement, &mut types);
                statement.clear();
            },
            _ => (),
        }
    }

    Ok(types)
}


fn lookup(name: &str, types: &HashMap<String, Stmt>, known: &mut HashSet<String>) -> bool {
    if known.contains(name) {
        return true;
    }

    known.insert(name.into());

    if let Some(t) = types.get(name) {
        match t {
            Stmt::Alias(alias) => {
                println!("pub type {} = {};\n", name, alias);
            },
            Stmt::Enum(values) => {
                println!("#[repr(C)]\npub enum {} {{", name);
                for value in values {
                    match value.value {
                        Some(ref v) => println!("\t{}={},", value.ident, v),
                        None => println!("\t{},", value.ident),
                    }
                }
                println!("}}\n");
            },
            Stmt::Struct(members) => {
                for member in members {
                    lookup(&member.type_, types, known);
                }

                println!("#[repr(C)]\npub struct {} {{", name);
                for member in members {
                    match member.dims {
                        Some(ref size) => println!("\tpub {}: [{}; {}],", member.ident, member.type_, size),
                        None => println!("\tpub {}: {},", member.ident, member.type_),
                    }
                }
                println!("}}\n");
            }
        }
        true
    } else {
        println!("Not found: {}", name);
        false
    }
}

static KNONW_ALIASES: &'static [(&'static str, &'static str)] = &[
    ("uint32_T", "u32"), ("int32_T", "i32"), ("boolean_T", "u8"), ("uint16_T", "u16"), ("real_T", "f64")];

fn main() {
    // file created with cl /P <header file>
    if let Ok(mut types) = parse(r"hdf5.i") {
        for &(name, alias) in KNONW_ALIASES {
            types.insert(name.into(), Stmt::Alias(alias.into()));
        }
        let mut known = HashSet::new();

        println!("#![allow(non_camel_case_types)]");
        println!("#![allow(dead_code)]");
        println!("#![allow(non_snake_case)]");

        println!();

        for k in types.keys() {
            lookup(k, &types, &mut known);
        }
    }
}


#[cfg(test)]
mod tests {

    use super::Tokenizer;

    #[test]
    fn it_works() {
        let tokenizer = Tokenizer::new("abc def = == ! != xyz ... < << <<= > >> >>=");
        for token in tokenizer {
            println!("{}", token);
        }
    }
}
